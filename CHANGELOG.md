# Changelog

Everything notable that happens to rbs is written down here, for whoever installs it —
not for whoever reads the repository, which is what the commit log is for.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html) in shape only: **no
compatibility promise holds before 1.0**, and the public API of `rbs-core` may change
between minor versions with no deprecation cycle.

*[Version française](CHANGELOG.fr.md).*

## [Unreleased]

### Added

- `rbs add cors` installs a CORS layer whose allowed origins are read from the project's
  configuration, never wide open by default.
- `rbs add rate-limit` installs a rate limiter. The counter is a Redis pipeline when the
  `redis` fragment is there — atomic across processes — and a fixed in-memory window
  otherwise; the generated file says which one it carries and why. The 429 it returns
  follows the project's error format and carries a `Retry-After`.
- `rbs add auth` now installs `rate-limit` along with it, and says so in the plan before
  writing anything. `/auth/login` hashes an Argon2 even for an unknown address, on
  purpose: without a limit, that protection is also a way to exhaust the server's memory.
  Login is capped at 5 attempts a minute against 120 globally.
- A `// <rbs:layers>` anchor in `src/router.rs`, where a fragment stacks a middleware. It
  sits inside `trace` and `request_id`, so an added layer sees the request id and its own
  short-circuit responses stay in the trace.
- `rbs new` writes a `config/production.toml` that closes Swagger UI and the OpenAPI
  document, and the compose's `api` service sets `RBS_ENV=production`. Every Docker
  deployment used to publish both.
- `rbs-core` registers a `TooManyRequests` response under `components/responses`.

### Changed

- **The minimum supported Rust version goes from 1.85 to 1.94.** 1.85 had already stopped
  resolving: `sea-orm` 2.0.2 and `sqlx` 0.9.0 require 1.94.0, and Cargo refuses to build
  below that. The declared floor was describing a toolchain no installation could have
  used. A CI job pinned to 1.94 now holds the promise.
- The generated CRUD answers **409** instead of 500 when a `unique` constraint is
  violated, on `create` and on `update`, and its OpenAPI contract declares the status.
  The `auth` fragment already did this; the generic template did the opposite.
- The generated `list` runs its page and its `COUNT(*)` together through
  `tokio::try_join!` rather than one after the other.
- `POST /auth/register` no longer repeats the submitted address in its 409. The status
  still tells that the address is taken, but the body no longer echoes it into logs and
  responses.
- A refresh token presented twice now revokes every session of the account and logs a
  warning carrying no personal data. Until now the replay only returned 401, leaving a
  stolen pair valid indefinitely and in silence.
- `rbs dev` announces the wait for the database — `en attente de la base (host:port)` —
  then one dot per second, instead of staying silent for up to thirty seconds. Nothing is
  printed when the database answers straight away.
- The `features` anchor keeps its block sorted instead of stacking in arrival order, so a
  project whose `cargo fmt --check` runs in CI is not failed by a line it did not write.

### Fixed

- `--fields "author_id:uuid,author:references:users"` is refused instead of generating a
  project that does not compile: both fields resolve to the same `author_id` column, and
  deduplication now happens on the column name rather than on the declared name.
- Two references that singularise alike — `author` and `authors` — no longer emit the
  same `Relation` variant twice.
- A database password containing a `/` is masked in the connection error. The authority
  was cut at the first `/`, which left the `@` out of reach and the secret in the logs.
- The test generated for a required `references` field no longer violates the foreign key
  on its first run. Scenarios that create are not generated, a banner names the blocking
  reference, and an optional reference is sent as `null` instead of a random UUID.
- The error bodies of 500s, the OpenAPI descriptions and several configuration messages
  read French again: a rename toward English identifiers had reached the string literals.

## [1.1.0] — 2026-08-29

### Added

- `rbs new` writes a `docker-compose.yml` carrying the project's database, with the
  identifiers, database name and published port all taken from the URL it was given.
  `docker compose up -d` then `cargo run` are enough — nothing is retyped. Nothing is
  written for a SQLite project or for a URL whose host is not local, in both cases for
  want of anything to mount.
- `rbs add docker` now inserts its `api` and `migrate` services into the project's
  compose, under the `app` profile, instead of depositing a whole file — unless there is
  no compose to insert into, in which case it still writes one entire, deployment
  services included. A compose that has lost its `# <rbs:services>` anchor is left
  untouched, the block printed to paste back.
