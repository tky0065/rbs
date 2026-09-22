---
sidebar_position: 2
title: Your first resource
---

# Your first resource

This is the second tutorial, and it picks up `demo` exactly where
[Setting up](./setup.md) left it: running, with nothing mounted but a health check. This
one adds the shape most APIs are built from — a table you create, list, and edit — with
one command that turns a `--fields` declaration into an entity, its migration, and every
layer between: `rbs generate crud`. The case is a blog's articles: a title, a body, and
whether the piece is published.

## What you need

Nothing beyond [Setting up](./setup.md): the same `demo`, still running with its
database, and `curl` again for the last section.

## 1. Generate the feature

```bash
rbs generate crud articles --fields "title:string,body:text,published:bool"
```

{/* rbs:transcript cmd="rbs generate crud articles --fields title:string,body:text,published:bool" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" */}
```text
$ rbs generate crud articles --fields title:string,body:text,published:bool
plan pour …/demo

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests/mod.rs                           créé
  + src/articles/tests/lifecycle.rs                     créé
  + src/articles/tests/errors.rs                        créé
  + src/articles/tests/filter.rs                        créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260909_092448_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  13 à créer, 7 à modifier
✓ articles générée — 13 créés, 7 modifiés

  la migration m20260909_092448_create_articles reste à appliquer avant de lancer le projet
```

Thirteen files written with no database running: proof that `--fields` alone was enough to
decide the entity's shape and its migration together, the schema declared once rather
than read back from a server. The `~` lines are not rewrites — they are insertions at
comment anchors already sitting in files you own, `// <rbs:features>` in `src/lib.rs`
among them. Delete one of those anchors and the next `generate` prints the block it would
have inserted instead of touching the file.

## 2. Apply the migration

```bash
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.74s
     Running `target/debug/migration up`
✓ migrations appliquées
```

The `migration` crate rebuilds because it just gained a file, and once it is done the
`articles` table exists — proof that the migration `generate` wrote a moment ago is not
just a file on disk, but a change the database has now applied.

## 3. Run the server

The server from [Setting up](./setup.md) is still running the old binary. Stop it and
start it again:

```bash
cargo run
```

{/* rbs:libre raison="journal d'un serveur qui tourne : le rejeu ne lance aucun serveur, et l'heure change à chaque démarrage" */}
```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

A clean start here is proof `src/articles/` compiles into the router: `demo` is now
listening with six new operations added to `/health` and `/health/live` — `GET` and `POST` on `/articles`,
`POST` on `/articles/filter`, and `GET`, `PATCH` and `DELETE` on `/articles/{id}` — if the
module had failed to compile, this line would never have printed.

## Verify

From the second terminal, still open from Setting up:

```bash
curl -i -X POST http://127.0.0.1:8080/articles \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier article","body":"Bonjour","published":true}'
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 201 Created
content-type: application/json
x-request-id: 01M22QWBW5CFQRAPDH41QB2D52
content-length: 191
date: Wed, 09 Sep 2026 09:27:14 GMT

{"id":"01a0857e-2f85-7d03-a129-6defbf74c73c","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-09-09T09:27:14.569998Z","updated_at":"2026-09-09T09:27:14.569998Z"}
```

`id`, `created_at` and `updated_at` are not in the request body — proof the row was
built and stored by the server, not echoed back from what was sent.

```bash
curl http://127.0.0.1:8080/articles
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
{"data":[{"id":"01a0857e-2f85-7d03-a129-6defbf74c73c","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-09-09T09:27:14.569998Z","updated_at":"2026-09-09T09:27:14.569998Z"}],"meta":{"page":1,"per_page":20,"total":1,"total_pages":1}}
```

The same `id` comes back under `data`, with `meta` describing the page it sits on —
proof the write from the previous request and this read agree on the same row, through
the same running server.

## What was installed

Four of the ten files `generate crud` wrote to `src/articles/` — every file there
besides `mod.rs` — each read from
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
the same feature, generated by the same command.

### The entity

`Model` is the SeaORM entity `--fields` produced: one struct, one row shape, `published`
mapped straight to a `bool` column.

```rust file=examples/hello-crud/src/articles/model.rs region=entite
```

### The input

`CreateArticle` is what `POST /articles` deserializes into. `title` carries a
`max = 255` constraint that no `--fields` syntax spelled out — the generator's own
default for a bare `string`.

```rust file=examples/hello-crud/src/articles/dto.rs region=entree
```

### The handler

`create` is the whole controller for this route: it hands the validated input to the
service and turns the result into a status code. It never opens a `DatabaseConnection`
itself.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

### The query

`list` and `filter` are the repository, the only layer allowed to build a SeaORM query.
`list` is `filter` with no condition at all — the same path the `GET` above took.

```rust file=examples/hello-crud/src/articles/repository.rs region=list
```

One direction of dependency runs through all four: controller → service → repository →
model, each layer seeing only the next. [Architecture](../architecture.md) maps every
layer these seven files land in, including the three this page never opened —
`service.rs`, `filter.rs` and `tests/`.

## Going further

- [`rbs generate`](../cli/generate.md) covers the flags this page didn't use —
  `--has-many`, `--soft-delete`, and `--role` for a write only some callers may make.
- [Filtering](../guides/filtering.md) is what `filter.rs` mounts: a `POST
  /articles/filter` route this page never called.
- [Testing](../guides/testing.md) reads the `tests/` directory the same command wrote, and
  the harness it runs against.
- [Architecture](../architecture.md) maps every file `rbs generate crud` just wrote to
  the layer it belongs to.
- [Locking the API down](./auth.md) is the next tutorial: closing `demo` to everyone it
  doesn't know, with a write only an administrator may make on a second resource.
