# Changelog

Everything notable that happens to rbs is written down here, for whoever installs it —
not for whoever reads the repository, which is what the commit log is for.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html) in shape only: **no
compatibility promise holds before 1.0**, and the public API of `rbs-core` may change
between minor versions with no deprecation cycle.

*[Version française](CHANGELOG.fr.md).*

## [1.5.0] — 2026-09-12

### Added

- **`Identity::user_uuid()` and `Claims::user_uuid()`** read the caller's identifier as a
  `Uuid`, and answer `Error::Unauthorized` when `sub` is not one. The `auth` fragment
  called `Uuid::parse_str` with the same error mapping at seven places; each now calls the
  method, as a generated CRUD that wants the author of a write can.
- **`rbs_core::db::redact_url`** returns a connection URL with its password replaced by
  `***` — the masking `db::connect` already applied to its own errors. The `redis` and
  `rate-limit` fragments call it to quote `[cache] url` in a log; any other URL of a
  project that carries a secret can do the same.
- **`rbs generate crud` and `rbs generate feature` take `--singular <NAME>`** for the
  cases the singularisation heuristic gets wrong: `rbs generate crud news` used to name
  its types `CreateNew` and its bindings `new`, OpenAPI schemas and TypeScript interfaces
  included. `news`, `series` and `species` are now recognised as invariable without the
  flag; anything else is one `--singular news_item` away. The value must be in
  snake_case, like the feature name.
- **A generated project stops gracefully on Ctrl-C or SIGTERM.** `main.rs` now serves
  with `with_graceful_shutdown`: the listener stops accepting, in-flight requests finish,
  the `jobs` worker completes the job it is running and the `scheduler` ticker its tour,
  `main` waits for them up to the new `server.shutdown_timeout_secs` (default `30`), then
  calls `rbs_core::logs::shutdown()` itself — no more last batch of spans lost on
  `docker stop`, and no more job left `running` until the lease expires. The signal comes
  from `rbs-core`: `CoreState::shutdown()` returns a `Shutdown` any background task can
  be spawned under and listen to, so no anchor was added and the `startup` anchor's
  content is unchanged. The message of `rbs add observability` no longer asks you to call
  `logs::shutdown()` yourself. A project generated before 1.5.0 keeps its old `main.rs`,
  which `rbs upgrade` does not rewrite; the upgrade note says what to paste.
- **The `jobs` worker runs several jobs side by side, and a failed job waits longer each
  time.** `[jobs] concurrency` (default `4`) bounds how many jobs one worker executes at
  once — one webhook delivery waiting on a slow receiver no longer holds the whole queue —
  and each job runs in a task of its own, so a panicking job no longer kills the worker.
  The retry delay is now `retry_delay_secs × 2^(attempt − 1)`, capped by the new
  `retry_max_delay_secs` (default `3600`). Both keys have a default: a project generated
  before 1.5.0 keeps working without them, and takes `src/modules/jobs/worker.rs` and
  `queue.rs` from the fragment when it wants the behaviour. `rbs doctor` proposes both
  keys in the block it prints when the section is missing.
- **Every GitHub release carries prebuilt binaries, and `cargo binstall rbs-cli` finds
  them.** A tag now attaches `rbs` and `rbs-cli` for Linux (x86_64 and aarch64, built
  against the glibc of Ubuntu 22.04), macOS (Intel and Apple silicon) and Windows
  (x86_64), each archive with its SHA-256 checksum, to a GitHub release whose notes are
  this file's section for the version. `rbs-cli` declares `[package.metadata.binstall]`,
  so `cargo binstall rbs-cli` downloads the archive for your platform instead of
  compiling — installing rbs on a CI runner no longer costs a build of axum and sea-orm.
- **`rbs generate job <name>` writes a job of the queue, and `--every "<cron>"` its due
  date.** One plan creates `src/modules/jobs/<name>.rs`, declares the module between the
  new `// <rbs:job_modules>` markers, registers it in `// <rbs:jobs>` and, under
  `--every`, pushes it onto the calendar in the new `// <rbs:schedules>`. The expression
  is judged with the crate and the normalisation the project's own startup uses, before
  anything is written. The command requires `jobs` (and `scheduler` under `--every`), and
  refuses a name that is a Rust keyword, a module of the queue, `jobs`, or a crate the
  queue's code names — declared in `src/modules/jobs/mod.rs`, it would hide that
  crate. On a project generated before 1.5.0 the anchors are
  missing: the job's file is written, and the declaration, the registration and the due
  date — each of which needs the one before — are printed to paste rather than written
  without what they name. A project that received `jobs` or `scheduler` before 1.3.0 is
  refused, with the move to make by hand.
- **`rbs doctor` checks seven more fragments.** `cors` warns on an empty `origins`,
  `rate-limit` wants its section, `scheduler` reads every literal expression of the
  calendar as the startup will, `webhooks` wants the delivery registered with the queue,
  `audit` its migration declared and in the `Migrator`, `docker` the
  `config/production.toml` its compose selects, `ci` its workflow. Each names a project
  that builds and then misbehaves. `scheduler` and `webhooks` read a project that received
  them before 1.3.0 where it still carries them, under `src/`. After the upgrade, a project
  carrying `cors` sees a new warning until it lists its front's origins.
- **A generated project answers `GET /health/live`, and its Docker image probes it.** The
  new route returns `200` without querying anything: a liveness check tied to the
  database would have an orchestrator restart the API in a loop over a database outage no
  restart can fix. `/health` is unchanged and keeps answering readiness, database and
  probes included. The image `rbs add docker` builds declares a `HEALTHCHECK` on the new
  route, spoken through bash and `/dev/tcp` since the image carries neither curl nor wget.
  A project generated before 1.5.0 keeps its health module and its `Dockerfile`, which no
  upgrade rewrites; the upgrade note gives the lines to paste.
