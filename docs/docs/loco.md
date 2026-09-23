---
sidebar_position: 8
title: rbs and Loco
---

# rbs and Loco

[Loco](https://loco.rs) stands on the same ground as rbs: Axum for HTTP, SeaORM for the
database, a command-line tool that scaffolds models, controllers and migrations. Anyone
who finds one of them meets the other within the hour, and asks the fair question: why
two?

This page does not answer it with a feature table. A table compares two lists on the day
it is written and is wrong from the other project's next release. What does not move from
one release to the next is where each project draws its lines — what the framework keeps,
what it hands over, how the generator touches your files — and that is what this page
compares.

:::info[How the claims about Loco were checked]
Every statement about Loco below was checked on **2026-09-22** against **Loco 1.1.0**, the
latest release of [`loco-rs` on crates.io](https://crates.io/crates/loco-rs), published on
2026-08-16. Each one links to the documentation page or the source file it rests on;
links into the repository point at the `v1.1.0` tag, so they keep saying what they said
that day. Loco moves: if a link now says something else, the link is right and this page
is late.
:::

## Two answers to the same question

Every framework decides what it keeps and what it hands over.

Loco describes itself as ["strongly inspired by Rails"](https://github.com/loco-rs/loco/blob/v1.1.0/README.md),
with convention over configuration as its first principle. Its documentation states a
[prime directive](https://loco.rs/docs/explanation/why-batteries-included/): reach for a
built-in first, for a generator second, and hand-wire only as a last resort. The built-ins
— database access, background jobs, scheduler, mailer, storage, cache, views, JWT auth —
are compiled into the `loco-rs` crate behind Cargo feature flags, and when one does not
fit, Loco offers typed seams to replace it (same page).

rbs answers with a single test, applied to each piece of code:
[will a developer want to read this?](./architecture.md#the-core--generated-boundary) If
not, it goes into `rbs-core`, a dependency upgraded like any other — the connection pool,
the error type and its RFC 9457 body, the log formatter. If so, the CLI writes it into your
`src/`, and it is yours. The runtime is deliberately small, and it does not grow a
built-in where generated code would do: an optional capability such as the job queue or
sign-in arrives as a [fragment](./cli/add.md), written into your project; when the core
has to carry a primitive for it, as it does for sign-in, that primitive sits behind a
feature flag.

Neither answer is the better one in the abstract. Loco puts more in the framework, so more
of your application improves with a version bump. rbs puts more in your repository, so
more of your application reads, and changes, without knowing the framework.

## Who owns the lifecycle

The difference shows most clearly in how the application starts.

In a Loco application, startup is driven by the framework. Your `App` implements the
[`Hooks` trait](https://loco.rs/docs/explanation/architecture/), and Loco's boot sequence
calls its methods in order — configuration, context, initializers, routes, middlewares.
Most of those methods have a default, so a minimal `App` only implements a handful of
them; the rest you override when you need to change it (same page). The generated
[`main.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/src/bin/main.rs.t)
is one call: `cli::main::<App, Migrator>()`.

In an rbs project, [`main.rs`](https://github.com/tky0065/rbs/blob/main/examples/hello-crud/src/main.rs)
*is* the startup sequence: load the configuration, open the pool, build the state, bind
the socket, serve, drain on shutdown. Each step calls into `rbs-core`, but the order is
written in your file, and changing it means editing a line rather than finding the hook
that controls it. The router, the state and the OpenAPI document are generated files of
the same kind.

The Loco way is shorter to read and cheaper to upgrade. The rbs way is longer to read and
has nothing behind it to discover.

## How the generator touches your files

On this axis the two projects are closer than one might expect, and it is worth saying so.

**Neither rewrites a syntax tree.** Loco's generator templates declare *injections*: a line
appended to a file, or inserted before or after a line that matches a regular expression. The
controller template, for instance, inserts its `.add_route(...)` after the line matching
`AppRoutes::` in `src/app.rs`
([template](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/controller/api/controller.t));
the migration template inserts before a comment, `// inject-above (do not remove this comment)`,
in `migration/src/lib.rs`
([template](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/model/model.t),
[skeleton](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/migration/src/lib.rs.t)).
The injection engine, [`rrgen` 0.6](https://crates.io/crates/rrgen), works out every
injection before writing anything, and fails naming the pattern it could not find
([source](https://docs.rs/crate/rrgen/0.6.0/source/src/lib.rs)).

rbs does the same kind of textual insertion, with one rule of its own: **it only ever
inserts between comment anchors**, paired markers such as `// <rbs:startup>` and
`// </rbs:startup>`, placed by the skeleton or by a fragment and listed in one place,
which [`rbs doctor`](./cli/doctor.md) walks. A line of real code is never a landmark. The
reason is ownership: a line of code is something you are entitled to rename, reformat or
move, and a generator that searches for it makes your edits the thing that breaks the
next generation. An anchor is a line whose only job is to be found. When one is missing,
rbs writes nothing, and prints the block for you to paste where you want it.

The format of those anchors is part of rbs's [compatibility promise](./compatibility.md#anchors-and-project-metadata):
a project generated by one version of the CLI stays readable by the next.

## Code that belongs to its author

Loco's documentation says of generated code that it is
["a starting point you own and edit"](https://loco.rs/docs/explanation/why-batteries-included/).
rbs says the same, and takes it one step further: the project must not need rbs at all
once it is written.

In Loco, the generator is part of the application. `cargo loco` is a Cargo alias for
`cargo run --`
([skeleton](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/.cargo/config.toml.t)):
generating a controller runs your own binary, whose command line comes from `loco-rs`. The
`generate` subcommand is compiled into debug builds only
([`src/cli.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/cli.rs)), and `loco-gen`,
the generator crate, is a dependency of `loco-rs`
([`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/Cargo.toml)). The coupling has
its advantage: the generator always matches the framework version your project compiles
against, and its templates can be overridden per project by copying them into a
`.loco-templates/` directory
([documentation](https://loco.rs/docs/how-to/override-templates/)).

In rbs, the generator is a separate tool, installed apart with `cargo install rbs-cli`. A
generated project depends on `rbs-core` and never on `rbs-cli`; its `Makefile` calls
`cargo`, `npm` and `docker compose`, never `rbs`. Clone it on a machine without the
generator and everything still builds, tests and runs. [`rbs upgrade`](./cli/upgrade.md)
writes to `Cargo.toml` and to the reserved zones of the agents guide, and creates the
files an older project lacks without ever rewriting one that exists: the code the CLI once
wrote is never read back nor re-rendered, which is why no generated
file carries a "generated, do not edit" banner.

Upgrading is where the two choices meet their price. A new Loco version reaches your code
through the framework: its [upgrade guide](https://loco.rs/docs/extras/upgrades/) is to
bump the version in `Cargo.toml`, read the changelog for breaking changes, and run
`cargo loco doctor`. A new rbs version reaches `rbs-core` the same way, but none of the
code already generated: an improvement to a template lands in the next feature you
generate, not in the ones you already have.

## Scaffolding from the command line

Both generators take a list of `name:type` fields. What they need to be running differs.

Loco's `generate model` writes a migration, **applies it against your development
database**, then regenerates the SeaORM entities into `src/models/_entities/` by calling
`sea-orm-cli` against that database
([documentation](https://loco.rs/docs/how-to/add-model/),
[`loco-gen/src/model.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/model.rs),
[`src/db/entities.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/db/entities.rs)).
Setting `SKIP_MIGRATION` writes the migration alone; the entities then come from a later
`cargo loco db entities` (same documentation page). The database schema is the source of
the entity.

rbs goes the other way: `rbs generate crud` writes the SeaORM entity **and** its migration
from `--fields`, with no database started. The field list is the source of both, and a
fresh clone can scaffold a resource before anything is running. This is the reverse of
`sea-orm-cli generate entity`, and a deliberate one; see the [`generate`](./cli/generate.md)
reference.

## When to prefer Loco

Honestly: often.

Prefer Loco if you want a framework rather than a generator — one that owns the
lifecycle, whose built-ins improve under you with each release, and whose conventions you
learn once and find in every Loco project. Prefer it if you come from Rails, which it
[openly follows](https://github.com/loco-rs/loco/blob/v1.1.0/README.md), or if you want
server-rendered views, which it [ships](https://loco.rs/docs/explanation/why-batteries-included/)
and rbs does not: rbs generates an HTTP API, with an optional Vue client beside it. Prefer
it if the features you need are already among its
[built-ins](https://loco.rs/docs/explanation/why-batteries-included/) and you would rather
configure them than read their code. And prefer it if an older project matters to you: `loco-rs` has
been published on [crates.io](https://crates.io/crates/loco-rs/versions) since November
2023, and rbs's first line dates from August 2026.

Prefer rbs if you want every line that runs your application in your own repository,
readable without knowing a framework, and a generator you can uninstall the day after.

## Sources

Consulted on 2026-09-22, Loco 1.1.0:

- [`loco-rs` on crates.io](https://crates.io/crates/loco-rs) — version, release date, and the [first release](https://crates.io/crates/loco-rs/versions), 0.1.0, on 2023-11-23
- [README](https://github.com/loco-rs/loco/blob/v1.1.0/README.md) — Rails inspiration, convention over configuration
- [Why "batteries included"?](https://loco.rs/docs/explanation/why-batteries-included/) — prime directive, built-ins behind feature flags, generated code as a starting point
- [Architecture: the request lifecycle](https://loco.rs/docs/explanation/architecture/) — the `Hooks` trait and the boot sequence
- [Add a model](https://loco.rs/docs/how-to/add-model/) — migration applied, entities regenerated, `SKIP_MIGRATION`
- [Override templates](https://loco.rs/docs/how-to/override-templates/) — `.loco-templates/`
- [Upgrades](https://loco.rs/docs/extras/upgrades/) — the upgrade procedure
- Source at `v1.1.0`: [`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/Cargo.toml), [`src/cli.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/cli.rs), [`src/db/entities.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/db/entities.rs), [`loco-gen/src/model.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/model.rs), the [controller](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/controller/api/controller.t) and [model](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/model/model.t) templates, the skeleton's [`main.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/src/bin/main.rs.t), [`.cargo/config.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/.cargo/config.toml.t) and [`migration/src/lib.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/migration/src/lib.rs.t)
- [`rrgen` 0.6.0](https://docs.rs/crate/rrgen/0.6.0/source/src/lib.rs), Loco's injection engine, a dependency of `loco-gen` ([`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/Cargo.toml))
