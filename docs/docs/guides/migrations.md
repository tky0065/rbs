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

Three columns are added to the ones you named. `id` is a UUID defaulted by `uuidv7()`,
which makes identifiers sort by creation time — **PostgreSQL 18 is the floor**, since
that is where `uuidv7()` became native. `created_at` and `updated_at` both default to the
transaction timestamp.

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
