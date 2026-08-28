# Changelog

Everything notable that happens to rbs is written down here, for whoever installs it —
not for whoever reads the repository, which is what the commit log is for.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html) in shape only: **no
compatibility promise holds before 1.0**, and the public API of `rbs-core` may change
between minor versions with no deprecation cycle.

*[Version française](CHANGELOG.fr.md).*

## [Unreleased]

Nothing has been published yet, so this first entry only adds. It gathers the four
milestones the repository has delivered — the foundation, authentication, integrations and
comfort — into what a single install now gives you.

### Added

**The `rbs` command.** Seven commands: `new` creates a project that boots, with its
database, its migrations and its `/health` route; `generate crud` and `generate feature`
write a feature into an existing project; `add` installs a feature fragment; `migrate`
drives migrations, `seed` inserts demonstration data, `dev` restarts the server on every
change, and `doctor` diagnoses a project.

**`rbs generate crud`, CLI first.** From `--fields 'title:string,body:text'` alone, and
with no database running, it writes the SeaORM entity, the DTOs, the repository, the
service, the controller, the migration, the seed and the integration tests. That is the
reverse of `sea-orm-cli generate entity`, which needs the tables to exist first.

**Generated code you own.** Every feature follows one shape —
`model · dto · repository · service · controller` — with dependencies going one way only:
`controller → service → repository → model`. It is plain Rust, with no macro to unfold and
no "generated, do not edit" banner, because nothing regenerates over your changes.

**Anchors instead of AST rewriting.** The CLI inserts into comment anchors you can see and
move (`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`,
`<rbs:state_champs>`, `<rbs:state_init>`). A missing anchor writes nothing and prints the
block to paste. Commands that touch an existing project read, plan, check, show, then
apply — all or nothing, restoring on partial failure, and idempotent through
`[package.metadata.rbs]`.

**`rbs-core`, the runtime.** Typed errors rendered as RFC 9457 problem documents;
configuration loaded from `config/*.toml` and the environment, validated at boot; a log
formatter that stays readable in development and turns to JSON in production; database
connection and application state; `request_id` and tracing middlewares; a validated JSON
extractor; pagination; OpenAPI helpers and a configurable Swagger UI.

**`rbs add auth`.** Registration, login, refresh with token rotation, logout and
revocation, a `require_role` guard, the `users` and `refresh_tokens` migrations, a `Role`
enum, and the routes registered in the OpenAPI document. Behind it, in `rbs-core` under
the `auth` feature: Argon2 hashing, JWT signing and verification, an `Identity` extractor,
and opaque tokens stored as fingerprints.

**`rbs add redis`, `rbs add mail`, `rbs add storage`.** A typed cache over a connection
pool; a mail transport with its templates; a `Storage` trait with a filesystem backend and
an S3 backend. The three were added without touching the core: a fragment declares its
dependencies, its configuration section and its state fields in its own `feature.toml`.

**`rbs add jobs`.** Background jobs, enqueued in the same transaction as the business
write that triggers them, and a worker that reserves, retries, and eventually gives up —
a job survives the restart of the process that was running it.

**`rbs dev`.** Starts the services the project needs, applies the pending migrations, then
runs the server and restarts it on every source change.

**`rbs seed`.** Demonstration data, in `src/seeds/` with its own binary. `generate crud`
drops the seed of the entity it just created, and the command refuses to run under
`RBS_ENV=production` unless told otherwise.

**Three database engines.** `rbs new --database postgres|mysql|sqlite`. Identifiers are
v7 UUIDs written by the application rather than by the database, so the three engines
behave alike; `rbs-core` no longer names PostgreSQL anywhere.

**`rbs doctor`.** Checks the anchors, the `.env`, whether the database answers, the
versions in use, and the configuration of every installed feature.

**Four example projects**, compiled in CI on Linux, macOS and Windows, and used as the
source of every code excerpt in the documentation: `hello-crud`, `blog-auth`, `file-drop`
and `newsletter-queue`.

**A bilingual documentation site**, at <https://tky0065.github.io/rbs/>: getting started,
architecture, CLI reference and guides, in English and French.

### Requirements

Rust 1.85 or later, Rust edition 2024. A generated project runs on PostgreSQL 14 or later,
MySQL 8.0 or later, or SQLite 3.35 or later — `rbs doctor` refuses anything below those.

[Unreleased]: https://github.com/tky0065/rbs/commits/main
