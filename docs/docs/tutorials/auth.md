---
sidebar_position: 3
title: Locking the API down
---

# Locking the API down

This is the third of nine tutorials. It picks up `demo` right after
[Your first resource](./first-resource.md) — running, with the `articles` CRUD from that
page — and closes it to everyone it doesn't know. The case: a blog where every visitor
may read a post, and only an administrator may write one. Rather than reopen `articles`,
this page generates a second resource, `posts`: what changes here is the protection, not
the resource, and a CRUD generated before the guard existed says nothing about the guard
itself.

## What you need

Nothing beyond [Setting up](./setup.md): the same running `demo`, and `curl` again, for
three requests instead of two.

## 1. Install the feature

```bash
rbs add auth
```

{/* rbs:transcript cmd="rbs add auth" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles
auth exige mail, rate-limit : posée avec elle

plan pour …/demo

  + src/auth/mod.rs                                        créé
  + src/auth/config.rs                                     créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository/mod.rs                             créé
  + src/auth/repository/user.rs                            créé
  + src/auth/repository/refresh_token.rs                   créé
  + src/auth/repository/one_time_token.rs                  créé
  + src/auth/service/mod.rs                                créé
  + src/auth/service/session.rs                            créé
  + src/auth/service/password.rs                           créé
  + src/auth/service/verification.rs                       créé
  + src/auth/controller/mod.rs                             créé
  + src/auth/controller/session.rs                         créé
  + src/auth/controller/password.rs                        créé
  + src/auth/controller/verification.rs                    créé
  + templates/mail/reinitialisation.html                   créé
  + templates/mail/verification.html                       créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests/mod.rs                                  créé
  + src/auth/tests/session.rs                              créé
  + src/auth/tests/password.rs                             créé
  + src/auth/tests/verification.rs                         créé
  + migration/src/m20260909_093150_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ src/state.rs                                           modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  ~ .env                                                   modifié
  + src/modules/mail/mod.rs                                créé
  + src/modules/mail/config.rs                             créé
  + src/modules/mail/template.rs                           créé
  + src/modules/mail/service.rs                            créé
  + src/modules/mail/tests.rs                              créé
  + templates/mail/bienvenue.html                          créé
  + src/modules/mod.rs                                     créé
  ~ docker-compose.yml                                     modifié
  + src/modules/rate_limit/mod.rs                          créé
  + src/modules/rate_limit/config.rs                       créé
  + src/modules/rate_limit/counter.rs                      créé
  + src/modules/rate_limit/tests.rs                        créé
  ~ AGENTS.md                                              modifié

  46 fichiers à écrire
✓ auth installée — 34 fichiers

  rbs migrate up
```

`add` refuses a dirty working tree, which is why the command above only runs on a
freshly committed project. `auth exige mail, rate-limit : posée avec elle` is proof the
feature does not arrive alone — a login endpoint with no rate limiting, and a password
reset with no way to send the email, are exactly the kind of gap a generator should not
leave for you to notice later, so the CLI installs all three together.

## 2. Apply the migration

```bash
git add -A && git commit -q -m "auth installée"
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.74s
     Running `target/debug/migration up`
✓ migrations appliquées
```

`generate` and `add` both require a clean tree, so the commit comes first — proof of
nothing by itself, but what makes the next two commands able to run without `--force`.
The migration that just applied is the one `auth` wrote: two new tables, for accounts and
refresh tokens.

## 3. Generate a protected resource

```bash
rbs generate crud posts --fields "title:string,body:text,published:bool" --role admin
```

```text
plan pour …/demo

  + src/posts/mod.rs                                 créé
  + src/posts/model.rs                               créé
  + src/posts/dto.rs                                 créé
  + src/posts/filter.rs                              créé
  + src/posts/repository.rs                          créé
  + src/posts/service.rs                             créé
  + src/posts/controller.rs                          créé
  + src/posts/tests.rs                               créé
  + src/seeds/posts.rs                               créé
  + migration/src/m20260909_093231_create_posts.rs   créé
  ~ src/lib.rs                                       modifié
  ~ src/router.rs                                    modifié
  ~ src/openapi.rs                                   modifié
  ~ migration/src/lib.rs                              modifié
  ~ src/seeds/main.rs                                modifié
  ~ Cargo.toml                                       modifié
  ~ AGENTS.md                                        modifié

  17 fichiers à écrire
✓ posts générée — 10 fichiers

  la migration m20260909_093231_create_posts reste à appliquer avant de lancer le projet
```

Here is the point of this page: on a project carrying `auth`, `generate crud` closes
**every** route it writes at the lowest threshold — `list`, `find`, `filter` included —
before `--role` is even read. What the flag does is narrow, not open: it raises the
three writes (`create`, `update`, `delete`) to the role named, and touches nothing else.
Drop `--role admin` from the command above and the plan would look the same, but reading
`/posts` would need a token too — the flag's whole job is to tell `create`, `update` and
`delete` apart from the rest, not to decide whether the feature is protected at all.

## 4. Apply the last migration and run the server

```bash
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.12s
     Running `target/debug/migration up`
✓ migrations appliquées
```

Only the `posts` table was pending this time — the first `migrate up` already applied
`auth`'s pair. Restart the server so the new routes are compiled in:

```bash
cargo run
```

```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

A clean start here is proof the binary now compiles both `auth` and the `--role
admin` guard on `posts` into one router — if either had been wired wrong, this line
would never have printed.

## Verify

From the second terminal, create an account and exchange it for a token:

```bash
curl -s -X POST http://127.0.0.1:8080/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com","password":"un-mot-de-passe-long"}'
```

```text
{"id":"01a08583-7b76-7a01-90a8-c526d25cb1e3","email":"alice@example.com","role":"user","created_at":"2026-09-09T09:33:01.713220Z"}
```

Registration always produces `"role":"user"` — proof that no route on this page can hand
out `admin` for the asking; the account you get here can read, but not write, `posts`.

```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com","password":"un-mot-de-passe-long"}' \
  | jq -r .access_token)
```

Three requests separate the two regimes the rest of this page described. No token at
all, on the route the flag raised:

```bash
curl -i -X POST http://127.0.0.1:8080/posts \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier post","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 401 Unauthorized
content-type: application/problem+json
x-request-id: 01M22R73CRW027SYBNBTYJYDBA
content-length: 100
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Unauthorized","status":401,"request_id":"01M22R73CRW027SYBNBTYJYDBA"}
```

The extractor refuses this one before any handler runs — proof that a missing token
never reaches `require_role` at all. Now the same write, with a real but insufficient
token:

```bash
curl -i -X POST http://127.0.0.1:8080/posts \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title":"Premier post","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 403 Forbidden
content-type: application/problem+json
x-request-id: 01M22R73CYV34S5K7MFAKGHTFY
content-length: 97
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Forbidden","status":403,"request_id":"01M22R73CYV34S5K7MFAKGHTFY"}
```

`401` became `403` — proof the caller is now identified, and refused by `require_role`
itself, inside the handler, rather than by the extractor. The same token reads:

```bash
curl -i http://127.0.0.1:8080/posts \
  -H "Authorization: Bearer $TOKEN"
```

```text
HTTP/1.1 200 OK
content-type: application/json
x-request-id: 01M22R73D3PS6VQHTRP0GSD8MD
content-length: 69
date: Wed, 09 Sep 2026 09:33:06 GMT

{"data":[],"meta":{"page":1,"per_page":20,"total":0,"total_pages":0}}
```

Proof that `--role admin` never touched this route: the same `user` token that was
forbidden on the write reads the empty list without complaint.

## What was installed

Three files, read from
[`examples/blog-auth`](https://github.com/tky0065/rbs/tree/main/examples/blog-auth) — the
same two commands, run in the same order, on a project compiled in CI.

### The generated write

`create` is what `--role admin` shaped: an `Identity` extracted before the handler runs,
and `require_role` called first inside it.

```rust file=examples/blog-auth/src/posts/controller.rs region=create
```

### The guard

`require_role` lives in your project, not in `rbs-core` — the core knows a caller has a
role, in the clear, but not what the roles are or which one a route needs. It compares a
threshold, not an equality, which is why an `Admin` token satisfies a route that only
asks for `User`.

```rust file=examples/blog-auth/src/auth/guard.rs region=require_role
```

### The test that tells the refusals apart

Two 401s and a 403 look alike from a status code alone. This test is the one that pins
down which is which — a `user` token forbidden on the write, and reading anyway.

```rust file=examples/blog-auth/src/posts/tests.rs region=refus
```

## Going further

- [Authentication](../guides/auth.md) covers the five routes `add auth` mounts, the
  token pair, and the `Role` enum this page only used at its default.
- [`rbs add`](../cli/add.md) covers the eleven other features this project could still
  install, and the `--force` this page never needed.
- [`rbs generate`](../cli/generate.md) has the full grammar of `--role`, including what
  it does under `--with-upload`, and [what to remove to reopen a
  route](../guides/auth.md#closed-by-default-at-generation-time).
- [Testing](../guides/testing.md) is the harness `posts/tests.rs` runs against, the same
  one this page's third excerpt extends by hand.
- [Taking a file](./storage.md) is the next tutorial: a client deposits a file, and
  `PUT /uploads/{id}/content` stores it.
