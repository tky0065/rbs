---
sidebar_position: 6
title: Not computing twice
---

# Not computing twice

This is the sixth tutorial. It picks up `demo` right after [Taking a
file](./storage.md) — running, with the `uploads` resource from that page. The case: a
`COUNT(*)` read a thousand times a minute, that three writes make stale the moment it
changes.

## What you need

Nothing beyond [Taking a file](./storage.md): the same running `demo`, with `uploads`
mounted.

## 1. Install the feature

```bash
rbs add redis
```

{/* rbs:transcript cmd="rbs add redis" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add redis
redis : cache Redis : pool paresseux partagé par l'état, valeurs typées par serde

plan pour …/demo

  + src/modules/cache/mod.rs      créé
  + src/modules/cache/config.rs   créé
  + src/modules/cache/tests.rs    créé
  + src/modules/mod.rs            créé
  ~ src/lib.rs                    modifié
  ~ src/state.rs                  modifié
  ~ src/health/controller.rs      modifié
  ~ docker-compose.yml            modifié
  ~ Cargo.toml                    modifié
  ~ config/default.toml           modifié
  ~ AGENTS.md                     modifié

  4 à créer, 7 à modifier
✓ redis installée — 4 créés, 7 modifiés

  le compose du projet porte déjà un service redis — docker compose up -d le démarre ; sans compose, faites écouter un Redis à l'URL de [cache] de config/default.toml
```

The name is a trap worth naming: the command is `redis`, and everything it writes calls
itself `cache` — `src/modules/cache/`, `mod cache;`, `state.cache()`. Only the Cargo
dependency and this command keep the name `redis`; your own code never writes it. Like
`storage` and `mail` before it, the brick mounts no route: what you get is a `Cache` on
`AppState`, deciding what to store in it left entirely to you. `add storage` and `add
mail` are not run again here;
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop)
carries both alongside `redis`, wired into the one service this page reads from below.

## Verify

`generate crud` has no reason to know a cache brick exists, so nothing wires `Cache`
into `uploads`' service for you — the calls below are `file-drop` code, read further
down this page. What `cargo test` can check without a live Redis is the part with no
network in it: does a prefix invalidation remove exactly the keys under that prefix, and
nothing that merely starts the same way?

```bash
cargo test modules::cache::
```

{/* rbs:libre raison="cargo test compile le projet entier, plusieurs minutes, et l'ordre de ses lignes suit l'ordonnanceur des threads" */}
```text
running 7 tests
test modules::cache::tests::a_missing_key_returns_none_and_not_an_error ... ok
test modules::cache::tests::a_prefix_carrying_a_glob_metacharacter_is_escaped ... ok
test modules::cache::tests::invalidate_prefix_only_removes_the_keys_of_the_targeted_prefix ... ok
test modules::cache::tests::a_cached_value_reads_back_deserialized ... ok
test modules::cache::tests::a_prefix_with_a_metacharacter_only_removes_what_it_designates ... ignored, joint le Redis de la section [cache]
test modules::cache::tests::a_value_with_a_one_second_ttl_is_gone_after_the_wait ... ignored, joint le Redis de la section [cache]
test modules::cache::tests::the_full_run_plays_against_a_server ... ignored, joint le Redis de la section [cache]

test result: ok. 4 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Proof of the one property no server is needed for:
`invalidate_prefix_only_removes_the_keys_of_the_targeted_prefix` hands the function four
keys — two matching the prefix, two that merely resemble it — and checks that only the
two under it are chosen for removal. The three ignored tests are the ones that actually
talk to Redis; `docker compose up -d` brings the project's own instance up, and `cargo
test -- --ignored` is what reaches them.

## What was installed

Three files, read from
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — the
one `add redis` command above, run on a project compiled in CI.

:::note
`file-drop` carries all three v0.3 bricks — `storage`, `mail` and `redis` — on one
project, so its `src/uploads/service.rs` also deposits content and sends mail, neither
of which this page installed. The `list` extract below is the part of that file the
cache alone accounts for.
:::

### Reading and writing

```rust file=examples/file-drop/src/modules/cache/mod.rs region=lecture
```

A missing or expired key answers `Ok(None)`, not an error — the ordinary state of a
cache is *not having* the value, and a caller falls through to its source of truth
without inspecting an error kind.

### Invalidating by prefix

```rust file=examples/file-drop/src/modules/cache/mod.rs region=invalidate_prefix
```

`SCAN` rather than `KEYS`, so the server is never blocked walking its whole keyspace to
answer one invalidation.

### What the example caches

`file-drop` caches the **total**, not the page:

```rust file=examples/file-drop/src/uploads/service.rs region=list
```

The choice is the point of this page. `COUNT(*)` walks the entire table on every call,
while the page itself reads only `per_page` rows — the expensive half is the one worth
caching. And `Page` is `Serialize` but not `Deserialize`: it renders, but reading one
back from the cache would mean making it deserialisable in `rbs-core`, which is a cost
the core does not carry for a choice one project made. Every write undoes the count it
invalidates — `create`, `update` and `delete` all call `invalidate_prefix`, so a stale
total never outlives the write that made it stale.

## Going further

- [Cache](../guides/cache.md) covers construction, why it stays synchronous, and what
  the feature leaves to you — stampede protection among it.
- [`rbs add`](../cli/add.md) covers the other features `demo` could still install,
  `storage` and `redis` now both on it.
- [Testing](../guides/testing.md) is the harness the generated `cache/tests.rs` splits
  against — tests with no server, and tests that need one.
- [Moving long work out of the request](./jobs.md) is the next tutorial: a campaign of
  5,000 letters, enqueued without making the caller wait for any of them to send.
