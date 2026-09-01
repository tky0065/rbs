---
sidebar_position: 4
title: rbs migrate
---

# `rbs migrate`

Drives the project's migrations. `up`, `down` and `status` wrap the binary of the project's
own `migration` crate: SeaORM's engine is not reimplemented, only made readable. `new`
needs nobody — neither cargo nor a running database.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs migrate --help
Pilote les migrations du projet

Usage: rbs migrate <COMMAND>

Commands:
  up      Applique les migrations en attente
  down    Annule la dernière migration appliquée
  status  Affiche les migrations appliquées et celles en attente
  new     Crée un fichier de migration vide
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

No subcommand takes a flag of its own, and neither `--template-dir` nor `--yes` is
accepted: each is declared on the commands that read it.

```text
$ rbs migrate up --help
Applique les migrations en attente

Usage: rbs migrate up

Options:
  -h, --help     Print help
  -V, --version  Print version
```

`down` and `status` are declared the same way. `new` alone takes an argument:

```text
$ rbs migrate new --help
Crée un fichier de migration vide

Usage: rbs migrate new <NAME>

Arguments:
  <NAME>  Nom de la migration

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Which database

`up`, `down` and `status` read the project's `.env` and take the target from
`RBS_DATABASE__URL` — the variable the core's configuration already uses to fill
`database.url`, not a `DATABASE_URL` known to rbs alone. They then shell out to `cargo`,
which means the `migration` crate is compiled on the first run.

## `status`

Applied migrations carry `✓`, pending ones `·`. On a project whose migration has never
run:

```text
$ rbs migrate status
  · m20260826_213608_create_articles   en attente
```

## `up`

```text
$ rbs migrate up
✓ migrations appliquées
```

And the same project, once up to date:

```text
$ rbs migrate status
  ✓ m20260826_213608_create_articles   appliquée
```

## `new`

Creates an empty migration file, timestamped, and registers it in the `Migrator`. It
touches neither cargo nor the database, so it works with nothing running:

```text
$ rbs migrate new add_tags_index
✓ migration/src/m20260826_213622_add_tags_index.rs créée

  décrivez le changement de schéma, puis `rbs migrate up`
```

Registration goes through two anchors of `migration/src/lib.rs`, kept apart because Rust
forbids a non-inline `mod` inside a block, so the declaration cannot live in the
`Migrator`'s `vec!`:

```text
$ cat migration/src/lib.rs
pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260826_213608_create_articles;
mod m20260826_213622_add_tags_index;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260826_213608_create_articles::Migration),
            Box::new(m20260826_213622_add_tags_index::Migration),
            // </rbs:migrations>
        ]
    }
}
```

`status` now has one of each:

```text
$ rbs migrate status
  ✓ m20260826_213608_create_articles   appliquée
  · m20260826_213622_add_tags_index    en attente
```

The new file's body is a `todo!()` carrying the instruction, so running `up` before
describing the schema change says exactly that instead of applying an empty migration:

```text
$ rbs migrate up

thread 'main' (7889417) panicked at migration/src/m20260826_213622_add_tags_index.rs:11:9:
not yet implemented: décrivez le changement de schéma, puis relancez `rbs migrate up`
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
erreur : la crate migration a échoué (code 101)
```

## `down`

Rolls back the last applied migration — one, not all:

```text
$ rbs migrate down
✓ dernière migration annulée

$ rbs migrate status
  · m20260826_213608_create_articles   en attente
  · m20260826_213622_add_tags_index    en attente
```

## Failures

Outside a project — the search walks up from the current directory looking for a
`Cargo.toml` that carries `[package.metadata.rbs]`, which is also what keeps a command run
from `migration/src` from targeting the wrong root:

```text
$ rbs migrate status
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

With a `.env` that does not say which database to target:

```text
$ rbs migrate status
erreur : RBS_DATABASE__URL est absente du .env : rbs ne sait pas quelle base migrer
```

With nothing listening at the other end, the message comes from the migration binary,
whose exit code rbs reports:

```text
$ rbs migrate status
Connection Error: pool timed out while waiting for an open connection
erreur : la crate migration a échoué (code 1)
```

`rbs migrate new` is immune to the last two: it never reads the `.env` and never opens a
connection. It still needs a project.

Each of these exits with status 1.
