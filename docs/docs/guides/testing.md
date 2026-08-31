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

Others check the error paths that the runtime handles on its own. Two are always written:
an unknown identifier returns 404, and an unparsable body returns 400. Two more depend on
what you asked `--fields` for, because nothing else could reach them: a field carrying the
email constraint earns a body that parses but does not validate, which returns 422; a
`unique` column earns a replayed value, which returns 409. Each of those four statuses is
described in the [errors guide](./errors.md).

Text values carry a random suffix, and every test deletes the rows it created. Without
either, a `unique` field would make the second run of the suite fail on what the first one
left behind — the tests share the database your `.env` names, and no transaction rolls
them back.

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
PostgreSQL container with `testcontainers`, generate a project into a temporary
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

### Which PostgreSQL the harness starts

Two versions matter, and CI runs the suite against both.

**18 is what ships.** It is what the generated `docker-compose.yml` pins, so it is what a
project actually meets. It is the default here: a harness that starts something other than
what ships proves nothing about what ships.

**14 is the floor.** It is what `rbs doctor` enforces — the oldest release still receiving
security fixes — and a floor nothing exercises is a promise nobody keeps. For the reason
given in the [migrations guide](./migrations.md), generated primary keys are set by the
model rather than by a column default, so nothing a generated project runs needs the
`uuidv7()` that arrived with PostgreSQL 18. That claim is now tested rather than asserted.

`RBS_TEST_PG` picks the version, and 18 applies when it is unset:

```bash
RBS_TEST_PG=14 cargo test -p rbs-cli --no-fail-fast -- --ignored
```

The variable is read when the container starts, not at compile time, so both branches of
the matrix share one build and differ only in what Docker pulls. Every starter in the
repository — the three in the integration tests, the one in the generator bench — resolves
its image through the same function, so no version can be pinned behind the matrix's back.
