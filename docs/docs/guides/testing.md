---
sidebar_position: 6
title: Testing
---

# Testing

`rbs generate crud` writes a test file next to the feature it generates, and mounts it in
the feature's `mod.rs`. The tests it contains go through HTTP, against a real database.
They are a starting bench, not a suite: they prove the wiring, and leave the rules to you.

## The bench

The application is mounted in-process. No socket is opened, no server task is spawned —
the router is built exactly as `main` builds it, and requests are handed to it directly:

```rust file=examples/hello-crud/src/articles/tests.rs region=harnais
```

Configuration is loaded the same way the binary loads it, which means the tests talk to
the database named in your `.env`. **They assume the migrations have already been
applied.** They are not run against a mock or an in-memory substitute: a repository that
compiles against SeaORM but writes broken SQL is precisely the failure a mock would hide.

## What the CLI generates

One test walks the full lifecycle of the resource — create, read, list, update, delete,
then read again to confirm the resource is gone:

```rust file=examples/hello-crud/src/articles/tests.rs region=cycle_de_vie
```

Two more check the error paths that the runtime handles on its own: an unknown identifier
returns 404, and an unparsable body returns 400. Both are shown in the
[errors guide](./errors.md).

Text values carry a random suffix. Without it, a `unique` field would make the second run
of the suite fail on the row the first one left behind.

## What it leaves to you

Everything that is specific to your domain, which is everything that matters:

- business rules — what makes a value acceptable beyond its type;
- authorization — who may read, who may write;
- edge cases of your own — concurrency, pagination boundaries, states a resource cannot
  leave.

The generated file is ordinary Rust in your source tree. Add to it, split it, delete the
parts that stop being useful. Nothing marks it as generated, because nothing should stop
you from editing it.

## Running them

From the project root, with a database reachable:

```bash
rbs migrate up
cargo test
```

The first command is not optional. `application()` fails with a message saying so if the
schema is not there.

## How rbs tests itself

The framework's own integration tests do not assume anything is running. They start a
PostgreSQL 18 container with `testcontainers`, generate a project into a temporary
directory, apply its migrations and run its tests — the `rbs` binary invoked exactly as
you would invoke it.

**These tests are slow and they require Docker.** Starting a database and compiling a
complete Axum + SeaORM project takes minutes, so they are marked `#[ignore]` and stay out
of an ordinary `cargo test`:

```bash
cargo test -p rbs-cli --test integration_crud -- --ignored
```

Slow as it is, this is the only test that proves rbs actually works. Everything else
checks a string.

PostgreSQL 18 is not negotiable there, for the reason given in the
[migrations guide](./migrations.md): generated primary keys default to `uuidv7()`, which
is native only from that version on.
