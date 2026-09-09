---
sidebar_position: 7
title: Moving long work out of the request
---

# Moving long work out of the request

This is the seventh of nine tutorials. It picks up `demo` exactly where [Setting
up](./setup.md) left it: running, with nothing mounted but a health check. The case: a
campaign of 5,000 letters, enqueued without making the caller wait for any of them to
send.

## What you need

Nothing beyond [Setting up](./setup.md): the same `demo`, its database reachable. This
page's checks run as `cargo test`, not `curl` — nothing here needs the server running.

## 1. Install the feature

```bash
rbs add jobs
```

{/* rbs:transcript cmd="rbs add jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add jobs
jobs : jobs en arrière-plan : une table, un enfilage transactionnel, un worker qui réessaie

plan pour …/demo

  + src/modules/jobs/mod.rs                         créé
  + src/modules/jobs/config.rs                      créé
  + src/modules/jobs/model.rs                       créé
  + src/modules/jobs/queue.rs                       créé
  + src/modules/jobs/worker.rs                      créé
  + src/modules/jobs/demo.rs                        créé
  + src/modules/jobs/tests.rs                       créé
  + migration/src/m20260909_090303_create_jobs.rs   créé
  ~ migration/src/lib.rs                            modifié
  + src/modules/mod.rs                              créé
  ~ src/lib.rs                                      modifié
  ~ src/main.rs                                     modifié
  ~ Cargo.toml                                      modifié
  ~ config/default.toml                             modifié
  ~ AGENTS.md                                       modifié

  15 fichiers à écrire
✓ jobs installée — 8 fichiers

  rbs migrate up, puis inscrivez vos jobs dans src/modules/jobs/mod.rs
```

Like every brick `rbs add` installs, `jobs` mounts no route. What it does wire on its own
is the worker: `src/main.rs` gains a `// <rbs:startup>` line that spawns it before the
server starts, so it is already polling by the time `cargo run` answers — against a table
that does not exist yet, which is exactly why it finds nothing to do until the next step.
Enqueuing anything stays yours to write, on the model of the demo `Log` job the install
already registered in `registry()`.

The trade-off this page is about reads best against [Sending mail](./mail.md)'s
`send_detached`: that call spawns a task and returns, and an SMTP server down for the
minute it takes to try loses the mail outright — a line in the log, and nothing else. A
job pays for the opposite guarantee, and the price is real: throughput is bounded by the
database doing the reserving, not by however many tasks a process can spawn.

## 2. Apply the migration

```bash
git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m "jobs installée"
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.51s
     Running `target/debug/migration up`
✓ migrations appliquées
```

Proof the `jobs` table now exists, and with it something for the worker already running
to poll: the migration `add jobs` wrote a moment ago is applied, and a job enqueued from
here on is a row, not something held only in the memory of the process that queued it.

## Verify

The point this whole page is built on never leaves the database, so no mounted route
would show it. `add jobs` wrote it as a test, `#[ignore]`d because it needs the table
just migrated:

```bash
cargo test modules::jobs::tests:: -- --ignored
```

```text
running 4 tests
test modules::jobs::tests::a_job_enqueued_in_a_rolled_back_transaction_does_not_exist ... ok
test modules::jobs::tests::a_job_enqueued_in_a_committed_transaction_is_visible_to_the_worker ... ok
test modules::jobs::tests::a_failing_job_is_retried_then_marked_failed_after_the_last_attempt ... ok
test modules::jobs::tests::two_concurrent_workers_never_reserve_the_same_job ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.43s
```

Proof of the whole point: `a_job_enqueued_in_a_rolled_back_transaction_does_not_exist`
enqueues inside a transaction, rolls it back, and asserts the row is gone — the queue is
a table, so `INSERT` and rollback apply to it exactly as they do to any other row that
transaction touched. Its neighbor runs the same call with `commit` instead, and the very
next line reserves it. Move the queue to Redis, or anywhere outside the database, and
both tests would need rewriting: a rollback there leaves the job standing, disagreeing
with the row that motivated it.

## What was installed

Three extracts, read from
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue) —
a project generated the same way and compiled in CI.

:::note
`newsletter-queue` carries a `subscribers` resource with a `POST /subscribers/broadcast`
action that enqueues one letter per confirmed subscriber. That action is domain code, not
something `rbs generate crud` produces — `add jobs` only installs the queue, the trait,
and the worker below it. The three extracts here show what wiring a real campaign into
that queue looks like.
:::

### The job

`SendNewsletter` carries the subscriber's id, not their address: between enqueue and
execution they may have corrected it, and it is the one at send time that matters.

```rust file=examples/newsletter-queue/src/modules/jobs/newsletter.rs region=job
```

### The registry

Where the demo `Log` job above is registered by the install itself, `SendNewsletter` is
registered by hand — the one line `registry()` asks for once a job of your own exists.

```rust file=examples/newsletter-queue/src/modules/jobs/mod.rs region=registry
```

### The enqueue call

`broadcast` reads the confirmed subscribers and enqueues one job per subscriber, all
inside a single transaction.

```rust file=examples/newsletter-queue/src/subscribers/service.rs region=broadcast
```

The `&transaction` on this call is the entire point of the page: pass `db` instead, and
the campaign becomes two things able to disagree — subscribers read as of a committed
point, and jobs a crash between the read and the commit could still lose or duplicate,
wholesale.

## Going further

- [Jobs](../guides/jobs.md) covers the worker's polling and retry in full, and how to
  schedule a job for later with `enqueue_at`.
- [`rbs add`](../cli/add.md) covers the twelve other features `demo` could still install,
  `jobs` now on it.
- [Testing](../guides/testing.md) is the harness the generated `jobs/tests.rs` runs
  against, and what `-- --ignored` reaches that a plain `cargo test` does not.
- [Seeing what the API does](./observability.md) is the next tutorial: a route has gone
  slow, and `/metrics` is what finally says since when.