- **`rbs new` writes a one-line `CLAUDE.md` that imports `AGENTS.md`.** Claude Code reads
  `CLAUDE.md`, and reaches the handbook only through its `@AGENTS.md` import. `rbs upgrade`
  creates the file on a project that lacks it, and never rewrites one that exists.
- **`rbs test` runs a project's whole test suite the way its CI does.** It brings up the
  compose services, waits for the database, applies the migrations, then runs
  `cargo test --workspace --no-fail-fast -- --include-ignored` — the command the workflow
  of `rbs add ci` runs. A filter and arguments for libtest pass through
  (`rbs test articles -- --nocapture`), and the exit code of `cargo test` comes back
  unchanged, so a script tells a red test from a failure of the CLI. The testing guide now
  starts from it.
- **`rbs routes` lists a project's routes, and `rbs openapi export` prints its OpenAPI
  document**, neither starting a server: both read what the project's `openapi` binary
  prints, as `rbs generate client` does. `routes` shows method, path, `operation_id` and
  guard — `bearer` or `public` —, with `--json` for a script; `openapi export` writes to
  standard output, or to the file `--out` names, relative to the directory it runs in.
- **`rbs generate crud --cursor` pages the list by cursor.** `GET /<resource>` takes `after`
  and `per_page` and returns a `rbs_core::CursorPage`: no `COUNT(*)`, and a row inserted
  between two requests no longer shifts the window. The filter route keeps its numbered
  pages, since a cursor on `id` is wrong as soon as the sort falls on another column. The
  generated tests walk the pages until `next` goes out; `--soft-delete`, `--role`,
  `--with-upload` and `--has-many` combine with it.
- **Every command that plans takes `--json`.** `rbs add`, `rbs generate crud`, `feature`,
  `client` and `job`, and `rbs upgrade` then print a single JSON document on standard
  output instead of the coloured plan: every action with its full content, the insertions
  left to paste with their `bloc` and their `cause`, and the count of created and modified
  files, `applique` saying whether anything was written. A refusal becomes an `erreur`
  document carrying `code`, `message`, `remede` and `bloc`, with the exit code unchanged;
  the codes are stable, and listed in the agents guide. An argument the parser refuses
  stays text, with exit code 2.

- **A generated project compresses its responses.** The skeleton's `<rbs:layers>` block
  now carries a `CompressionLayer`, and `tower-http` gains the `compression-gzip`
  feature: `/api-docs/openapi.json`, which grows with every CRUD, and every list travel
  gzipped to any client that accepts it. The default predicate leaves small bodies,
  images and server-sent events alone. A project generated earlier keeps its router; the
  upgrade note gives the lines to paste.

### Changed

- **`rbs` speaks French from end to end in its help screens and usage errors.** clap
  wrote its own parts in English — `Usage:`, `Commands:`, `Options:`, `Print help`,
  `[default: …]`, `[possible values: …]`, and every usage error (`error:`, `tip:`,
  `For more information, try '--help'`) — around French descriptions. Headings, the
  `-h` and `-V` flags, the `help` subcommand, default and possible values, and the
  common usage errors — unknown argument, invalid value, unknown command, missing
  argument, conflict — are now French; a usage error still exits with clap's code, 2.
  The shell completions describe the options in French as well.

- **`rbs add jobs` and `rbs add scheduler` each carry one more anchor, and `schedules()`
  is written as instructions.** `// <rbs:job_modules>` sits under `pub mod worker;`, and
  `// <rbs:schedules>` under `let mut calendrier = Vec::new();` — the calendar left its
  `vec![]` literal, where an anchor does not survive rustfmt once a second element joins
  it. On a project generated earlier, `rbs doctor` fails on the missing `job_modules`,
  which `rbs doctor --fix` puts back, and only warns about `schedules`: `schedules()` has
  to be rewritten by hand first, and a healthy project must not fail a CI meanwhile. The
  upgrade note gives the form to paste.

- **A webhook subscription can no longer reach the project's own network.** Outside the
  `development` profile, `POST /webhooks/subscriptions` answers 400 to a non-`https` URL
  and to any host that is a loopback, private, link-local or CGNAT address, or
  `localhost`. At delivery the host is resolved and filtered again, inside the HTTP
  client's resolver as well, and redirects are never followed. A blocked delivery is
  abandoned rather than retried. Subscriptions registered before this version are judged
  at delivery by the same rule. The policy reads the profile from the configuration that
  `AppState::new` receives, so the skeleton now runs the `// <rbs:state_init>` anchor
  before `core: CoreState::new(db, config)` consumes it, and `rbs doctor --fix` puts that
  anchor back where it belongs when the file no longer has it at all.

- **`rbs-core` moves to argon2 0.6 and jsonwebtoken 11**, behind its `auth` feature, with
  no change to its public API. A password hashed before the upgrade still verifies and a
  token issued before it is still accepted; a token signed afterwards is byte-identical to
  one signed before, so a rolling deploy or a rollback keeps every session open. `rsa`
  still enters the lockfile through jsonwebtoken's `rust_crypto` backend, so the
  `RUSTSEC-2023-0071` exception stays.
- **`rbs add redis` writes `redis = "1.7"` and `rbs add storage` `aws-sdk-s3 = "1.146"`**
  (were `"1.6"` and `"1.144"`). A project generated earlier already resolves these
  versions through its own requirement; raising the floor in its `Cargo.toml` makes it
  explicit.
- **The `auth` controllers no longer send email themselves.** A single `notify` helper in
  the service layer renders and dispatches every message; `verification::send_link` and
  `password::send_reset_link` carry the emissions, and `service::register` now receives
  the mailer and the flow settings. Subjects and templates do not change. A render that fails is logged once as « préparation du
  courriel échouée » with a `gabarit` field, instead of one message per flow. Only a
  fresh `rbs add auth` writes the new layout; an existing project keeps its own.
