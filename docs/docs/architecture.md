---
sidebar_position: 3
title: Architecture
---

# Architecture

rbs rests on three decisions. Where the boundary between the framework and your code
falls; what a feature is made of; which way its parts are allowed to point. Each one
motivates the next, so this page takes them in that order — and shows them on
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), a
project CI compiles, rather than on code written for the occasion.

## The core / generated boundary

Every framework has to answer the question of what it keeps and what it hands over. rbs
answers it with a single test, applied to each piece of code it is responsible for:

> **Will a developer want to read this?**

If the answer is no, it belongs in `rbs-core` — a dependency in your `Cargo.toml`,
upgraded like any other. Opening a connection pool, parsing a page number, colouring a log
line, turning an error into an RFC 9457 body: nobody opens those files, and nobody should
have to.

If the answer is yes, the `rbs` command-line tool writes it into your `src/`, and it is
yours. The shape of an `Article`, the rule that decides when one is publishable, the route
that lists them — that is the code a developer opens the day something changes.

Note what the test is *not*. It does not sort by generality: a CRUD service is generic
enough to live in the core behind a trait, and rbs generates it anyway, because a service
is the first place anyone looks when a business rule moves. That is why no generated file
carries a "generated, do not edit" banner. It is written to be edited, and the boundary
exists so that editing it is enough.

### What the core carries

Eleven public modules, all of them on the "nobody wants to read this" side of the test:

| Module | What it does | Why it never varies |
|---|---|---|
| `config` | Loads and validates the application's configuration | Layering files, environment and defaults is the same problem in every project |
| `db` | Opens the connection pool at startup | An unreachable database must stop the process, not surface on the first request |
| `error` | The runtime's error type and its `Result` alias | One error type, one HTTP mapping — the value of it lies in being shared |
| `extract` | Request extractors, `ValidatedJson` among them | Deserialising then validating a body is plumbing, and identical everywhere |
| `health` | The health handler | The core owns the check; the generated project decides where to mount it |
| `logs` | The `pretty` and `json` formatters | A log format is a house style, not a project decision |
| `openapi` | `ProblemDetails` and the shared error responses | Declared once, so every operation's error responses cannot drift apart |
| `pagination` | The `Pagination` extractor and the `Page` envelope | Bounds are core constants, which keeps the extractor stateless |
| `request_id` | The correlation id of the current request | Read by logs and error responses that never received it as an argument |
| `state` | `CoreState` — pool plus configuration — and `HasCoreState` | The project owns its `AppState`; the core owns only what it needs to reach through it |
| `trace` | The per-request span and its outcome log | Every HTTP API wants the same span, with the same fields |

The crate re-exports the handful of items a feature names constantly — `Config`, `Error`,
`Result`, `ValidatedJson`, `ProblemDetails`, `Page`, `Pagination`, `CoreState`,
`HasCoreState` — so generated code imports them straight from `rbs_core`.

`AppState` is the boundary in miniature. The core carries `CoreState`, the pool and the
configuration; your project declares its own `AppState` around it, free to gain a Redis
client or a mail service without asking the framework's permission. Core handlers reach
the pool through the `HasCoreState` trait, whatever wraps it.

### One filled feature flag, three still empty

`rbs-core` declares four Cargo features beyond the database drivers. One carries code; the
other three reserve a name and nothing more:

| Flag | State | What it carries |
|---|---|---|
| `auth` | **filled** | Argon2 hashing, JWTs, opaque tokens, an identity extractor |
| `redis` | empty | A Redis client shared through the application state |
| `mail` | empty | Sending mail and rendering templates |
| `storage` | empty | File storage, local or S3-compatible |

Enabling one of the three empty ones compiles nothing extra and pulls in no dependency —
it is not an error, simply a no-op. Naming them early cost nothing and settled the question
of what they would be called; `auth` is what that reservation was for, and it was filled
without renaming anything.

The empty three are also outside the compatibility promise: they carry no public API to
freeze. Filling them is an addition, never a break.

## Anatomy of a feature

Everything on the generated side of the boundary is organised by feature, never by layer:
one directory per resource, six files inside it. `rbs generate crud articles` writes them
all.

```text
src/articles/
├── mod.rs          declares the siblings, exposes the routes
├── model.rs        the SeaORM entity — the table, as Rust
├── dto.rs          what crosses the HTTP boundary, in and out
├── repository.rs   the only place a query is built
├── service.rs      the business rules
└── controller.rs   HTTP: extraction, status codes, OpenAPI
```

There is no `src/models/`, no `src/services/`. A feature is read, moved and deleted in one
piece, and a directory listing tells you what the API does.

### `mod.rs` — the wiring

