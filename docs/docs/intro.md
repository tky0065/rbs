---
sidebar_position: 1
slug: /
title: Introduction
---

# rbs

rbs is a web API framework for Rust, built on Axum and SeaORM. It gives a project the
things that have no reason to differ from one API to the next — error handling, logging,
configuration, database access, OpenAPI documentation — and generates the rest into your
own source tree, where you can read and change it.

That boundary is the whole design. `rbs-core` carries the runtime. The `rbs` command-line
tool writes features into your project: model, DTO, repository, service, controller. None
of it is marked "do not edit" — it is written to be edited.

## Status

Version 0.4.0. The four milestones of the roadmap are delivered — the foundation,
authentication, integrations, comfort. **rbs follows semantic versioning from 1.0 on**: the
[compatibility page](./compatibility.md) says what the promise covers, and what it
deliberately leaves out.

## What generated code looks like

This is the `POST /articles` handler, as `rbs generate crud` writes it. It is read from
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), a
project CI compiles — no code block in this documentation is typed by hand:

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

The controller does no more than that: it hands the request to the service and maps the
result to a status code. The service receives the `DatabaseConnection` and hands it on
without ever querying through it: of the six files in a feature, `repository.rs` is the
only one that names an `Entity`.

## Where to go next

- **[Getting started](./getting-started.md)** — from installation to a CRUD API that
  answers.
- **[Architecture](./architecture.md)** — the core/generated boundary, the anatomy of a
  feature, the dependency rule.
- **[CLI reference](./cli/new.md)** — every command and flag, with real output.
- **Guides** — [configuration](./guides/configuration.md), [logging](./guides/logs.md),
  [errors](./guides/errors.md), [OpenAPI](./guides/openapi.md),
  [migrations](./guides/migrations.md), [testing](./guides/testing.md).

The [roadmap](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) lists what is in scope
and what is deliberately left out.