- **Error responses speak the project's language, and `--lang` now covers them.** A
  generated project used to answer in two languages at once — an English `title`
  (`"Not Found"`) next to a French `detail` (`"article introuvable"`) — whatever
  `rbs new --lang` said, since the flag only chose the language of `AGENTS.md`. The new
  `[server] lang` key of `config/default.toml` (`"fr"` by default, or `"en"`) is now the
  project's language for everything a client sees: at run time `rbs-core` writes the
  `title` and fixed `detail` of every `application/problem+json` body and the common
  response descriptions of the OpenAPI document in it (`RBS_SERVER__LANG` overrides it
  there), and `rbs add` and `rbs generate crud` read it — from `config/default.toml`
  alone, never from the environment — to write the messages they hand to the client
  (`"too many requests: try again later"`, `"this value is already taken"`…). `rbs new --lang` writes it next to
  `[package.metadata.rbs] lang`, which now only decides the language of `AGENTS.md`.
  `Error::Domain` keeps its `code` as `title`, validation codes stay `validator`'s own;
  logs, code comments, emails and per-operation OpenAPI texts stay in French. **A French
  project's `title`s become French too** (`"Introuvable"`, `"Conflit"`,
  `"Validation échouée"`…): a client matching on `title` rather than `status` must be
  updated. A project generated before this version has no key and stays French;
  `rbs upgrade` rewrites nothing. To switch one to English, set `lang = "en"` under
  `[server]` — the runtime and every later `add` and `generate` follow it — then
  translate by hand the messages already generated in `src/`.

- **`POST /auth/register` answers 202 without a body, whether the address is new or
  taken.** It used to answer 201 with the profile, and 409 — before hashing — for a taken
  address: the status, and the response time, told whoever tried several addresses which
  ones were registered. Argon2 now runs in both branches. A new address still has its
  account written before the answer, so a client logs in right away; a taken one is left
  untouched, and its holder receives `templates/mail/inscription.html`. This holds for new
  projects: a project generated earlier keeps its code, and the upgrade note lists the
  files to take from the fragment. The answer alone no longer tells; a login with the
  submitted password still does, since an unverified account can log in — the auth guide
  says what would close it.

- **`forgot-password`, `resend-verification` and registration emit their tokens in a
  detached task.** The request only looks the account up; the purge, the invalidation, the
  token write and the email rendering happen after the answer, their failures logged with
  the account id — awaiting those writes let the response time say whether an address was
  registered.

- **Reset and verification links carry their token in the fragment.**
  `…/reset-password#token=…` rather than `?token=…`: a browser never sends a fragment to a
  server, so the token stays out of access logs and `Referer` headers. The client reads it
  from `location.hash`.

- **The exit code tells a script what kind of failure it is.** `1`: the project carries
  a fault the command found or that stops it — `rbs doctor` finding something, a missing
  or misplaced anchor, a migration that fails. `2`: the call is to be fixed — outside a
  project, an unknown feature, a name already taken, a dirty working tree, a conflict
  that `--force` would override — like the usage errors clap already reported with `2`.
  `3`: the environment failed — an unreadable file, `docker` or `cargo` that cannot be
  started, a database that does not answer, a CLI older than the project. Every failure
  used to exit with `1`, so a CI could not tell `rbs doctor` finding a fault from
  `rbs doctor` failing to run. `rbs test` keeps `cargo test`'s own code when a test
  fails, and a script that only checks for a non-zero status sees no difference.

- **Plans and summaries count created and modified files apart.** The plan footer reads
  `3 à créer, 6 à modifier, 2 inchangés`, and the summary `✓ cors installée — 3 créés,
  6 modifiés`: `rbs add cors` used to announce nine files to write, then call itself
  installed in three, counting only the files it had created. `rbs generate` and
  `rbs generate job` follow; the `--json` output is unchanged.

- **`rbs doctor` reports a key missing from `.env` once.** The `.env` check already names
  every key `.env.example` declares and `.env` lacks, with the line to add. The `auth`
  and `mail` checks no longer fail a second time on the same `RBS_AUTH__SECRET` or
  `RBS_MAIL__SMTP_PASSWORD`, with a different remedy, and `base` warns that it could not
  check the database instead of failing on the missing `RBS_DATABASE__URL`. A key absent
  from `.env.example` too is still reported by the feature's own check.

- **The `jobs` queue records a job's outcome without reading its row back.**
  `mark_done` and `retry_or_fail` issue a targeted `UPDATE`: `ActiveModel::update`
  returned the whole row, payload included — through `RETURNING` on PostgreSQL and
  SQLite, through one more `SELECT` on MySQL — for a model nobody read.

### Removed

- **`rbs-core` drops its empty `redis`, `mail` and `storage` features.** They had
  activated nothing since v0.3: the three live as fragments generated into the project,
  and no `feature.toml` nor any example named them. A manifest that lists one of them on
  `rbs-core` no longer resolves until it is removed — see the upgrade note.

### Fixed

- **The Redis password no longer reaches the logs.** The `redis` and `rate-limit`
  fragments quoted `[cache] url` verbatim in the error of a pool that fails to build,
  password included. Both now go through `rbs_core::db::redact_url`.

- **Every `.env` rbs writes is `0600` on Unix.** `rbs new`, `rbs add` and every other
  plan that writes the file — its rollback included — left it at the umask's mode,
  `0644` as a rule: readable by every account on the machine, database password and
  signing secret with it. The permissions are now set on the descriptor before the
  content is written, which also closes the `.env` of an older project the next time a
  plan touches it — `rbs add auth`, for one. `.env.example` keeps ordinary permissions.

