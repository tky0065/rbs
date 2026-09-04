---
sidebar_position: 11.7
title: Webhooks
---

# Outgoing webhooks

`rbs add webhooks` gives a project a way to tell the outside world what just happened: ten
files under `src/webhooks/`, a migration for the `webhook_subscriptions` table, three
routes, and a signed HTTP POST for every subscriber that listens.

**The fragment delivers; it does not decide what is worth telling.** Nothing is emitted
until your own code calls `webhooks::emit`. Installed and never called, the feature has no
observable effect — which is deliberate: which of your writes deserve to leave the process
is a question only your domain answers.

It requires `jobs`, because the queue already knows how to deliver-with-retries — reserving
a row without double-dequeuing, `attempts`, `available_at`, `last_error` — and a second
retry mechanism would have left two loops to maintain instead of one. It requires `auth`
too, which in turn pulls `rate-limit`: a subscription endpoint left open would let anyone
have the project's events delivered to their own server, and `user.created` carries
addresses. On a bare project all four go down in a single plan:

```text
$ rbs add webhooks
webhooks : webhooks sortants : abonnements, signature HMAC horodatée, livraison par la file
webhooks exige auth, jobs, rate-limit : posée avec elle

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                                  créé
  …
  + src/jobs/mod.rs                                                  créé
  …
  + src/rate_limit/mod.rs                                            créé
  …
  + src/webhooks/mod.rs                                              créé
  + src/webhooks/config.rs                                           créé
  + src/webhooks/model.rs                                            créé
  + src/webhooks/dto.rs                                              créé
  + src/webhooks/repository.rs                                       créé
  + src/webhooks/service.rs                                          créé
  + src/webhooks/controller.rs                                       créé
  + src/webhooks/signature.rs                                        créé
  + src/webhooks/delivery.rs                                         créé
  + src/webhooks/tests.rs                                            créé
  + migration/src/m20260904_160207_create_webhook_subscriptions.rs   créé
  ~ AGENTS.md                                                        modifié

  43 fichiers à écrire
✓ webhooks installée — 32 fichiers

  rbs migrate up, inscrivez un abonné par POST /webhooks/subscriptions — son secret n'est rendu qu'à cet instant — puis appelez webhooks::emit dans vos services
```

Three migrations come with it, so [`rbs migrate up`](../cli/migrate.md) is the next command.

## Emitting an event

```rust
webhooks::emit(&transaction, "user.created", &dto).await?;
```

`emit` takes a `&C: ConnectionTrait` rather than a connection, and that is the whole point:
**hand it the transaction carrying your change, and the deliveries exist if and only if
that change is committed.** A `user.created` delivered for a signup that was rolled back is
a lie no retry can take back.

It returns how many deliveries were enqueued. Nobody listening is not an error — a project
with no webhook configured emits into the void, which is the nominal case.

The payload is anything that is `Serialize`, and in practice it is the DTO you already
return from the API. What it must *not* be is your entity: a webhook body is a public
contract, and coupling it to the table means every column you rename becomes a breaking
change for every subscriber.

## Subscribing

Three routes, all requiring a valid token:

| Route | Effect |
|---|---|
| `POST /webhooks/subscriptions` | Registers a URL and the patterns it listens to. **Returns the secret — this once and never again.** |
| `GET /webhooks/subscriptions` | Lists every subscription, revoked ones included, without their secrets |
| `DELETE /webhooks/subscriptions/{id}` | Revokes one, by stamping `revoked_at` |

```json
{ "url": "https://example.test/hooks", "events": ["user.*"] }
```

**The secret belongs to the subscription, not to the project.** A shared secret would give
each subscriber what they need to forge the events delivered to all the others. It is
returned by the creation response alone: a single read of the list would otherwise hand out
everyone's secrets at once.

`Identity` only says "the token is valid". A project that reserves subscription
administration for a role replaces the extractor with its own guard — see
`src/auth/guard.rs` and the [auth guide](./auth.md).

## Event patterns

Three forms, and not one more:

| Pattern | Matches |
|---|---|
| `*` | every event |
| `user.*` | every event of the family |
| `user.created` | itself, and nothing else |

The sorting happens in Rust and not in SQL, which is deliberate: searching a JSON array has
no form common to PostgreSQL, MySQL and SQLite, and prefix patterns would have ruled it out
anyway. The price is one read of the subscriptions table per event emitted — without
consequence as long as subscribers number in the hundreds, and the thing to revisit if they
ever number in the millions.

## What the receiver reads

The body is the envelope, not your payload alone:

```json
{
  "id": "0192f3a0-…",
  "event": "user.created",
  "created_at": "2026-09-04T16:02:07+00:00",
  "data": { "…": "what you passed to emit" }
}
```

Three headers come with it:

