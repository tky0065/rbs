---
sidebar_position: 5
title: Migrations
---

# Migrations

Schema changes live in the `migration` crate of your project, a plain SeaORM migrator.
What rbs adds is the direction things flow in: you describe the fields on the command
line, and both the entity and its migration come out. No database has to be running for
that to work.

## The command line writes the schema, not the other way around

`sea-orm-cli generate entity` reads an existing database and produces Rust from it. That
assumes the schema already exists — written by hand, or by a migration you also wrote by
hand.

`rbs generate crud` goes the other way:

```bash
rbs generate crud articles --fields 'title:string,body:text,published:bool'
```

One command, and the feature's six files, the SeaORM entity **and** the migration that
creates its table are all written from the same description. Nothing is connected to.

A field is `name:type`, optionally followed by modifiers:

| Type | Column | Rust |
|---|---|---|
| `string` | `string()` | `String` |
| `text` | `text()` | `String` |
| `int` | `integer()` | `i32` |
| `float` | `double()` | `f64` |
| `bool` | `boolean()` | `bool` |
| `uuid` | `uuid()` | `Uuid` |
| `datetime` | `timestamp_with_time_zone()` | `DateTimeWithTimeZone` |

| Modifier | Effect |
|---|---|
| `unique` | unique index on the column |
| `optional` | nullable column, `Option<T>` in the entity |
| `index` | plain index — refused together with `unique`, which already indexes |

## What comes out

The migration for `articles` above, exactly as generated:

```rust file=examples/hello-crud/migration/src/m20260826_205243_create_articles.rs region=up
```

Three columns are added to the ones you named. `id` is a UUID with no column default:
**the generated model sets it**, with `Uuid::now_v7()`, just before the insert. Identifiers
still sort by creation time, and no engine is ever asked for a `uuidv7()` of its own — which
is what lets the same migration run on PostgreSQL, MySQL and SQLite alike. `created_at` and
`updated_at` both default to the transaction timestamp.

The column names are declared in the `DeriveIden` enum at the bottom of the file, which is
what the SeaORM query builder refers to:

```rust file=examples/hello-crud/migration/src/m20260826_205243_create_articles.rs region=colonnes
```

The file name carries the date and time of its creation — `m20260826_205243_create_articles.rs`
— and that is what orders the migrations. It is also where `DeriveMigrationName` takes
the name recorded in the database, so renaming an applied migration file makes the
migrator believe it has never run.

The migrator itself is registered through two anchors, one for the module and one for the
list:

```rust file=examples/hello-crud/migration/src/lib.rs
```

As everywhere in rbs, an absent anchor means the CLI writes nothing and prints the block
for you to paste.

## Soft delete

```bash
rbs generate crud articles --fields "title:string,slug:string:unique" --soft-delete
```

The HTTP contract does not move: `DELETE` still answers 204, a second one still 404, `GET`
on a deleted row still 404, and the row is gone from `list` and `filter` alike. What
`--soft-delete` changes is what `DELETE` does underneath — the row stays, its `deleted_at`
column dated, and every read filters it out.

The column is nullable, carries no default, and the flag injects it: it is not a field you
declare. Naming it yourself under `--fields` is refused, by the flag that owns it:

```text
$ rbs generate crud comments --fields "body:text,deleted_at:datetime" --soft-delete --dry-run
erreur : `--soft-delete` pose lui-même la colonne `deleted_at` : retirez-la de `--fields`, ou renoncez au drapeau
```

Outside `--soft-delete`, `deleted_at` is an ordinary name — nothing reserves it project-wide.

Every read filters on the column, so the migration lays down the column and an index on it:

```rust
.col(
    ColumnDef::new(Articles::DeletedAt)
        .timestamp_with_time_zone()
        .null(),
)
```

```rust
manager
    .create_index(
        Index::create()
            .if_not_exists()
            .name("idx_articles_deleted_at")
            .table(Articles::Table)
            .col(Articles::DeletedAt)
            .to_owned(),
    )
    .await?;
```

A `unique` field moves its constraint off the column and onto an index restricted to live
rows — `WHERE deleted_at IS NULL` — rather than the whole table. That is what lets a value a
deleted row held come back into use: someone can re-register with the address they had
before deleting their account. Proven by running the generated migration on both
PostgreSQL and SQLite.

```rust
// PostgreSQL et SQLite savent restreindre un index à un sous-ensemble de lignes :
// deux lignes portent alors la même valeur si l'une est supprimée. MySQL ne le
// sait pas — l'unicité y reste globale, et une valeur supprimée y reste réservée.
let mut uq_articles_slug = Index::create()
    .if_not_exists()
    .unique()
    .name("uq_articles_slug")
    .table(Articles::Table)
    .col(Articles::Slug)
    .to_owned();

if !matches!(manager.get_database_backend(), sea_orm::DbBackend::MySql) {
    uq_articles_slug = uq_articles_slug
        .and_where(Expr::col(Articles::DeletedAt).is_null())
        .to_owned();
}
```

The comment states the limit outright: MySQL has no partial index, so the migration keeps a
global uniqueness there instead of restricting it — on MySQL, a value a deleted row held
stays reserved.

### What it changes for the neighbouring features

The HTTP contract holds for the feature carrying the flag. It does not hold for the ones
that reference it. A generated foreign key carries an `ON DELETE Restrict | Cascade |
SetNull`, and a logical delete triggers none of them: the row is never removed, so the
engine has nothing to react to. Seen from a client:

- a parent deleted logically leaves its children pointing at a row that now answers 404,
  and those children stay listable through their own API — where `Cascade` would have
  removed them and `SetNull` untied them;
- `Restrict`, the default, used to make `DELETE /parents/{id}` fail as long as a child
  existed. It now answers 204;
- `POST /children` carrying the id of a deleted parent **succeeds**, the foreign key still
  being satisfied, where it used to be refused.

Put the flag on a feature others reference, and what a deleted parent means for them is a
decision the flag leaves to you — in the service of the child feature, or in a `deleted_at`
of its own.

The flag stops there. It writes no restoration route, no `?include_deleted` query
parameter, and no purge job. Restoring a row is a SQL `UPDATE`, for as long as no real need
has said what shape that route should take — a restoration route raises a question the flag
does not settle on its own: who is allowed to restore.

## Running them

```bash
rbs migrate up       # apply everything pending
rbs migrate down     # undo the last applied migration
rbs migrate status   # what is applied, what is waiting
rbs migrate new add_slug_to_articles
```

`up`, `down` and `status` wrap `cargo run -p migration -- <command>` inside your project:
the SeaORM engine is not reimplemented, only made readable. They need to know which
database to talk to, and they read it from the project's `.env` under
`RBS_DATABASE__URL` — the same variable the runtime configuration uses, not a
`DATABASE_URL` that only rbs would know about. The caller's environment wins, so

```bash
RBS_DATABASE__URL=postgres://… rbs migrate up
```

targets another database without editing the file.

`new` is the exception: it creates an empty migration and mounts it in the crate, without
cargo and without a running database. Use it for anything `generate crud` does not
produce — an added column, an index, a backfill.

## Judge for yourself

From a generated project, with a database reachable:

```bash
rbs migrate status
```

It lists every migration the crate knows about, applied or pending, before you change
anything.