- **The generated CI pins its actions by SHA.** `rbs add ci` wrote
  `actions/checkout@v7`, `dtolnay/rust-toolchain@stable` and `Swatinem/rust-cache@v2`,
  and a tag can be moved to another commit behind the project's back. Each action is now
  pinned by its SHA, the version in a comment, with `toolchain: stable` spelled out since
  the SHA no longer carries it; the fragment also writes `.github/dependabot.yml`, which
  proposes their updates every week.

- **`rbs generate crud --with-upload` writes the tests of its three content routes.** The
  flag used to mount `PUT`, `GET` and `HEAD` on `/<name>/{id}/content` and leave
  `tests.rs` without a single `/content`: a regression in one of the three handlers went
  unseen by the project's own `cargo test -- --include-ignored`. The generated file now
  carries the round trip — a binary body deposited, read back byte for byte as
  `application/octet-stream`, `HEAD` before and after, replaced by a second `PUT` —, the
  404 of an unknown id on all three verbs, the 413 one byte past `TAILLE_MAX`, and under
  `auth` the 401 of a request without a token.

- **The `fs` backend of `storage` no longer writes an object in place.** `put` used to
  `fs::write` the final path, which truncates it before filling it: a concurrent `GET` was
  served an empty or truncated body, and a crash mid-write left the truncated file under
  the final name. The bytes now go to a temporary file next to the target, one UUID per
  deposit, synced to disk and then `rename`d onto it. The root is created when the storage
  is built — a root that cannot be created fails at startup, naming the path — and the
  `/health` probe only checks that it is still a directory, instead of a `create_dir_all`
  that silently recreated a vanished root and kept the probe green on an empty store. A
  project that already carries `storage` gets the rule by copying `files.rs` from the
  fragment and adding `?` to `FileStorage::new` in `mod.rs`.

- **`rbs generate crud --with-upload` refuses a project whose `storage` fragment still
  lives at `src/storage/`** — one that received the fragment before 1.3.0 and never
  moved it under `src/modules/`. The generated service imports `crate::modules::storage`,
  so the generation used to succeed and the project stopped compiling. The refusal names
  the expected path and the move to make; nothing is written.
- **A changed cron expression takes effect at the next start, not after the next
  occurrence of the old one.** `reconcilier` used to keep the stored `next_run_at` of any
  known schedule, so going from `0 3 1 * *` to `*/5 * * * *` left the next tick on the
  first of the month, silently. A stored occurrence that is overdue is left to the ticker,
  one that matches the freshly computed occurrence stays put, and one that differs is
  replaced — with an `info` log naming the kind, the old and the new occurrence. No
  column, no migration: a project that already carries `scheduler` gets the rule by
  copying `sync.rs` from the fragment.
- **The TypeScript client types an optional nested struct as `null | T`, not
  `unknown | T`.** utoipa renders `Option<Struct>` as `oneOf: [{type: "null"}, {$ref}]`,
  and the `null` variant fell into the `unknown` fallback, which swallowed the whole
  union.
- **`modules`, `seeds`, `bin` and `lib` are refused as feature names**, like `main`,
  `router`, `openapi`, `state` and `health` before them. `rbs generate crud modules` used
  to succeed, turn `src/modules/mod.rs` into a CRUD, and break every `rbs add` that
  followed on a missing `<rbs:modules>` anchor.
- **`rbs doctor` and `rbs migrate status` no longer print cargo's `Compiling` lines
  before their verdict**, `doctor --json` included. When the CLI captures cargo's standard
  output to read it, it now captures its error output too and only replays it when the
  build fails. `migrate up`, `seed` and `dev` still show the build progress.
- **`rbs add webhooks` on a project generated before this version no longer leaves it
  unable to compile.** Such a project carries `// <rbs:state_init>` below
  `core: CoreState::new(db, config)`, which has already consumed `config` by the time
  `Sender::from_config(&config)` reads it. The command now refuses at planning time,
  writes nothing, names the line and prints the block to move above it. A fragment
  declares the line its insertion must precede with `before` on its `[[anchors]]` entry;
  `webhooks` is the only one that does.
- **A refresh token closed by logout, replayed, no longer closes the whole account.**
  `refresh_tokens` gains `replaced_at`: rotation sets it, closing (`logout`,
  `DELETE /auth/sessions`, password reset or change) sets `revoked_at`, and only a
  *replaced* token presented again triggers the family revocation — once: the replayed
  row is closed in turn, and presenting it again gets a 401 and nothing else. An attacker
  thrown out by a reset could otherwise log the victim out at will for thirty days by
  replaying a dead token.
- **An access token no longer survives the revocation of its sessions.** `rbs-core`'s
  `HasAuth` gains a provided `accept(&claims)` method that `Identity` calls after the
  signature check; the `auth` fragment implements it by reading the account: gone,
  sessions closed after `iat` (`users.sessions_revoked_at`, stamped by reset, change and
  `DELETE /auth/sessions`), or role changed → 401. Tokens are issued past the revocation
  second so the pair returned by `change-password` works at once. Generated tests that
  signed a token for a random `sub` now create an account.
- **Email addresses are trimmed and lowercased** before `register`, `login`,
  `forgot-password` and `resend-verification`. Two registrations differing only by case
  used to make two accounts — and the verification link of one landed in the other's
  mailbox.
- **`change-password`, `reset-password`, `refresh`, `verify-email` and
  `DELETE /auth/sessions` write everything or nothing.** Each used to chain its writes on
  separate connections of the pool: a failure between consuming a reset token and setting
  the password burnt the token for nothing; one between the new password and the
  revocation left the sessions of a possibly compromised account open; one between
  rotating a refresh token and issuing the new pair left the client with a dead token and
  no replacement — and its next attempt counted as a replay. Every `auth` repository now
  takes `&impl ConnectionTrait`, like `jobs::enqueue`, and the five services open one
  transaction each, committed after the last write.
