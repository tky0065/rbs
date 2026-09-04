# rbs-cli

The `rbs` command: it generates and maintains web API projects in Rust, built on Axum and
SeaORM. Part of [rbs](https://github.com/tky0065/rbs).

*[Version française](README.fr.md).*

## Install

```bash
cargo install rbs-cli
```

The package is `rbs-cli`; the binary it installs is `rbs`.

> **`cargo install rbs` gets you something else.** The name `rbs` on crates.io belongs to an
> unrelated serialization crate. Install `rbs-cli`.

The same install also puts a second binary, `rbs-cli`, next to `rbs`. The Ruby ecosystem ships
an unrelated tool also called `rbs`, and package managers often place it ahead of
`~/.cargo/bin`. If `rbs --version` prints something like `rbs 3.10.0`, that one is winning on
your `PATH` — use `rbs-cli`, which nobody else claims.

Requires Rust 1.94 or later. A generated project runs on PostgreSQL 14+, MySQL 8.0+ or
SQLite 3.35+.

## Commands

| Command | What it does |
|---|---|
| `rbs new <name>` | Creates a project ready to start: database, migrations, `/health` route |
| `rbs add <feature>` | Installs a feature: `audit`, `auth`, `ci`, `cors`, `docker`, `jobs`, `mail`, `observability`, `rate-limit`, `redis`, `scheduler`, `storage`, `webhooks` |
| `rbs generate crud <name>` | Generates a full CRUD feature — entity and migration included |
| `rbs generate feature <name>` | Generates an empty feature: six files, no fields |
| `rbs migrate up\|down\|status\|new` | Drives the project's migrations |
| `rbs seed` | Inserts the project's demonstration data |
| `rbs dev` | Starts services and migrations, and restarts the server on every change |
| `rbs doctor` | Diagnoses the project: anchors, `.env`, database reachable, versions |
| `rbs upgrade` | Aligns the project's manifest on the CLI version |
| `rbs completions <shell>` | Writes the shell's completion script to standard output |

`generate` answers to `g`. `rbs new` takes `--yes`, which accepts the defaults without
asking so the CLI stays scriptable; `rbs new` and `rbs add` take `--template-dir`, which
swaps the templates embedded in the binary for your own. No other command accepts either.

## What it writes

From an empty directory to a CRUD API, with its entity, its migration, its OpenAPI document
and its tests:

```bash
rbs new blog-api
cd blog-api
rbs generate crud articles --fields 'title:string,body:text,published:bool'
rbs migrate up
```

This is the shape of the thing, not a transcript to paste — the
[getting started guide](https://tky0065.github.io/rbs/getting-started) has the runnable
version, with the database the commands expect and the output of each one.

`rbs generate crud` produces the SeaORM entity *and* its migration from `--fields`, with no
database running — the reverse of `sea-orm-cli generate entity`.

## The boundary it draws

[`rbs-core`](https://crates.io/crates/rbs-core) — which `rbs new` writes into the generated
manifest — carries what has no reason to vary from one project to the next: errors, logging,
configuration, application state. The CLI generates into your own sources everything you will
want to read or change: model, DTO, repository, service, controller, migration, tests, as
plain Rust with no macro to unfold. That generated code is yours from the moment it is
written, and no rbs release rewrites it.

Which is also why the CLI never rewrites an AST. It inserts into comment anchors
(`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`); if an anchor is
missing it writes nothing and prints the block for you to paste. Every command that touches an
existing project shows its plan before applying it, stays idempotent, and restores what it
touched if a step fails.

## Documentation

The site is at <https://tky0065.github.io/rbs/> — getting started, architecture, a reference
page per command. A French version lives at <https://tky0065.github.io/rbs/fr/>.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
