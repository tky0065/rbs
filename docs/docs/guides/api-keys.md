---
sidebar_position: 11.8
title: API keys
---

# Machine authentication with API keys

`rbs add api-keys` gives a project credentials that machines can hold: a table, four routes
to administer them, and one more header that `Identity` accepts — so that **everything a
token already opens, a key opens too**, without a single generated controller changing.

That property is the whole point of the fragment, and it is also its sharpest edge. Read
[What a key is worth](#what-a-key-is-worth) before you install it.

It requires `auth`, which in turn pulls `mail` and `rate-limit`: a key belongs to an
account, carries a role, and both come from there. On a bare project all four come down in
a single plan — here is an excerpt of it:

{/* rbs:transcript cmd="rbs add api-keys" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" extrait="oui" */}
```text
$ rbs add api-keys
api-keys : clés d'API : authentification machine, rôle plafonné par le porteur, trace d'usage
api-keys exige mail, rate-limit, auth : posée avec elle

plan pour …/demo

  + src/modules/mail/mod.rs                                créé
  + src/modules/rate_limit/mod.rs                          créé
  + src/auth/mod.rs                                        créé
  + src/modules/api_keys/mod.rs                            créé
  + src/modules/api_keys/model.rs                          créé
  + src/modules/api_keys/repository.rs                     créé
  + src/modules/api_keys/service.rs                        créé
  + src/modules/api_keys/dto.rs                            créé
  + src/modules/api_keys/controller.rs                     créé
  + src/modules/api_keys/tests/mod.rs                      créé
  + src/modules/api_keys/tests/accept.rs                   créé
  + src/modules/api_keys/tests/routes.rs                   créé
  + migration/src/m20260918_144456_create_api_keys.rs      créé
  ~ AGENTS.md                                              modifié

  62 à créer, 13 à modifier
✓ api-keys installée — 62 créés, 13 modifiés

  rbs migrate up, puis POST /api-keys pour tirer une clé — elle n'est rendue qu'à cet instant — et présentez-la en X-Api-Key
```

Two migrations come with it, so [`rbs migrate up`](../cli/migrate.md) is the next command.

## What a key is worth

A key is a second credential that `Identity` accepts. Present it in `X-Api-Key`, and the
extractor builds the same `Identity` a bearer token would have built:

```http
GET /articles
X-Api-Key: rbs_kJ3…
```

**A token wins when both are presented.** It is the more specific credential, and a proxy
injecting a service key must not override the one the caller offered.

The consequence is deliberate and worth stating plainly: a key opens **every** route that
`Identity` guards — including `/auth/change-password` and `/auth/sessions`. Nothing in the
generated code distinguishes a request made with a key from one made with a token, because
that is exactly what makes the fragment useful: a CRUD you generated six months ago accepts
a key the day you install this, with nothing to rewrite.

If that is more than you want, the cap below is how you bound it.

## The cap

Every key carries its own role, and the role it is **served** is the lesser of the key's and
its bearer's, recomputed on every request:

- at creation, a role above the creator's is refused with `403` — nobody delegates more than
  they hold;
- at each request, the account is read back, so a key stops administering the day its bearer
  is demoted. You do not have to remember to revoke it.

This is what buys the read-only key: an administrator mints a `user` key for a script, and
that script can never write on a route the CRUD generator closed with `--role admin`.

The ordering comes from the `Role` enum in `src/auth/model.rs` — the same one `require_role`
reads. A role you add there becomes usable by keys with no migration.

## Minting, listing, revoking

Four routes, modelled on `/auth/sessions`, which administers the same kind of object.

| Route | Answers |
|---|---|
| `POST /api-keys` | `201` with the key in clear — **this once only** |
| `GET /api-keys` | the caller's own keys, never the key, never its fingerprint |
| `DELETE /api-keys/{id}` | `204`, or `404` if it is not the caller's |
| `DELETE /api-keys` | `204`, all of the caller's keys |

```http
POST /api-keys
{ "name": "ci", "role": "user", "expires_in_days": 90 }

201 Created
Cache-Control: no-store
{ "id": "…", "name": "ci", "prefix": "rbs_kJ3aB7c", "role": "user",
  "expires_at": "2026-12-17T…", "key": "rbs_kJ3aB7c…" }
```

The clear key leaves the process once. Afterwards only `prefix` comes back, which is enough
to recognise a key in a list and never enough to present it. The response carries
`Cache-Control: no-store` on the **type**, not on the handler, so a second handler returning
that body cannot forget it.

Revoking someone else's key answers `404`, not `403`: a `403` would confirm that the key
exists.

**A key may mint a key.** The cap forbids escalation, and forbidding it outright would break
the automated provisioning that is the fragment's reason to exist.

## Revoking everything

`DELETE /auth/sessions` closes the bearer's sessions. **It does not touch their keys**, and
`DELETE /api-keys` does not touch their sessions. Two surfaces, two gestures.

That is a decision, not an oversight: a human clicking "sign out everywhere" from an app
does not mean "stop the CI". If you want both, call both.

## The usage trace

`last_used_at` answers the only question anyone asks of an unused credential: does this key
still serve? A key nobody dares revoke because nobody knows is a key that lives forever.

The write is bounded to **one per minute per key**, decided in memory before any round trip —
the row has just been read, so its previous trace is already in hand. The update is detached:
a missed trace is better than a refused call.

## The table

| Column | Role |
|---|---|
| `user_id` | the bearer; a key never outranks the account |
| `name` | what it serves — "ci", "billing script" |
| `prefix` | the first twelve characters, to recognise it in a list |
| `token_hash` | SHA-256 of the whole key, uniquely indexed — the read path of every machine request |
| `role` | the key's own role, capped on read |
| `last_used_at`, `expires_at`, `revoked_at` | the trace, the optional deadline, the dated revocation |

The key itself is never stored. A base read by a third party yields no usable credential —
the same rule the one-time tokens of `auth` follow.

Expiry and revocation live **inside** the lookup condition, not in a check that would follow
it: a key revoked between the read and the check must not open the request in flight.

## What it leaves to you

- **The generated TypeScript client cannot present a key.** `rbs generate client` marks an
  operation as protected as soon as its `security` is non-empty, but it only emits a `Bearer`
  holder. A machine integration written against that client still needs a token, or a hand-
  written header.
- **Choosing who may mint.** The four routes require no role: everyone administers their own
  keys, as everyone administers their own sessions. If your project wants minting reserved to
  administrators, add `identite.require_role(Role::Admin)?` to `controller::create`.
- **A key's deadline does not bound what that key issues.** The cap applies to the role, at
  every request; nothing caps time. A key created with `expires_in_days: 1` may call `POST
  /api-keys` and receive a key of the same role with no deadline at all — an access that
  outlives the expiry meant to close it. The design allows a key to mint a key, because
  provisioning depends on it, and the role cap forbids escalation; the deadline was never
  arbitrated. Until it is, treat a leaked key as a leaked account: `GET /api-keys` shows what
  it has minted and `DELETE /api-keys` closes all of them at once. To make the deadline
  propagate, cap the child's `expires_at` by the caller's in `service::create` — `Claims.jti`
  already tells you the request came from a key rather than a session.

## Testing

The fragment's tests ship under `src/modules/api_keys/tests/`. They join the
database your `.env` describes, so they carry `#[ignore]`: `cargo test -- --ignored` runs
them once `rbs migrate up` has been applied.

They are what proves the fragment works — the cap recomputed when a bearer is demoted, an
unknown, revoked and expired key answering alike, the trace written once across close calls,
and a key opening a route guarded by `Identity`.