- **The `AGENTS.md` guide stops miscounting, and says how to close a route by hand.** It
  gave six files for `rbs generate feature` and seven for `rbs generate crud`, one short
  each since `filter.rs`; it suggested `rbs generate feature webhooks`, a name `rbs add`
  now installs; it pointed at `clippy` as the line of its checklist that matters instead
  of `cargo test -- --ignored`; and on a project carrying `auth` it said nothing of the
  `Identity` argument, `require_role`, `security(("bearer" = []))` and the 401 and 403
  responses a hand-written route needs. `rbs upgrade` rewrites the guide zone of an
  existing project.
- **`rbs add webhooks` no longer leaves a project that `cargo fmt --check` rejects.** It
  writes three migrations under a single timestamp, and their `mod` lines were declared
  in install order, which rustfmt rewrites: the CI that `rbs add ci` generates failed on
  its first push. The `migration_modules` anchor now keeps its block sorted; the run order
  still lives in the `Migrator`'s `vec!`, which nothing reorders. A project already
  affected runs `cargo fmt` once.

#### Projects already generated

The `create_auth_tables` migration only changes for a fresh `rbs add auth`; it still
alters nothing. A project already migrated runs the statements itself — `timestamptz`
reads `timestamp` on MySQL and `timestamp_with_timezone_text` on SQLite, which is what
`rbs migrate` writes for this column type — then copies the named files from the fragment:

- `ALTER TABLE refresh_tokens ADD COLUMN replaced_at timestamptz NULL;`, then `model.rs`
  (the `replaced_at` field), the whole of `repository/refresh_token.rs` (`rotate`, `close`,
  and the `replaced_at IS NULL` filter on `open_sessions_of`, `revoke_sessions_of` and
  `revoke_session` — without it, `GET /auth/sessions` gains a row on every refresh) and
  `service/session.rs` (`refresh` and `logout`).
- `ALTER TABLE users ADD COLUMN sessions_revoked_at timestamptz NULL;`, then
  `impl HasAuth for AppState` from the fragment's `mod.rs`, `stamp_sessions_revoked` from
  `repository/user.rs` and `close_every_session` from `service/mod.rs`. Without it,
  `rbs-core` 1.5.0 compiles unchanged and nothing changes.
- `UPDATE users SET email = lower(trim(email));` — the unique key refuses it if two
  accounts differ only by case, which is the case to settle by hand — then `normalise`
  from `service/mod.rs` and its four call sites (`register`, `login`,
  `password::request_reset`, `verification::request`).
- No statement: copy the fragment's `repository/` and `service/` directories whole (and
  the two new tests of `tests/password.rs` and `tests/session.rs`) so that the five
  flows write everything or nothing. A caller that passed the connection to a
  repository compiles unchanged.
- `scheduler`: copy `sync.rs` from the fragment (and the two new tests of `tests.rs`) so
  that a changed cron expression takes effect at the next start. Nothing else changes;
  the table keeps its shape.
- Before `rbs add webhooks`: move the `// <rbs:state_init>` … `// </rbs:state_init>` block
  above `core: CoreState::new(db, config),` in `src/state.rs`. The command refuses and
  prints that block as long as it stays below the line; `rbs doctor --fix` only restores
  a missing anchor, it never relocates one that is still present.

- **A verified address is no longer verified again.** `resend-verification` issued a fresh
  token to an account already verified, and every token spent rewrote
  `email_verified_at`, making the address look younger than its first proof. A verified
  account now receives nothing, and `mark_verified` only writes a date that is still null.

- **`one_time_tokens` is purged.** The table grew by one row per request and never shrank:
  every emission now deletes the expired tokens of every account, and the migration adds
  `idx_one_time_tokens_expires_at` so that purge does not scan the table. A project
  migrated earlier creates the index by hand, as the upgrade note shows.

- **`rbs dev` on a MySQL project names MySQL** when the database URL cannot be read, and
  its remedy gives a `mysql://` URL instead of a PostgreSQL one.

- **`contains` in a generated filter matches the value literally.** `%` and `_` went
  into `LIKE` as wildcards: `{"title": {"contains": "%"}}` returned every row, while the
  comment above claimed the value was escaped. `%`, `_` and `!` are now escaped, with an
  explicit `ESCAPE '!'` that reads the same on the three engines. A feature generated
  earlier keeps its `filter.rs`; the upgrade note gives the function to paste.

- **The in-memory rate-limit counter no longer walks its table on every request under
  load.** Once the table held 10,000 keys, the sweep ran on each hit and removed only
  expired windows: with 10,000 clients active at once, every request walked the whole
  table under the lock. The next sweep now waits for the table to double.

## [1.4.0] — 2026-09-11

### Added

- **The `auth` fragment grows from five routes to thirteen.** `/auth/change-password`
  (authenticated) returns 200 with a fresh token pair rather than 204 — changing a
  password revokes every session, the caller's included, and reissuing one avoids logging
  out someone who just did the right thing; a wrong current password answers 403, not 401,
  since the caller is already identified and a 401 would send it toward a useless refresh.
  `/auth/forgot-password` and `/auth/resend-verification` (anonymous) both answer 202
  whether or not the address belongs to an account, and the email send is detached from
  the response — telling the two cases apart, or waiting on SMTP, would enumerate
  accounts. `/auth/reset-password` and `/auth/verify-email` (anonymous) consume a one-time
  token and answer the same 401 for every way it can fail — unknown, expired, already
  consumed, or issued for the other purpose. `/auth/sessions` (GET, DELETE) lists or closes
  every open session of the caller, without ever exposing a token hash in the view, and
  `/auth/sessions/{id}` (DELETE) answers 404 rather than 403 when the id names no session
  of the caller's — a 403 would confirm it exists somewhere else.
