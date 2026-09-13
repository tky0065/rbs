---
sidebar_position: 11.5
title: Scheduler
---

# Scheduled triggers

`rbs add scheduler` gives a project a calendar: six files under `src/modules/scheduler/`, a
migration for the `schedules` table, and a ticker started with the server. It is the answer
to the last line of the [jobs guide](./jobs.md) — a queue knows how to run work and retry
it, but nothing enqueues anything except an event of your own.

**The scheduler triggers; it does not execute.** A tick reserves a due schedule and
enqueues a job in the `jobs` table; the worker does the rest. Retries, the registry, the
logging and the execution already exist and are proven — rewriting them for the sole reason
that a clock started them would leave two loops to maintain instead of one.

That is why the fragment requires `jobs`, and it is the only one in the
[`rbs add` table](../cli/add.md#the-thirteen-features) that pulls another feature along
besides `auth`. On a bare project, `rbs add scheduler` lays down `jobs` first and
`scheduler` second, in a single plan:

{/* rbs:transcript cmd="rbs add scheduler" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add scheduler
scheduler : déclenchement calendaire : une échéance due enfile un job, une seule fois entre réplicas
scheduler exige jobs : posée avec elle

plan pour …/demo

  + src/modules/jobs/mod.rs                              créé
  + src/modules/jobs/config.rs                           créé
  + src/modules/jobs/model.rs                            créé
  + src/modules/jobs/queue.rs                            créé
  + src/modules/jobs/worker.rs                           créé
  + src/modules/jobs/demo.rs                             créé
  + src/modules/jobs/tests.rs                            créé
  + migration/src/m20260913_132217_create_jobs.rs        créé
  ~ migration/src/lib.rs                                 modifié
  + src/modules/mod.rs                                   créé
  ~ src/lib.rs                                           modifié
  ~ src/main.rs                                          modifié
  ~ Cargo.toml                                           modifié
  ~ config/default.toml                                  modifié
  + src/modules/scheduler/mod.rs                         créé
  + src/modules/scheduler/config.rs                      créé
  + src/modules/scheduler/model.rs                       créé
  + src/modules/scheduler/sync.rs                        créé
  + src/modules/scheduler/ticker.rs                      créé
  + src/modules/scheduler/tests.rs                       créé
  + migration/src/m20260913_132217_create_schedules.rs   créé
  ~ AGENTS.md                                            modifié

  22 fichiers à écrire
✓ scheduler installée — 15 fichiers

  rbs migrate up, puis déclarez vos échéances dans src/modules/scheduler/mod.rs — les expressions sont évaluées en UTC
```

Two migrations come with it, so [`rbs migrate up`](../cli/migrate.md) is the next command:
until both tables exist, neither the ticker nor the worker has anything to read.

## Declaring a schedule

The calendar is declared in code, in `src/modules/scheduler/mod.rs`, and the database holds nothing
but its state. `schedules()` is to the ticker what `registry()` is to the worker — the one
list you edit. The example keeps the demonstration schedule exactly as the fragment lays it
down:

```rust file=examples/event-hub/src/modules/scheduler/mod.rs region=schedules
```

`Schedule::every::<J>` takes the job as a type parameter, and reads the `kind` from
`J::KIND` rather than from a string you retype. **A schedule aiming at a job that is not in
the registry is therefore impossible to write**: the compiler refuses it, where a calendar
stored in the database would have accepted a misspelled `kind` and failed every night
without saying so.

The second argument is a factory, not a value. It is replayed at every trigger, so a
payload carrying a date gets the date of the tick and not the date of the deployment.

Changing the calendar means a deploy. That is the ordinary price of a versioned
configuration, and it is what makes the list reviewable in a diff.

[`rbs generate job <name> --every "<cron>"`](../cli/generate.md#rbs-generate-job) is the
command that writes an entry like the one above: the job's file under
`src/modules/jobs/`, its registration with the worker, and — because of `--every` — the
`calendrier.push(…)` line between the `// <rbs:schedules>` markers, all three in one plan.
`--every` refuses an expression `Schedule::compiler` would refuse at boot, and refuses a
second due date for a job the calendar already schedules under a different expression —
the row is keyed by `kind`, and the command will not silently pick one over the other.

### A calendar predating the anchor

A project generated before 1.5.0 still declares `schedules()` as a `vec![]`
literal, with no `// <rbs:schedules>` markers inside it:

```rust
pub fn schedules() -> Vec<Schedule> {
    vec![Schedule::every::<crate::modules::jobs::demo::Log>(
        "0 3 * * *",
        || crate::modules::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    )]
}
```

`rbs doctor --fix` cannot put the anchor back on such a project: an anchor is restored
beneath the line it declares as its hook, and this function's hook —
`let mut calendrier = Vec::new();` — does not exist in it yet. `rbs doctor` reports the
anchor missing, same as always, but the remedy here is a hand edit rather than a rerun. An
anchor placed inside an expression does not survive rustfmt once a second element joins it
— which is what moved [`jobs::registry`](./jobs.md#registering-it) off its chain of
`.register()` calls for `// <rbs:jobs>` — so rewrite the function to instructions, and add
the markers yourself:

```rust
#[allow(clippy::vec_init_then_push)]
pub fn schedules() -> Vec<Schedule> {
    let mut calendrier = Vec::new();
    // <rbs:schedules>
    // </rbs:schedules>
    calendrier.push(Schedule::every::<crate::modules::jobs::demo::Log>(
        "0 3 * * *",
        || crate::modules::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    ));
    calendrier
}
```

Once the anchor is in place, `rbs generate job --every` writes to it like on any other
project. `#[allow(clippy::vec_init_then_push)]` is required, not decorative: clippy prefers
a `vec![]` literal, which is exactly the one shape an anchor cannot survive inside — a
second `.push()` a fragment or a generated job adds later would be fine, but the marker
comment sitting between two literal elements is disfigured by the first `cargo fmt` that
runs after.

## Five fields or six

The `cron` crate expects six fields, seconds first: `0 0 3 * * *`. A Unix crontab has five,
and `0 3 * * *` is what everybody has in their fingers. The fragment accepts both — five
fields are prefixed with `0 `, which is what a crontab line means anyway, and six pass
through untouched. Any other length is refused by name:

```rust file=examples/event-hub/src/modules/scheduler/mod.rs region=normaliser
```

## Everything is UTC

Occurrences are computed in UTC. `0 3 * * *` is 3 a.m. UTC — not 3 a.m. in Paris, and not
3 a.m. in the machine's local time either. In summer that is 5 a.m. for a French reader,
and 4 a.m. in winter; if a task must fire at a fixed local hour across a daylight saving
change, no cron expression will do it, and the job itself has to decide whether it is due.

An unreadable expression stops the boot. Every expression is compiled before the first row
is written, and one bad expression aborts the process while naming it. Dropping the faulty
schedule and carrying on would give a service that looks healthy and whose task never runs
— the most expensive failure to diagnose. It is a deliberate departure from the worker,
which does let the API answer when its own configuration is unreadable: there an HTTP
service is at stake, here a static list the developer has just written.

## One replica wins a due schedule

Three instances of the API are three tickers, and the nightly purge has to run once. That
is the whole reason the `schedules` table exists: it is the shared state the replicas
arbitrate through.

A schedule is reserved by a conditional `UPDATE` — `WHERE kind = ? AND next_run_at <= ?`:

```rust file=examples/event-hub/src/modules/scheduler/ticker.rs region=reserve
```

`rows_affected == 1` designates the winner; the losers see zero. The condition is evaluated
under the row lock the `UPDATE` takes for itself: the second ticker waits for the first to
commit, re-reads a `next_run_at` already moved on, and affects nothing. No `SKIP LOCKED`,
no dialect-specific SQL — unlike dequeuing a job, there is no row to *elect* here, the row
is named by its primary key.

**The reservation and the enqueue share one transaction.** `begin`, conditional `UPDATE`,
and — if the row was won — `enqueue`, then `commit`. Without that transaction a crash
between the two would move the schedule on without ever creating the job, and nobody would
notice before the next occurrence.

The table is small by construction: one row per declared schedule, keyed by the job's
`kind`.

| Column | Type | Note |
|---|---|---|
| `kind` | `varchar(191)`, primary key | The triggered job's `KIND`. The primary key *is* the uniqueness |
| `next_run_at` | `timestamptz` | The due date. The reservation compares it, then moves it on |
| `last_run_at` | `timestamptz`, nullable | The last trigger, or nothing until there has been one |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

## What a restart does

At boot the ticker reconciles the table with `schedules()`:

- a `kind` declared in the code and absent from the table is **inserted**, its
  `next_run_at` set to the next occurrence of its expression;
- a `kind` present in the table and **no longer declared** is deleted — otherwise a
  schedule removed from the code would stay due forever, reserved by nobody;
- a `kind` already known whose `next_run_at` is **due** is left alone: the next tick
  reserves it, then moves it on by the expression the code now carries. A restart never
  replays a past occurrence, and never loses one either;
- a `kind` already known whose `next_run_at` is **still ahead** is compared to the next
  occurrence of its expression, computed now. Equal — the ordinary redeploy — nothing
  moves; different, the row is moved to the new occurrence and an `info` log names the
  `kind`, the old due date and the new one.

## Changing an expression

The last rule is what makes a change to `schedules()` take effect **at the next boot**.
Before it, the table kept the old due date until the old expression fired: going from
`0 3 1 * *` to `*/5 * * * *` left the next tick on the first of the month, without a log.

The one case the boot leaves alone is a schedule that was already due when it happened —
a `0 3 * * *` restarted at 03:10 fires at the first tick, as it would have without the
change, and only then follows the new expression. Moving it would silently drop an
occurrence the process had already earned.

## Configuration

```toml file=examples/event-hub/config/default.toml region=scheduler
```

One setting: how long the ticker sleeps between two examinations of the calendar. A
schedule accurate to the minute has no need of a wake-up every second, and thirty seconds
bound how late a trigger can be to thirty seconds. `config/{env}.toml` and
`RBS_SCHEDULER__POLL_INTERVAL_SECS` override it like any other section — see the
[configuration guide](./configuration.md).

The sleep is interruptible: on Ctrl-C or SIGTERM the ticker finishes the tour it has
started — each schedule is a short transaction — starts no other, and returns, so `main`
does not wait thirty seconds for it before exiting.

## Testing

The generated `src/modules/scheduler/tests.rs` runs against a real database, like every test that
touches one — see the [testing guide](./testing.md). Five of them are the ones worth
keeping when you edit the fragment:

- a five-field expression and its six-field form give the same next occurrence, and any
  other length is refused;
- reconciliation inserts a new `kind`, deletes a withdrawn one, leaves the `next_run_at`
  of a known one untouched when its expression has not changed, moves it when the
  expression has, and leaves a due one where it is;
- a due schedule is reserved — `next_run_at` moves on, `last_run_at` is set, and a job of
  the right `kind` has appeared in the queue;
- **two concurrent reservations of the same schedule: only one wins**, and the queue holds
  exactly one job;
- a schedule that is not due is left alone.

The fourth is the one that justifies the table. Without it, the guarantee is only an
intention.

## What it leaves to you

- **what a schedule does** — it enqueues a job, and the job is yours. The generated
  `demo::Log` is there to be replaced;
- **local hours** — expressions are UTC, and nothing converts them;
- **catching up after downtime** — a schedule missed while the process was down fires once
  at the next tick, not once per missed occurrence;
- **watching the calendar** — the rows are there, `last_run_at` says when each one last
  fired, and nothing looks at them.
