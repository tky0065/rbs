---
sidebar_position: 6.5
title: rbs test
---

# `rbs test`

Runs the project's tests the way its CI does: the services it needs, the pending
migrations, then `cargo test` over the whole workspace, database tests included. One
command instead of three, and the exit code is `cargo test`'s own.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs test --help
Lance les tests du projet : services, migrations, puis cargo test sur tout le workspace

Usage: rbs test [OPTIONS] [FILTRE] [-- <ARGS>...]

Arguments:
  [FILTRE]   Ne lance que les tests dont le chemin contient ce motif
  [ARGS]...  Arguments du harnais de test, passés après `--` (ex. --nocapture)

Options:
      --no-compose  Ne remonte pas les services du compose : ils tournent déjà, ou ailleurs
      --no-migrate  N'applique pas les migrations en attente
  -h, --help        Print help
  -V, --version     Print version
```

| Argument or option | Effect |
|---|---|
| `FILTRE` | Passed to `cargo test` as its filter: only the tests whose path contains it run — `rbs test articles` runs the tests of the `articles` feature. |
| `--no-compose` | Skips `docker compose up -d`, as with [`rbs dev`](./dev.md#synopsis): the services are already running, or run elsewhere. The wait for the database stays. |
| `--no-migrate` | Skips `rbs migrate up`: the tests run against the schema as it stands. |
| `-- ARGS` | Everything after `--` goes to the test harness, after `--include-ignored`: `rbs test -- --nocapture`, `rbs test articles -- --test-threads=1`. |

## The plan

Like [`rbs dev`](./dev.md), `rbs test` shows what it is about to do before doing it. The
first three steps are `rbs dev`'s — the same code plans them — and the server is replaced
by the tests:

```text
  base        127.0.0.1:1
  migrations  rbs migrate up
  tests       cargo test --workspace --no-fail-fast articles -- --include-ignored --nocapture
```

1. **`docker compose up -d`**, if the project has a `docker-compose.yml` and
   `--no-compose` is absent;
2. **waiting for the database**, skipped for SQLite. The same two waiting times as
   `rbs dev`: 30 seconds after bringing the compose stack up, 3 otherwise;
3. **[`rbs migrate up`](./migrate.md)**, unless `--no-migrate` — the generated tests
   assume the schema is there;
4. **`cargo test --workspace --no-fail-fast [FILTRE] -- --include-ignored [ARGS]`**, with the
   variables of the project's `.env`, exactly as [`rbs migrate`](./migrate.md) runs its
   own binary.

The last line is the command the workflow installed by [`rbs add ci`](./add.md) runs, flag
for flag. Each flag earns its place:

- `--workspace` covers the `migration` crate as well as the application;
- `--no-fail-fast` keeps going after the first red test binary, so one run shows every
  failure instead of the first one;
- `--include-ignored` runs the tests that reach the database. They are marked
  `#[ignore = "joint la base du projet"]` so that a bare `cargo test` stays fast where
  nothing is running; here the database has just been brought up and migrated, so they
  have every reason to run. See the [testing guide](../guides/testing.md).

## Exit code

The exit code is the one `cargo test` returned, unchanged — 101 when a test fails or the
project does not compile. A CI
that chains `rbs test` can therefore tell a red test apart from a command that could not
start, whose code is 1, 2 or 3 depending on what stopped it — see
[exit codes](./doctor.md#exit-codes). A red test ends on:

```text
erreur : `cargo test` a échoué (code 101)
```

## Failures

Everything before the tests fails the way [`rbs dev`](./dev.md#failures) does, with the
same messages:

```text
en attente de la base (127.0.0.1:1) ...
erreur : rien ne répond sur 127.0.0.1:1 : la base du projet n'est pas démarrée

démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez RBS_DATABASE__URL dans le .env du projet
```

| Situation | What happens |
|---|---|
| No `.env`, or no database URL in it | Refusal naming the file and the variable, exit 1 |
| Nothing listening | The message above, after the applicable timeout, exit 3 |
| Migration fails | The migration binary's own error, and no test runs, exit 1 |
| A test fails | `cargo test`'s report, then the line above, and its exit code |
| Not in a project | Refusal naming what it looked for, exit 2 |