- **`register` now sends a verification email.** Its contract is unchanged — still 201,
  still a `UserResponse` — with one field added, `email_verified_at`, null at signup. A
  send failure is logged, never surfaced to the caller.
- **A `VerifiedIdentity` guard**, shipped in `src/auth/guard.rs`, that a project applies
  itself to the routes it judges sensitive. `login` keeps accepting an unverified account
  unchanged; the guard re-reads verification state from the database on every request
  rather than from the token, which would otherwise freeze it for the token's lifetime.
- **A `one_time_tokens` table**, shared between password reset and email verification and
  told apart by a `purpose` column, plus an `email_verified_at` column on `users` — both
  added by the `create_auth_tables` migration, which only creates tables and alters none.
- **Three keys under `[auth]`**: `reset_ttl_secs` (3600), `verification_ttl_secs` (86400)
  and `app_url` (`http://localhost:3000`), read by a `FlowConfig` the project owns rather
  than `rbs-core`.
- **Three more rate limits**: `/auth/forgot-password` and `/auth/resend-verification` at
  3 requests per hour, `/auth/register` at 10 — all three send an email to an address the
  caller chooses.

### Changed

- **`auth` now requires `mail` in addition to `rate-limit`.** `rbs add auth` on a project
  that does not already carry `mail` installs it alongside: `lettre` in `Cargo.toml`, the
  `[mail]` section, a `mailpit` service in `docker-compose.yml`, `templates/mail/` and
  `RBS_MAIL__SMTP_PASSWORD` in `.env.example`.
- **`src/auth/` becomes a directory per layer.** `repository/`, `service/`, `controller/`
  and `tests/` each carry several files; the fragment now writes 21 files under `src/auth/`
  instead of 8. A project already generated is untouched — `rbs` rewrites no file it has
  already written, and `rbs upgrade` never touches the code of an installed feature — but a
  fresh `rbs add auth` renders a different tree.
- `rbs-core` did not change in this release: the entire change lives in the CLI's `auth`
  fragment and its templates.

## [1.3.1] — 2026-09-09

### Changed

- **A generated filter now documents each column by its type.** `filter.rs` cites the
  schema of its column's type — `rbs_core::BoolComparisonSchema` for a `bool`,
  `TextMatchSchema` for text, and so on — where every column used to cite a single
  `ComparisonSchema` carrying a free-form value. The OpenAPI document now describes both
  forms a condition has always accepted: a `oneOf` between the bare value, typed by the
  column, and the object naming its operators. And no column is required any more — the
  document used to demand every one of them, so a validating client had to send the whole
  filter to narrow a list by one field. Swagger now offers `{ "published": true }` in place
  of an object of `"string"` on every column. Nothing changes at runtime: a body accepted
  yesterday is accepted today. A project generated earlier compiles untouched —
  `ComparisonSchema` is still exported — and picks up the new document when its filter is
  regenerated.

### Added

- **Six schemas in `rbs-core`**, one per column type a `--fields` can name:
  `BoolComparisonSchema`, `IntComparisonSchema`, `FloatComparisonSchema`,
  `UuidComparisonSchema`, `DateTimeComparisonSchema` and `TextMatchSchema`, each naming its
  operators through a companion type. They exist only to be cited by
  `#[schema(value_type = ...)]`; nothing constructs them.

## [1.3.0] — 2026-09-08

### Changed

- **On a project carrying `auth`, `rbs generate crud` now writes closed routes** — all of
  them, and not just the writes. The six routes of the CRUD, nine when `--with-upload`
  brings the content route along, each take an `identite: Identity` parameter, open their
  body with `identite.require_role(Role::User)?`, carry `security(("bearer" = []))` and
  declare a 401 and a 403 in their `#[utoipa::path]`; the generated controller opens with
  a four-line header saying so. `--role admin` no longer decides *whether* the routes are
  guarded but how high: it raises the threshold of `create`, `update`, `delete` and the
  content route's `PUT` to the named role, while the reads — `list`, `filter`, `find`,
  `GET` and `HEAD` — keep the default `Role::User`. Opening a route to the public became
  the edit rather than the default: on the handler concerned, remove the `identite`
  parameter, the `require_role` call, the `security` entry and the 401 and 403 responses
  from its annotation. The generated `tests.rs` follows — it signs the token it presents
  with `rbs_core::jwt::sign`, needing no account, exercises the whole write cycle with it,
  and adds two tests that present nothing at all, one write and one read, to pin the 401
  that answers both. A project **without** `auth` generates exactly what it generated
  before, byte for byte; anything that compares the output of `rbs generate crud` against
  a stored reference will go red on a project carrying `auth`, and only there. Two
  consequences for an existing project, since `rbs` rewrites no file it has already
  written: a CRUD generated before this version stays wide open, including on a project
  that installs `auth` afterwards, and `rbs add auth` therefore names those still-public
  features in its closing output, once the feature is installed, so that you know which ones
  to close by hand.
- **`require_role` compares a threshold instead of an equality.** It lets the call through
  as soon as the caller's role is greater than or equal to the one required
  (`porte >= minimum`), so an `Admin` satisfies a `require_role(Role::User)`; without that,
  the `Role::User` a generated CRUD names on its reads would lock the project's own
  administrators out of them. The `Role` enum consequently derives `PartialOrd, Ord`, and
  **the order in which it declares its variants now carries a hierarchy**: a role inserted
  between two others moves the threshold of every guard in the project at once, so a wider
  role belongs at the end. Only a project generated from this version on receives that
  guard; one generated earlier keeps the strict equality it was given, because `rbs` does
  not rewrite `src/auth/guard.rs` once it has laid it down, and the two semantics therefore
  coexist by the date of the project. The 1.3.0 release note, printed by `rbs upgrade`,
  carries the exact lines to replace in `src/auth/guard.rs` and in the `derive` of
  `src/auth/model.rs` to move an existing project over.
