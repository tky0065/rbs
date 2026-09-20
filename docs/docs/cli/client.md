---
sidebar_position: 2.5
title: rbs generate client
---

# `rbs generate client`

Writes a typed TypeScript client from the project's own OpenAPI document. One method per
operation, one interface per schema, and no dependency to install on the TypeScript side.

```bash
rbs generate client --lang ts
```

The client lands in `clients/ts/client.ts` — or in `frontend/src/api/client.ts` on a
project carrying the [`frontend`](./add.md) fragment, where the base imports it and where
the dev server can reach it. Regenerate it after every contract change rather than editing
it: the command refuses to overwrite a file that has been touched, and `--force` lifts that
refusal.

## Where the document comes from

**No server runs.** `rbs new` writes a third binary, `src/bin/openapi.rs`, which prints what
`ApiDoc::openapi()` returns; `generate client` runs `cargo run --bin openapi` in the project
and reads its standard output.

The document is memorised under `target/rbs/`, keyed by a digest of the project's
sources: a second run on an unchanged project answers without compiling anything.
[`rbs openapi export`](./openapi.md#the-memorised-contract) describes what the digest
covers.

That is what makes the client follow the code rather than an approximate reading of the
sources: the document carries the routes your fragments mounted, the DTOs your `--fields`
produced, and the `operationId` of every handler — including the ones you wrote by hand.

The binary is useful on its own. Freezing the contract in CI is an
[`rbs openapi export --out openapi.json`](./openapi.md) followed by a `git diff` that must
stay empty.

## Flags

| Flag | Effect |
|---|---|
| `--lang <LANGAGE>` | **Required.** `ts` is the only value today. No default: the day a second language arrives, no existing invocation changes meaning. |
| `--out <DIR>` | Output directory, relative to the project root, in place of the default the project dictates. The file name does not change — it is the name the client carries in an import. |
| `--from <FILE>` | Reads a contract already exported by [`rbs openapi export`](./openapi.md), relative to the directory the command runs from, instead of compiling the project. The client is then written with no Rust toolchain involved — what a CI job that commits its `openapi.json` needs, and the way out when the memorised contract is wrong. |
| `--force` | Writes even though the Git working tree is dirty, and overwrites a client reported as conflicting. |
| `--dry-run` | Prints the plan and stops. rbs writes nothing — but the project is still compiled, since that is how the document is read, unless `--from` supplies it. |
| `--json` | Prints the plan — or the error — as one JSON document on standard output, the client's full content included; the project's compilation stays on standard error. Independent of `--dry-run`. [The agents guide](../guides/agents.md#reading-a-plan-as-json) has the document and the error codes. |

## What the client looks like

A configurable class rather than free functions: the token is set once, at construction,
instead of being threaded through every call.

```ts file=examples/hello-crud/clients/ts/client.ts region=options
```

`headers` takes a function as well as an object, which is what makes a rotating token
workable — it is called on every request. `fetch` is injectable for the same reason a test
needs it.

```ts file=examples/hello-crud/clients/ts/client.ts region=classe
```

Then one method per operation, named after its `operationId` in camelCase:

```ts file=examples/hello-crud/clients/ts/client.ts region=methodes
```

Path parameters come first, then the body, then the query — and a query whose fields are all
optional gets a default, so `articlesList()` needs no argument.

## Errors

Any non-2xx response throws an `ApiError` carrying the status, the parsed body, and — when
the body is an RFC 9457 problem — a typed `problem`. `rbs-core` answers every error in that
shape, so `error.problem?.title` is the message your API actually sent.

```ts
try {
  await api.articlesCreate({ title: "", body: "…", published: false });
} catch (error) {
  if (error instanceof ApiError && error.status === 422) {
    console.error(error.problem?.errors);
  }
}
```

That example is written by hand: it shows how to *use* the client, and no file under
`examples/` calls it.

## Regenerating

The client is projected as a creation, so a second run on an unchanged contract reports
`· clients/ts/client.ts inchangé` and writes nothing. A client you edited comes back as a
conflict instead of being silently overwritten:

```text
  ! clients/ts/client.ts   conflit — relancer avec --force
```

This is the point at which to move your own code out of the generated file rather than to
reach for `--force`.

## Called after `rbs generate crud`

On a project that already carries a generated client, you do not have to: `rbs generate
crud` rewrites it right after writing the entity, from the contract and never from the
entity it has just produced — [ADR-0004](https://github.com/tky0065/rbs/blob/main/docs/adr/0004-une-seule-source-pour-le-client-engendre.md)
explains why the client has exactly one source. The generation therefore pays one
incremental rebuild, by construction: the module has just been added, so the memorised
contract is stale at the moment the command needs it.

It stays conditional. A project with no client yet keeps the old behaviour — the command
prints the `rbs generate client` line to run and invents no `frontend/src/api` nobody asked
for.

The rewrite is a full overwrite, not a conflict: a contract that has just gained a table
makes every existing client differ, so refusing would refuse every time. What protects a
client you edited is `generate crud`'s own guard — it does not run on a dirty working tree
without `--force`, so the previous version is a `git checkout` away.

## Two refusals

Both arrive **before** cargo is launched, and in the order in which they are fixed.

A project without `src/lib.rs` — one created before rbs 1.0 — is refused by name: `ApiDoc`
lives in the main binary there, where a second binary cannot reach it. Announcing the
missing binary first would send you to write a file that would not compile.

A project without `src/bin/openapi.rs` is refused with the block to paste — the file, and
the `[[bin]]` section that declares it. A project created by `rbs new` already carries both.

## What it leaves to you

- **the languages** — `ts` alone today;
- **the packaging** — the file is written, and nothing turns it into an npm package;
- **operations without an `operationId`** — the command refuses the whole document rather
  than emitting a partial client. Every handler rbs generates carries one; a handler you
  wrote by hand needs its own, as `examples/newsletter-queue` shows on `broadcast`.
