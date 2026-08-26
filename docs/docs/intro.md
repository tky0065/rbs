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

Version 0.1 is under construction. **There is no semver promise before 1.0**: the public
API of `rbs-core` may change between minor versions.

## What generated code looks like

This is the `POST /articles` handler, as `rbs generate crud` writes it. It is read from
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), a
project CI compiles — no code block in this documentation is typed by hand:

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

The controller does no more than that: it hands the request to the service and maps the
result to a status code. The service, in turn, never sees a `DatabaseConnection`.

## Where to go next

This documentation is being written alongside the code. The pages below fill in as the
0.1 milestone closes:

- **Getting started** — from installation to a CRUD API that answers.
- **Architecture** — the core/generated boundary, the anatomy of a feature, the
  dependency rule.
- **CLI reference** — every command and flag, with real output.
- **Guides** — configuration, logging, errors, OpenAPI, migrations, testing.

The [roadmap](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) lists what is in scope
for 0.1 and what is deliberately left out.