- **`rbs add` now installs ten of its modules under `src/modules/` instead of at the root
  of `src/`** — `audit`, `cache` (the directory `redis` writes), `cors`, `jobs`, `mail`,
  `observability`, `rate_limit` (what `rate-limit` writes), `scheduler`, `storage` and
  `webhooks`. The point of mount is `src/modules/mod.rs`, opened at the first fragment
  that needs it and inserted at the `<rbs:modules>` anchor from then on — the fourteenth,
  alongside the thirteen the CLI already knew. `auth` is the one fragment left out: it
  lays down the `User` entity you extend like any feature of your own, so it stays where
  your own code lives. A project generated before this version keeps its modules exactly
  where it received them — moving them would mean rewriting `use` statements the CLI does
  not own — but `rbs doctor` gained a `disposition` check that warns, without failing,
  the day such a project receives a module laid out the new way alongside ones still at
  the root. That guarantee holds for a fragment installed on its own, but not for three
  that reach into another module by its new path: `webhooks` targets the `<rbs:jobs>`
  anchor inside `src/modules/jobs/mod.rs`, and on a project that still carries `src/jobs/`
  from an earlier version the install simply fails — `src/modules/jobs/mod.rs is
  missing`, with no block to paste, but nothing written either. `scheduler` writes a
  `use crate::modules::jobs::{self, Job};`, and `rate-limit` a call to
  `crate::modules::cache::Config::load()?` when the project also carries `redis`; both
  install successfully and leave code that does not compile — `rbs doctor` reports it
  afterwards, but the install already claimed success. On a project predating 1.3.0, move
  `src/jobs/` (and, before adding `rate-limit`, `src/cache/`) under `src/modules/` and fix
  their `use` statements before installing `webhooks`, `scheduler` or `rate-limit`.

## [1.2.0] — 2026-09-04

### Added

- `rbs new --preset api|worker|full` installs a named set of features: `api` is `auth`,
  `cors`, `docker` and `rate-limit`; `worker` is `docker`, `jobs`, `redis` and `scheduler`;
  `full` is everything the binary can install, derived from what it carries rather than
  written down — a frozen list would go stale at the first fragment added. It adds to
  `--with` rather than replacing it, without repeating what both name, and in the order of
  the features rather than the order typed, so two equivalent invocations produce two
  identical projects.
- `rbs generate client --lang ts` writes a typed TypeScript client from the project's own
  OpenAPI document — one method per operation, one interface per schema, no dependency to
  install on the TypeScript side. No server runs: `rbs new` now writes a third binary,
  `src/bin/openapi.rs`, which prints what `ApiDoc::openapi()` returns, and the command runs
  it. The client is a configurable `ApiClient` class rather than free functions, so a token
  is set once at construction instead of being threaded through every call; `headers` takes
  a function as well as an object, for a token that rotates. It is projected as a creation,
  so regenerating an unchanged contract writes nothing and a client you edited comes back as
  a conflict rather than being overwritten. The binary is useful on its own: `cargo run
  --bin openapi > openapi.json` followed by a `git diff` freezes the contract in CI.
- `rbs add webhooks` installs outgoing webhooks: a `webhook_subscriptions` table, three
  routes to register, list and revoke a subscriber, and an `emit` function that enqueues one
  signed delivery per listening subscription. `emit` takes a `&C: ConnectionTrait` rather
  than a connection, for the same reason `audit::record` does: hand it the transaction
  carrying your change, and the deliveries exist if and only if that change is committed —
  an event announcing a signup that was rolled back is a lie no retry takes back. Delivery
  goes through the `jobs` queue unchanged, which is why the fragment requires it: retries,
  backoff and `last_error` were already proven, and a second retry loop would have been a
  second thing to maintain. It requires `auth` too — a subscription endpoint left open lets
  anyone have the project's events delivered to their own server. The body is signed
  HMAC-SHA256 over `<timestamp>.<raw body>` and carried as `X-Rbs-Signature: t=…,v1=…`: the
  timestamp is inside the digest, which is what closes replay. Each subscription gets its
  own secret, returned once at creation and never by the list — a shared one would give every
  subscriber what they need to forge the events delivered to all the others. The subscription
  is named by its id in the job rather than copied, so a rotated secret applies to deliveries
  already queued and a revocation stops them.
- `rbs add audit` installs a write log: an `audit_log` table, an `Entry` type and a
  `record` function under `src/audit/`. `record` takes a `&C: ConnectionTrait` rather than
  a connection, which is the whole reason the log lives in the database: hand it the
  transaction carrying your change, and the trace exists if and only if that change is
  committed. `actor_id` is nullable and takes a `String` rather than the `auth` feature's
  `Identity`, so the fragment installs on a service with no JWT and keeps writes made
  outside a request — a job, a seed, an admin command — traceable. `action` is a string
  rather than an enum, with `CREATE`, `UPDATE` and `DELETE` as constants: `login` and
  `export` are legitimate actions a closed enum would only force you around. The fragment
  mounts no route and wires itself into none of the generated CRUD — which writes deserve
  a trace is a question only your domain answers.
- `rbs add scheduler` installs calendar triggering: a due schedule enqueues a job in the
  existing queue, once, however many replicas are running. It pulls in `jobs` — the
  scheduler triggers, it does not execute — and the calendar is declared in code, in
  `src/scheduler/mod.rs`, where `Schedule::every::<J>` takes the `kind` from `J::KIND`, so
  a schedule aiming at an unregistered job cannot be written. A reservation is one
  conditional `UPDATE` on `next_run_at`, sharing its transaction with the enqueue, so no
  crash can advance a schedule without creating its job. Expressions take five fields as
  well as six — a line pasted from a crontab is served, not punished — and are evaluated
  in UTC; a single unreadable one stops start-up by name.