It declares the five other files and publishes the feature's routes as a `Router` the
project's router merges. Nothing else lives here.

```rust file=examples/hello-crud/src/articles/mod.rs region=routes
```

### `model.rs` — the entity

The table as a Rust type, and the only place that describes it. The primary key is a UUID
the application generates, not a sequence the database hands out.

```rust file=examples/hello-crud/src/articles/model.rs region=entite
```

### `dto.rs` — the wire types

Separate from the model on purpose: a column is not a field of your API. `CreateArticle`
says what a client may send — `id`, `created_at` and `updated_at` are absent because they
are not the client's to set —

```rust file=examples/hello-crud/src/articles/dto.rs region=entree
```

— and `ArticleResponse` says what it gets back, with the `utoipa` annotations that put it
in the OpenAPI document.

```rust file=examples/hello-crud/src/articles/dto.rs region=reponse
```

The day the two diverge from the table, they diverge alone: the model does not follow.

### `repository.rs` — the queries

The only file that builds a SeaORM query. It takes the connection as an argument and
returns models, never DTOs.

```rust file=examples/hello-crud/src/articles/repository.rs region=list
```

### `service.rs` — the rules

Composes repository calls and turns the result into DTOs. "Not found" is a business
verdict, not a database one: it is decided here, and the repository is left free to return
`Option`.

```rust file=examples/hello-crud/src/articles/service.rs region=find
```

### `controller.rs` — the HTTP surface

Extraction, status code, OpenAPI annotation, and nothing else. `ValidatedJson` has already
rejected a malformed or invalid body before this function runs.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

## The dependency rule

The six files are ordered, and the arrows all point the same way:

```text
controller ──> service ──> repository ──> model
     │            │                          ▲
     └────────────┴───────> dto ─────────────┘
```

Nothing points left. A repository does not call a service; a model knows nothing of HTTP.
This is what makes each file readable on its own: to understand `service.rs` you need to
know the repository's signatures, not Axum's extractors nor SeaORM's query builder.

Two things follow, and they are visible in the imports rather than in a rule anyone has to
remember. Here is what the repository names:

```rust file=examples/hello-crud/src/articles/repository.rs region=imports
```

and here is what the service names:

```rust file=examples/hello-crud/src/articles/service.rs region=imports
```

**A controller never builds a query.** It has a `State`, so it could. Ask the feature
which files know SeaORM at all:

```bash
grep -l sea_orm examples/hello-crud/src/articles/*.rs
```

```text
examples/hello-crud/src/articles/controller.rs
examples/hello-crud/src/articles/dto.rs
examples/hello-crud/src/articles/filter.rs
examples/hello-crud/src/articles/model.rs
examples/hello-crud/src/articles/repository.rs
examples/hello-crud/src/articles/service.rs
```

Six files out of seven — so "only the repository imports SeaORM" would be false, and it is
worth saying why. Four of those other files name `sea_orm` for its scalar types, `Uuid` and
`DateTimeWithTimeZone`, which cross every layer; the service adds `ActiveValue::Set` to
build the active model it hands over. `filter.rs` is the exception, and a deliberate one:
it names the query traits, because it translates a body into conditions. It belongs to the
repository layer, which `repository.rs` being its only caller enforces. The narrower probe
is the honest one:

```bash
grep -l 'Entity::' examples/hello-crud/src/articles/*.rs
```

```text
examples/hello-crud/src/articles/repository.rs
```

One file. `Entity::find` is called in `repository.rs` and nowhere else — `filter.rs`
receives the `Select` already opened and returns it narrowed, without ever reaching for the
entity itself. The whole query vocabulary stops at those two files, and the three layers
above them never see it.

**A service never holds a connection.** `DatabaseConnection` appears in the service's
imports, and the `find` snippet above shows why: the service receives a
`&DatabaseConnection` and passes it straight to the repository. It never stores one in a
struct, never calls a method on it, never learns what kind of database is behind it. The
borrow travels through; the knowledge does not.

That is a deliberate cost. Passing `db` down through every signature is more typing than
holding a handle would be, and it buys a service you can read without knowing what a pool
is.

### When a file gets long

**A feature file past about 200 lines is telling you the feature should be split.** The
threshold is not a lint, it is a reading habit: past that length a file stops fitting in
one sitting, and the boundary that keeps each layer legible starts paying for itself in
scrolling.

`hello-crud`'s `articles` feature runs 26, 21, 49, 47, 77 and 101 lines — a full CRUD with
pagination, validation and an OpenAPI document, none of its files halfway to the mark.
When yours crosses it, the answer is rarely a smaller function. It is that the directory
holds two resources wearing one name.
