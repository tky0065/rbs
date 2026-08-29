---
sidebar_position: 2
title: Getting started
---

# Getting started

This page goes from an empty directory to a CRUD API answering on `localhost:8080`,
in eight commands. Every block of output below was copied from a real run — if what
your terminal prints matches, you have not drifted — timings, identifiers and dates
excepted, which are yours. Only one thing was edited out of the blocks: the absolute
path of the directory the run happened in, written `…/demo` below.

The CLI speaks French: `✓ demo créé — 16 fichiers` is a success line, not an error.
Only the messages are translated-in-waiting; the flags, the file names and the generated
code are the same in every locale.

## What you need

- **Rust stable**, edition 2024. This run used `rustc 1.96.0`.
- **PostgreSQL 14 or later.** The Docker one-liner below is enough; an existing server
  works just as well, as long as you can point a URL at it. That floor is the oldest
  release still receiving security fixes: generated models set their own v7 identifier, so
  nothing a project runs asks the server for a `uuidv7()` of its own.
- **curl**, or any HTTP client, for the last section.

## Installing the CLI

The package is `rbs-cli`, the command it installs is `rbs`, and the name `rbs` on
crates.io belongs to an unrelated project:

```bash
cargo install rbs-cli
```

That drops an `rbs` executable in `~/.cargo/bin`, and a second copy of it named
`rbs-cli`, for the case the note below describes. Check the binary answers:

```bash
rbs --version
```

```text
rbs 1.0.0
```

:::note

The Ruby ecosystem ships an unrelated tool also called `rbs`. If `rbs --version` prints
something like `rbs 3.10.0`, that one is winning on your `PATH`. There is nothing to
reorganise: the install put the same program there a second time, under a name nobody
else claims.

```bash
rbs-cli --version
```

Use `rbs-cli` wherever this page writes `rbs`. It is the same binary, and it names itself
after the way you called it, so its `--help` stays true to what you type.

:::

## Starting a database

rbs does not manage your database; it expects a URL that answers. The shortest way to
get one:

```bash
docker run --rm -d --name rbs-demo \
  -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=demo \
  -p 5432:5432 postgres:18
```

Leave it running for the rest of this page. `docker stop rbs-demo` removes it when you
are done — the container was started with `--rm`, so nothing is left behind.

## Creating the project

```bash
rbs new demo --yes --database-url postgres://rbs:rbs@localhost:5432/demo
```

```text
✓ demo créé — 16 fichiers

  cd demo
  cargo run          # la base visée est dans .env
```

The manifest the command writes depends on `rbs-core` from crates.io, so there is
nothing to point the project at and nothing to build first — `cargo build` resolves it
like any other dependency. The one case that needs more is contributing to the core
itself, and [`rbs new --core-path`](./cli/new.md) covers it.

`--yes` answers every question with its default — here, the `health` feature and
nothing else. Drop it and the CLI asks, in order, for the database URL if
`--database-url` is missing and for the optional features to install. It also refuses to
run without a terminal to ask in, so `--yes` is what a script or a CI job needs:

```text
erreur : aucun terminal interactif pour poser les questions : relancez avec `--yes` pour prendre les défauts, ou donnez les réponses en flags — le nom en argument, `--database-url` et `--with`
```

Sixteen files, and none of them a black box:

- `src/main.rs`, `src/router.rs`, `src/state.rs`, `src/openapi.rs` — the wiring.
- `src/health/` — a first feature, so the shape is visible before you generate one.
- `src/seeds/` — a second binary, `seed`, which `rbs seed` runs.
- `migration/` — a second crate, holding the migrations.
- `config/default.toml` and `config/development.toml` — host, port, pool sizes.
- `.env` — the database URL and the log settings, kept out of Git.
- `.env.example` — the same keys with no secrets, committed.

The `.env` the command wrote carries the URL you passed:

```text
RBS_ENV=development
RBS_DATABASE__URL=postgres://rbs:rbs@localhost:5432/demo

RBS_LOG_FORMAT=pretty
RUST_LOG=info,demo=debug
```

## The first migration

```bash
cd demo
rbs migrate up
```

The first run compiles the `migration` crate, which takes a minute; the last lines are
the ones that matter:

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.48s
     Running `target/debug/migration up`
✓ migrations appliquées
```

There is nothing to migrate yet — the command creates the table SeaORM uses to track
applied migrations. Running it now is how you find out the URL in `.env` is right,
before any generated code depends on it.

## Generating a CRUD feature

```bash
rbs generate crud articles --fields "title:string,body:text,published:bool"
```

The command prints what it intends to do, then does it:

```text
plan pour …/demo

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260829_100554_create_articles.rs   créé
  ~ src/main.rs                                         modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié

  15 fichiers à écrire
✓ articles générée — 9 fichiers

  la migration m20260829_100554_create_articles reste à appliquer avant de lancer le projet