- `rbs generate crud --with-upload` mounts three content routes on the generated
  resource — `PUT`, `GET` and `HEAD` on `/<resource>/{id}/content` — backed by the
  `storage` fragment's trait. The body travels as `application/octet-stream`, not JSON:
  base64 would hold the file in memory twice. The storage key is derived from the `id`,
  so no column carries it. Without the `storage` feature the flag is refused before
  anything is written, naming `rbs add storage`. A body limit applies to the upload route
  alone, as a constant you can raise.
- `rbs_core::Cursor` and `CursorPage<T>` paginate on the `id` instead of an offset, for
  lists where `OFFSET n` makes the engine walk the rows it is about to discard. `after` is
  exclusive and the response carries no `total` — the `COUNT(*)` it would need is the cost
  the cursor avoids. The generated CRUD is unchanged and keeps `Pagination`: switching it
  would drop `total` from every response already being served.
- `rbs add observability` installs OTLP traces and a Prometheus `/metrics`. Traces leave
  through `rbs-core`, behind its new `observability` cargo feature: `logs::init()` posts
  the global subscriber itself, and nothing added at the `// <rbs:startup>` anchor could
  graft an export layer onto it afterwards. `OTEL_EXPORTER_OTLP_ENDPOINT` names the
  collector — absent, nothing is exported — and `rbs_core::logs::shutdown()` flushes the
  last batch. Metrics count under the route template taken from axum's `MatchedPath`, and
  never under the requested URL, and they are served on a listener of their own so that no
  deployment has to hide them behind a reverse-proxy rule. `rbs doctor` refuses a
  configuration where that port equals `server.port`.
- `--fields` takes a `max=<n>` modifier, which bounds the length of a textual field in the
  generated DTOs. It is refused on any other type.
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
- `rbs generate crud --soft-delete` makes `DELETE` logical: the row stays, its `deleted_at`
  column dated, and every read hides it. The HTTP contract is unchanged — 204 on delete,
  404 on a second one, 404 on reading a deleted row — so no client notices. A `unique`
  field moves its constraint to an index restricted to live rows, which is what lets
  someone re-register with an address they had before. **MySQL has no partial index**: the
  generated migration branches at run time and keeps a global uniqueness there, so on MySQL
  a deleted value stays reserved. The unchanged contract covers the feature carrying the
  flag, not the ones referencing it: a foreign key's `ON DELETE` never fires on a logical
  delete, so children survive a deleted parent and a `Restrict` stops refusing anything.

### Changed

- `rbs new` warns when it could not decompose the database URL. No `docker-compose.yml`
  is written in that case, and until now the project was born without one and without a
  word — the absence showed up only as a missing file. A warning and not a refusal: a Unix
  socket such as `postgres:///demo` is legitimate and does not decompose either.
- A `string` field is bounded at 255 characters in the generated DTOs, without anyone
  asking. Nothing else bounded it — `ColumnDef::string()` renders a `varchar` with no
  length on PostgreSQL — so every public route accepted a string of arbitrary length.
  `text` keeps none by default: it is the type one picks to exceed that bound.
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
- The generated feature modules no longer carry a module-wide `#![allow(dead_code)]`. A
  project generated today has a `src/lib.rs`, and a public item of a public module stays
  reachable from outside the crate: the permission masked nothing but a forgotten call.
- The mailer and the object store are reached through `state.mail()` and `state.storage()`,
  as the cache already was through `state.cache()`. Their field drops the
  `#[allow(dead_code)]` that stood in for the accessor.

### Fixed

- Every handler the CLI generates carries an `operation_id`, and the health probe carries
  its `tag`. Without them utoipa derives an identifier from the function name alone, so
  `list` on two features collided in the document — and a generated client could not name
  its methods. The five routes of the `auth` fragment, the filter route and the three
  content routes had none at all.

- `rbs dev` names the file that triggered a restart. Nothing on screen told a wanted
  restart from a server that had just died on its own.
- `POST /auth/login` and `POST /auth/refresh` answer with `Cache-Control: no-store` and
  `Pragma: no-cache`, as RFC 6749 §5.1 requires of a response carrying tokens. The header
  is carried by the `TokenPair` type rather than by the two handlers, so a third one added
  later gets it without thinking about it.
- Writing into an anchor follows the line endings of the host file. On a repository with
  `core.autocrlf=true`, the CLI laid LF lines down in the middle of a CRLF file, which the
  `cargo fmt --check` of the generated `ci` workflow could then refuse. Repairing an
  anchor rewrote the whole file in LF.
- A zone of `AGENTS.md` only opens on a marker alone on its line. Quoting
  `<!-- rbs:inventory -->` in one's own prose made `rbs upgrade` erase everything between
  that quotation and the real closing marker.
- The entity inventory no longer counts the braces of strings and comments. A
  `format!("{{")` shifted the depth and attached the following entities to the wrong
  module, and an entity commented out in a block was inventoried as a real one — both
  ending in a wrong `belongs_to`.
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

[1.5.0]: https://github.com/tky0065/rbs/releases/tag/v1.5.0
[1.4.0]: https://github.com/tky0065/rbs/releases/tag/v1.4.0
[1.3.1]: https://github.com/tky0065/rbs/releases/tag/v1.3.1
[1.3.0]: https://github.com/tky0065/rbs/releases/tag/v1.3.0
[1.2.0]: https://github.com/tky0065/rbs/releases/tag/v1.2.0
[1.1.0]: https://github.com/tky0065/rbs/releases/tag/v1.1.0
[1.0.1]: https://github.com/tky0065/rbs/releases/tag/v1.0.1
[1.0.0]: https://github.com/tky0065/rbs/releases/tag/v1.0.0
[0.4.0]: https://github.com/tky0065/rbs/releases/tag/v0.4.0
