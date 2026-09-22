---
sidebar_position: 9
title: Calling the API from TypeScript
---

# Calling the API from TypeScript

This is the last tutorial. It picks up `demo` right after [Your first
resource](./first-resource.md) — running, with the `articles` CRUD from that page and its
migration applied — and reads it into a typed client instead of a server response typed
by hand on the front end. The case: a browser or a Node script that calls `articles`,
`PATCH` and all, without a single interface written twice.

## What you need

Nothing beyond [Your first resource](./first-resource.md): the same `demo`, with the
`articles` CRUD generated on that page. No server needs to be running for this one —
`generate client` compiles the project rather than calls it.

## 1. Generate the client

```bash
rbs generate client --lang ts
```

{/* rbs:transcript cmd="rbs generate client --lang ts" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs generate crud articles --fields title:string,body:text,published:bool" dans="demo" */}
```text
$ rbs generate client --lang ts
plan pour …/demo

  + clients/ts/client.ts   créé

  1 à créer
✓ client engendré — clients/ts/client.ts porte 8 opérations
```

Nothing here reads `src/articles/` directly, and nothing guesses a route's shape from its
handler: the command runs `src/bin/openapi.rs` — the third binary `rbs new` wrote
alongside `demo` itself and the seed runner — and reads what `ApiDoc::openapi()` prints on
its standard output. That is what makes the command work with no server listening and no
database reachable: the document is a build artifact, not a network response. Eight
operations is `articles`'s five routes, `POST /articles/filter`, `GET /health` and
`GET /health/live` — every handler that carries an `operationId`, which every one `rbs generate crud` writes does by
default.

The point that carries the rest of this page: the client is read off a document the
project already exposes, so it follows the server rather than a second, hand-maintained
copy of its types. Change a field with another `rbs generate crud`, or edit a handler's
signature by hand, and the very next `rbs generate client --lang ts` is what catches the
drift — not a runtime error a caller reports days later. Regenerating after every contract
change is the loop this command is built for, not an extra step bolted onto it.

## Verify

The file just written is its own proof, and reading it needs nothing beyond `rbs`
itself: run the exact same command again.

```bash
rbs generate client --lang ts
```

{/* rbs:transcript cmd="rbs generate client --lang ts" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs generate crud articles --fields title:string,body:text,published:bool && rbs generate client --lang ts" dans="demo" */}
```text
$ rbs generate client --lang ts
plan pour …/demo

  · clients/ts/client.ts   inchangé

  1 inchangé
✓ client engendré — clients/ts/client.ts porte 8 opérations
```

`inchangé` is proof by idempotence: reading the same OpenAPI document a second time
produces the same file byte for byte, so nothing was left for the second run to write.
The operation count printed again is the same eight — not recomputed from the file on
disk, but read fresh from `ApiDoc::openapi()` each time, which is what makes this
command safe to run after every `rbs generate crud` rather than only once.

## What was installed

Two extracts, read from
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
the same command shown above, run on the exact CRUD [Your first
resource](./first-resource.md) generated.

### The client

`ApiClient` is a configurable class rather than a set of free functions: the base URL and
the headers are set once, at construction, instead of threaded through every call.

```typescript file=examples/hello-crud/clients/ts/client.ts region=classe
```

### The operations

One method per operation, named after its `operationId` in camelCase. `articlesFilter`
posts rather than gets, because the conditions it carries would not fit in a URL; `health`
and `healthLive` return `Promise<void>`, since neither health route declares a body to
parse.

```typescript file=examples/hello-crud/clients/ts/client.ts region=methodes
```

## Going further

- [`rbs generate client`](../cli/client.md) covers the flags this page didn't use —
  `--out`, `--force`, `--dry-run` — and what the command does when it finds a client
  you've since edited by hand.
- [OpenAPI](../guides/openapi.md) covers the document this page's client was read from,
  and what makes a handler contribute an `operationId` in the first place.
- [`rbs generate`](../cli/generate.md) covers the full grammar of `--fields`, for a
  resource richer than the one this series built one command at a time.
