---
sidebar_position: 11.5
title: Audit log
---

# Audit log

`rbs add audit` installs a write log into an existing project: four files under
`src/modules/audit/`, and a migration for the `audit_log` table. Like the other bricks, it mounts
no route — and unlike them, it does not even wire itself into the ones you already have.
Calling it is your service's job, and the reason is [below](#what-the-fragment-does-not-do).

## What gets installed

{/* rbs:transcript cmd="rbs add audit" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add audit
audit : journal des écritures : qui a modifié quoi, quand, dans la transaction du changement

plan pour …/demo

  + src/modules/audit/mod.rs                             créé
  + src/modules/audit/model.rs                           créé
  + src/modules/audit/repository.rs                      créé
  + src/modules/audit/tests.rs                           créé
  + migration/src/m20260913_132219_create_audit_log.rs   créé
  ~ migration/src/lib.rs                                 modifié
  + src/modules/mod.rs                                   créé
  ~ src/lib.rs                                           modifié
  ~ Cargo.toml                                           modifié
  ~ AGENTS.md                                            modifié

  10 fichiers à écrire
✓ audit installée — 5 fichiers

  rbs migrate up, puis appelez audit::record dans vos services — l'entrée s'écrit dans la transaction du changement
```

The migration comes with it, so [`rbs migrate up`](../cli/migrate.md) is the next command:
until the `audit_log` table exists, the first `record` fails on a missing relation.

## What it is, and what it is not

A generated project already knows *that* a request changed a row: `trace.rs` logs the
method, the path, the status and the duration of every request that goes through. What it
does not know is *what* changed, or who changed it, past the retention of your logs.

This is not a replacement for that trace, and it is not a middleware. A layer logging every
mutating request would cost you nothing to install — and it could never say what changed:
it sees a request body, never the before and after of a row. It would also duplicate
`trace.rs`, which already logs every request.

So the fragment gives you a table, an entry and a function, and your service decides what
deserves a trace.

## The write and its trace are inseparable

`record` takes a `&C: ConnectionTrait` rather than a `DatabaseConnection`, and that is the
whole reason for putting the log in the database rather than in a file. A transaction *is*
a `ConnectionTrait`: hand it the one carrying your change, and the trace exists if and only
if the change is committed.

The repository `rbs generate` writes takes a `&DatabaseConnection`. Widen the signature of
the write you want to trace, so that the service can hand it the transaction instead — it
is the one change the fragment asks of your existing code, and
[`examples/event-hub`](https://github.com/tky0065/rbs/tree/main/examples/event-hub) widens
the signature of a creation the same way:

```rust file=examples/event-hub/src/orders/repository.rs region=create
```

The service then holds the transaction, and decides what deserves a trace:

```rust file=examples/event-hub/src/orders/service.rs region=create
```

The same transaction also carries `webhooks::emit(&transaction, "order.created", &order)` —
see the [webhooks guide](./webhooks.md) for the delivery that shares it.

A log that keeps the trace of a rolled-back `UPDATE` lies. A log that misses the trace of a
committed one lies too. The transaction settles both at once, and the test that proves it
ships with the fragment:
`an_entry_written_in_a_rolled_back_transaction_does_not_exist`.

It is the same contract as [`jobs::enqueue`](./jobs.md), for the same reason.

## The actor

`Entry::new` takes the three fields without which a log line means nothing — the action,
the entity, and the identifier of the row. `actor` and `changes` are chained on: what the
caller has no choice to make, the caller does not have to write.

Under [`auth`](./auth.md), the actor is one line in your handler — passed down to the
service, which builds the `Entry`:

```rust file=examples/event-hub/src/orders/controller.rs region=create
```

Without `auth`, leave it out. `actor_id` is nullable, and `Entry::actor` takes a `String`
rather than the `Identity` type, which only exists under `rbs-core`'s `auth` feature. Two
consequences, both deliberate: the fragment installs on an internal service with no JWT at
all, and **writes made outside a request stay traceable**. A cleanup job, a seed, an admin
command have no HTTP identity, and a log that demanded an actor would make them invisible —
precisely the writes you are trying to explain after the fact.

A missing actor is stored as `NULL`, never as an empty string. The distinction carries: an
empty string would say "an anonymous actor", `NULL` says "no HTTP identity".

## `action` and `changes` are open

`action` is a `String`, not an enum. Three constants cover the usual case:

```rust file=examples/event-hub/src/modules/audit/mod.rs region=actions
```

Anything else is a legitimate action — `login`, `export`, `impersonate` — and a closed enum
would only force you to work around it. `jobs::Status` *is* an enum, because that set is
closed; this one is not.

`changes` is a `serde_json::Value`, and the fragment imposes no schema on it. The example
above writes the values a creation produces; an `UPDATE` would write a before/after per
field, but a list of touched columns, a diff, or `Value::Null` are all valid. `entity_id`
is `TEXT` rather than `UUID` for the same reason: the generator lays down UUIDv7 keys, but
a hand-written entity may carry an integer or a composite key, and the log has to be able
to cite it.

## The table

| Column | Type | Note |
|---|---|---|
| `id` | `uuid` PK | UUIDv7, laid down by `ActiveModelBehavior::new` as everywhere else |
| `actor_id` | `text` null | The author, or nothing for a system write |
| `action` | `text` | `create`, `update`, `delete`, or whatever your project decides |
| `entity` | `text` | The name of the table concerned |
| `entity_id` | `text` | The key of the row concerned, as it is written |
| `changes` | `json` | What changed. Free form |
| `created_at` | `timestamptz` | Defaults to `current_timestamp` |

Two indexes: `(entity, entity_id)`, the one that reads the history of a row, and
`created_at`, the one that reads the history of a day. Without the first, the cost of a
read grows with the whole log — and a log is meant to grow.

There is no `updated_at`: a log line is not modified.

Reading a row's history is the query the shipped test replays:

```rust file=examples/event-hub/src/modules/audit/tests.rs region=history
```

## What the fragment does not do

It wires nothing into the CRUD `rbs generate` produces. No handler calls `record` for you,
and installing the feature changes the behaviour of not one existing route.

That is a choice, not an omission. Which writes deserve a trace is a question only your
domain answers: a `PATCH` on a draft and a `DELETE` on an invoice do not carry the same
weight, and a fragment that logged both would either flood the table or force you to
unpick its wiring. The service that owns the change is the only place that knows, and it is
also the only place holding the transaction the trace has to join.

It also mounts no route to *read* the log. What you expose of it, and to whom, is a
decision with its own access rules — the query above is all you need to build it.