| Header | Carries |
|---|---|
| `X-Rbs-Signature` | `t=<unix seconds>,v1=<hex HMAC-SHA256>` |
| `X-Rbs-Event` | the event name, readable without opening the body |
| `X-Rbs-Delivery` | the envelope's `id`, **stable across retries** |

The last one is how a receiver deduplicates. The queue can deliver twice — a response lost
after the work was done is enough — and without that identifier nothing would say so.

## Verifying the signature

The signed bytes are the timestamp, a dot, then the body verbatim:

```text
HMAC-SHA256(secret, "<timestamp>.<raw body>")
```

**The timestamp is inside the signature**, and that is what closes replay: a third party who
captures a delivery cannot re-serve it later under a fresh date without invalidating the
digest. A receiver should reject a timestamp too far from its own clock — five minutes is
the usual tolerance — and compare digests in constant time.

```js
const [t, v1] = header.split(',').map((part) => part.split('=')[1]);
const expected = crypto.createHmac('sha256', secret).update(`${t}.${rawBody}`).digest('hex');
const ok = crypto.timingSafeEqual(Buffer.from(v1), Buffer.from(expected));
```

`v1=` names the scheme rather than leaving the digest bare: the day a second one arrives,
both live in the same header and an up-to-date receiver picks.

Verify against the **raw body**, before any JSON parsing. Re-serializing a parsed object
can reorder keys, and the digest of a re-serialized body is not the digest that was signed.
The emitter has the same constraint and honours it: the envelope is serialized once, and
those bytes are what get signed and sent.

## Delivery, retries and duplicates

An emission enqueues one job per listening subscriber; the worker does the rest. Retries,
backoff, `attempts` and `last_error` are the queue's, unchanged — see the
[jobs guide](./jobs.md).

**The subscription is named by its id, not copied into the job.** URL and secret are re-read
at dequeue time, so a rotated secret applies to deliveries already queued, and a revocation
stops them. A delivery whose subscription has been revoked — or deleted — ends in success
without an HTTP request: there is nothing to deliver and nothing to retry, and returning an
error would spend five attempts on a row that no longer matters.

Any non-2xx response counts as a failure, **4xx included**. A receiver answering 400 to a
well-formed delivery is broken, and telling that apart from a 503 would mean guessing which
of the two parties is wrong.

The delivery is at-least-once, never exactly-once. That is not a shortcoming to be fixed
later: a receiver that answers after doing the work but before its response arrives must be
delivered to again, and `X-Rbs-Delivery` is what makes that harmless.

## The table

| Column | Type | Note |
|---|---|---|
| `id` | `uuid`, primary key | UUIDv7, laid down by the application |
| `url` | `varchar(191)` | Where the delivery is POSTed |
| `events` | `json` | The listened patterns |
| `secret` | `varchar(191)` | This subscription's signing secret |
| `revoked_at` | `timestamptz`, nullable | The revocation, dated. Null while the subscription serves |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

No further index, and that is not an oversight: emission reads every non-revoked
subscription to sort them itself, and an index on a column that is always read whole earns
nothing while costing every write.

## Configuration

```toml
[webhooks]
timeout_secs = 10
```

One setting: how long a receiver is given to answer. Past that, the delivery counts as a
failure and goes back for a retry — a slow endpoint must not hold a worker.
`config/{env}.toml` and `RBS_WEBHOOKS__TIMEOUT_SECS` override it like any other section —
see the [configuration guide](./configuration.md).

The HTTP client lives in `AppState` and is built once at boot: a `reqwest::Client` carries
its connection pool, and rebuilding it per delivery would reopen a TLS session per attempt.
An unreadable timeout therefore stops the boot rather than surfacing six hours later in a
worker's log.

## Testing

The generated `src/webhooks/tests.rs` covers the two halves separately. The signature is
proven against **a vector computed outside Rust**, so the test would survive a rewrite of
the signing code and catch a change of scheme; the pattern matching is proven on its three
forms.

The database half is what justifies the design:

- an emission enqueues one delivery per listening subscription, and none for a subscriber
  that does not listen;
- a revoked subscription is not delivered to;
- a delivery whose subscription was revoked between emission and dequeue **succeeds without
  an HTTP request**;
- **an emission rolled back with its transaction enqueues nothing** — the test that makes
  the `&C: ConnectionTrait` signature more than a stylistic choice.

## What it leaves to you

- **what is worth emitting** — `emit` is called by your code or by nobody;
- **the shape of your payloads** — no versioning of the envelope beyond `v1` on the
  signature, and no schema published to subscribers;
- **incoming webhooks** — the fragment signs what it sends and verifies nothing it
  receives;
- **circuit breaking** — an endpoint that has been dead for a week is retried like any
  other, and nothing disables a subscription that never answers;
- **a delivery log** — `last_error` on the job says why the last attempt failed, and once
  the job is gone, so is the trace. [`rbs add audit`](./audit.md) is the fragment for
  keeping a record.