- `rbs add redis` and `rbs add mail` each insert their own service — `redis:8-alpine`,
  `axllent/mailpit` — into the project's compose, outside any profile: `docker compose up
  -d` alone brings them up.
- `rbs dev` mounts the compose stack whenever the project has one, regardless of whether
  `docker` is installed — the compose is the skeleton's since `rbs new`, not a mark of the
  fragment above.
- `rbs new` writes an `AGENTS.md` at the project root: the rbs handbook, written for an
  agent rather than for a reader. Two zones belong to rbs — the guide, which carries the
  version of the CLI that wrote it, and an inventory of the project — and everything
  outside them belongs to you and is never rewritten. `rbs add` and `rbs generate` refresh
  the inventory; `rbs upgrade` refreshes both zones and writes the file back if it is
  missing. The language follows `rbs new --lang fr|en`, or the locale when the flag is
  absent, and is recorded in `[package.metadata.rbs].lang`.
- `rbs doctor` checks that file — present, whole, current — and names, as a **warning**,
  any directory of `src/` that nothing declares. Writing by hand what rbs does not cover
  stays legitimate: the warning says so, and never changes the command's exit code.

### Changed

- `--with` installs the features it names instead of refusing all of them: `rbs new
  mon-api --with auth` used to fail with an explicit error and exit code 1; it installs
  `auth` now, in the same pass that writes the project. The installation order is
  derived from the names — alphabetical — rather than the order they were typed in.
- `--with jobs` is accepted: it was refused by a list the fragment's addition had left
  out of.

## [1.0.1] — 2026-08-29

### Fixed

- Both crates were published without a README: neither manifest declared one, and the
  repository's own files live outside the package — `cargo package` carries nothing from
  outside the crate. Each crate now ships its own.
- The documentation still walked new users through `--core-path`, the workaround for a core
  that was not on crates.io. It has been published since 0.4.0. The flag keeps its real
  purpose — building a project against a local core, which is how rbs is developed — and
  the getting-started path no longer mentions it.
- `rbs add` documented six features when the binary ships seven: `jobs` was missing from
  the page and from its captured help output.
- The architecture page described four "empty" core feature flags. `auth` has carried code
  since v0.2; only `redis`, `mail` and `storage` still reserve a name.

## [1.0.0] — 2026-08-29

The public API of `rbs-core` is frozen. From here on, semantic versioning is a promise and
not a shape: nothing inside the 1.x line is removed, renamed or given another meaning, and
`cargo-semver-checks` fails the build rather than let it happen. The promise covers the
format of the comment anchors and of `[package.metadata.rbs]` too, so a project generated
by one version of the CLI stays readable by the next. The [compatibility
page](https://tky0065.github.io/rbs/compatibility) sets out the five scopes.

### Added

- `rbs upgrade` aligns an existing project's manifest on the version of the CLI, and shows
  the migration notes of the jump. It writes to `Cargo.toml` and to nothing else: the code
  generated into your tree is yours from the moment it is written.
- `rbs doctor` now names that command when it finds a project behind the CLI, instead of
  describing an alignment done by hand.
- Migration notes are embedded in the binary, one per version that introduces a break.

### Changed

- **Breaking.** 22 public types of `rbs-core` carry `#[non_exhaustive]`: the 7 enums
  (`Error`, `ConfigError`, `JwtError`, `LogError`, `Status`, `Check`, `LogFormat`) and 15
  structs. An exhaustive `match` on one of those enums now needs a `_ =>` arm, and those
  structs are no longer built from a literal outside the crate — go through the
  constructor, or through the deserialised configuration. This is the price of the freeze,
  and it is paid here because after 1.0 it would have cost a 2.0.
- `Claims`, `ValidatedJson<T>` and `CommonResponses` are deliberately left out: the code
  `rbs new` and `rbs generate` write builds or destructures them. **A generated project
  crosses this version without a single line to change.**

### Fixed

- The documented PostgreSQL floor was 18, a requirement that fell when generated models
  started setting the v7 identifier themselves. `rbs doctor` enforces 14, and the guides
  now say so.

## [0.4.0] — 2026-08-28

This first entry is the first published version, so it only adds. It gathers the four
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

[1.0.1]: https://github.com/tky0065/rbs/releases/tag/v1.0.1
[1.0.0]: https://github.com/tky0065/rbs/releases/tag/v1.0.0
[0.4.0]: https://github.com/tky0065/rbs/releases/tag/v0.4.0
