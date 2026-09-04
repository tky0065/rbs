---
sidebar_position: 11.5
title: Scheduler
---

# Scheduled triggers

`rbs add scheduler` gives a project a calendar: six files under `src/scheduler/`, a
migration for the `schedules` table, and a ticker started with the server. It is the answer
to the last line of the [jobs guide](./jobs.md) — a queue knows how to run work and retry
it, but nothing enqueues anything except an event of your own.

**The scheduler triggers; it does not execute.** A tick reserves a due schedule and
enqueues a job in the `jobs` table; the worker does the rest. Retries, the registry, the
logging and the execution already exist and are proven — rewriting them for the sole reason
that a clock started them would leave two loops to maintain instead of one.

That is why the fragment requires `jobs`, and it is the only one in the
[`rbs add` table](../cli/add.md#the-eleven-features) that pulls another feature along
besides `auth`. On a bare project, `rbs add scheduler` lays down `jobs` first and
`scheduler` second, in a single plan:

```text
$ rbs add scheduler
scheduler : déclenchement calendaire : une échéance due enfile un job, une seule fois entre réplicas
scheduler exige jobs : posée avec elle

plan pour /private/tmp/rbs-demo/blog

  + src/jobs/mod.rs                                      créé
  + src/jobs/config.rs                                   créé
  + src/jobs/model.rs                                    créé
  + src/jobs/queue.rs                                    créé
  + src/jobs/worker.rs                                   créé
  + src/jobs/demo.rs                                     créé
  + src/jobs/tests.rs                                    créé
  + migration/src/m20260903_173943_create_jobs.rs        créé
  ~ migration/src/lib.rs                                 modifié
  ~ src/lib.rs                                           modifié
  ~ src/main.rs                                          modifié
  ~ Cargo.toml                                           modifié
  ~ config/default.toml                                  modifié
  + src/scheduler/mod.rs                                 créé
  + src/scheduler/config.rs                              créé
  + src/scheduler/model.rs                               créé
  + src/scheduler/sync.rs                                créé
  + src/scheduler/ticker.rs                              créé
  + src/scheduler/tests.rs                               créé
  + migration/src/m20260903_173943_create_schedules.rs   créé
  ~ AGENTS.md                                            modifié

  21 fichiers à écrire
✓ scheduler installée — 15 fichiers

  rbs migrate up, puis déclarez vos échéances dans src/scheduler/mod.rs — les expressions sont évaluées en UTC
```

Two migrations come with it, so [`rbs migrate up`](../cli/migrate.md) is the next command:
until both tables exist, neither the ticker nor the worker has anything to read.

## Declaring a schedule

The calendar is declared in code, in `src/scheduler/mod.rs`, and the database holds nothing
but its state. `schedules()` is to the ticker what `registry()` is to the worker — the one
list you edit:

```rust
pub fn schedules() -> Vec<Schedule> {
    vec![Schedule::every::<crate::jobs::demo::Log>(
        "0 3 * * *",
        || crate::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    )]
}
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

## Five fields or six

The `cron` crate expects six fields, seconds first: `0 0 3 * * *`. A Unix crontab has five,
and `0 3 * * *` is what everybody has in their fingers. The fragment accepts both — five
fields are prefixed with `0 `, which is what a crontab line means anyway, and six pass
through untouched. Any other length is refused by name:

```text
`0 3 * *` porte 4 champ(s) : une expression cron en compte cinq (minute heure jour mois jour-de-semaine) ou six, la seconde en tête
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

A schedule is reserved by a conditional `UPDATE`:

```sql
UPDATE schedules
SET next_run_at = ?, last_run_at = ?, updated_at = ?
WHERE kind = ? AND next_run_at <= ?
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
- a `kind` already known **keeps its `next_run_at`**. A restart neither replays a past
  occurrence nor pushes back an imminent one, which is what makes a deployment invisible to
  the calendar.

## Changing an expression

The third rule is worth stating the other way round: **changing the expression of an
existing schedule only takes effect at its next trigger.** The table keeps the old due date
until then, and the reconciliation deliberately leaves it alone — it cannot tell a
rescheduling from a redeploy, and guessing wrong would either replay a task or delay it.

To force the new expression immediately, delete the row and restart:

```sql
DELETE FROM schedules WHERE kind = 'nightly_purge';
```

The next boot finds the `kind` missing, and inserts it at the next occurrence of the
expression the code now carries.

## Configuration

```toml
[scheduler]
poll_interval_secs = 30
```

One setting: how long the ticker sleeps between two examinations of the calendar. A
schedule accurate to the minute has no need of a wake-up every second, and thirty seconds
bound how late a trigger can be to thirty seconds. `config/{env}.toml` and
`RBS_SCHEDULER__POLL_INTERVAL_SECS` override it like any other section — see the
[configuration guide](./configuration.md).

## Testing

The generated `src/scheduler/tests.rs` runs against a real database, like every test that
touches one — see the [testing guide](./testing.md). Five of them are the ones worth
keeping when you edit the fragment:

- a five-field expression and its six-field form give the same next occurrence, and any
  other length is refused;
- reconciliation inserts a new `kind`, deletes a withdrawn one, and leaves the
  `next_run_at` of a known one untouched;
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