```

Your migration file will carry a different timestamp: the name is built from the moment
you ran the command. Everything else matches.

Two things to notice. The entity and its migration both came from `--fields`, with no
database running and no introspection — the schema is declared once, in the command.
And the `~` lines are edits to files you own: the CLI inserted into comment anchors
(`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:seeds>`, `<rbs:migrations>`)
rather than rewriting your code. Delete an anchor and the CLI stops writing there,
printing the block for you to paste instead.

Apply the new migration:

```bash
rbs migrate up
rbs migrate status
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.02s
     Running `target/debug/migration up`
✓ migrations appliquées
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running `target/debug/migration status`
  ✓ m20260829_100554_create_articles   appliquée
```

## What the generator wrote

Six files per feature, plus its tests and its seed, with one direction of dependency:
controller → service → repository → model. Here is the `POST /articles` handler, read from
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
the same feature, generated by the same command:

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

The controller hands the request to the service and maps the result to a status code;
that is all it does. The service never sees a `DatabaseConnection`, and the controller
never builds a SeaORM query. Nothing here is marked "do not edit" — validation rules,
extra endpoints and business logic all go in these files.

## Running it

```bash
cargo run
```

The first build is long — it is the whole Axum, SeaORM and utoipa tree. When it is
done:

```text
10:06:25  INFO   demo                démarrage  adresse=127.0.0.1:8080
```

That is the `pretty` log formatter: timestamp, level, target, message, fields. Set
`RBS_LOG_FORMAT=json` in `.env` when a collector reads the output instead of a human.

## First requests

Leave the server running and open a second terminal.

```bash
curl -i http://127.0.0.1:8080/health
```

```text
HTTP/1.1 200 OK
content-type: application/json
x-request-id: 01M16FRBHD0ZHWCN4EAEAW3TGK
content-length: 42
date: Sat, 29 Aug 2026 10:06:30 GMT

{"status":"ok","checks":{"database":"ok"}}
```

`/health` came with the project and checks the database, not just the process. The
`x-request-id` header is on every response, and the same value appears in the log line
for that request.

```bash
curl -i -X POST http://127.0.0.1:8080/articles \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier article","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 201 Created
content-type: application/json
x-request-id: 01M16FRBHQDCDV859ADGK8ZMJG
content-length: 191
date: Sat, 29 Aug 2026 10:06:30 GMT

{"id":"01a04cfc-2e37-78a1-bcb1-6599b0c362e2","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-29T10:06:30.457086Z","updated_at":"2026-08-29T10:06:30.457086Z"}
```

The identifier and the timestamps are the server's — `id`, `created_at` and
`updated_at` are not part of the request body.

```bash
curl http://127.0.0.1:8080/articles
```

```text
{"data":[{"id":"01a04cfc-2e37-78a1-bcb1-6599b0c362e2","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-29T10:06:30.457086Z","updated_at":"2026-08-29T10:06:30.457086Z"}],"meta":{"page":1,"per_page":20,"total":1,"total_pages":1}}
```

Collections are paginated by default, under `data` and `meta`. `?page=` and `?per_page=`
move through them. The three remaining routes — `GET`, `PUT` and `DELETE` on
`/articles/{id}` — were generated at the same time.

Meanwhile, the server's terminal has been printing one line per request:

```text
10:06:30  INFO   rbs_core::trace     request  status=200 latency_ms=0.80275 request_id=01M16FRBHD0ZHWCN4EAEAW3TGK method=GET path=/health
10:06:30  INFO   rbs_core::trace     request  status=201 latency_ms=4.517125 request_id=01M16FRBHQDCDV859ADGK8ZMJG method=POST path=/articles
10:06:30  INFO   rbs_core::trace     request  status=200 latency_ms=35.174416 request_id=01M16FRBJ3RVKC5T37ACD8SM6F method=GET path=/articles
```

## The OpenAPI document

The document is built from the annotations on the handlers, so it describes the routes
that exist rather than the ones someone remembered to write down. Open
`http://127.0.0.1:8080/docs` for the Swagger UI, or read the document itself:

```bash
curl http://127.0.0.1:8080/api-docs/openapi.json
```

Its `paths` now holds `/health`, `/articles` and `/articles/{id}`. Both routes are
switched by `[docs]` in `config/default.toml`; turn them off in production.

## Checking a project

When something looks wrong, ask before guessing:

```bash
rbs doctor
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
     Running `target/debug/migration version`
  ✓ ancres     les 9 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core 1.0.0 alignés sur le CLI 1.0.0
  ✓ base       postgres 18.6 répond sur localhost:5432
✓ le projet est sain
```

Four checks: the anchors are still in place, `.env` holds every key `.env.example`
declares, the project and `rbs-core` agree with the CLI's version, and the database
answers.

## Where to go next

- [Logs](./guides/logs.md) — the two formatters, and what to put in `RUST_LOG`.
- The generated code is yours: open `src/articles/service.rs` and add a rule.
- `rbs generate crud --dry-run` prints the plan and writes nothing, which is the
  cheapest way to see what a set of `--fields` produces.
- The [roadmap](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) lists what rbs
  covers and what is deliberately out of scope.
