---
sidebar_position: 4
title: OpenAPI
---

# OpenAPI

A generated project documents itself. `rbs new` writes a `src/openapi.rs` that declares
the document, `rbs generate crud` adds the new handlers to it, and two settings decide
what is served over HTTP.

## The document

One `#[derive(OpenApi)]`, one list of operations, one modifier:

```rust file=examples/hello-crud/src/openapi.rs region=document
```

The `// <rbs:openapi>` anchor is where `rbs generate crud` writes. The CLI never rewrites
an AST — it inserts between comment markers. Delete the anchor and the CLI writes
nothing: it prints the block for you to paste instead of guessing where it belongs.

Adding an operation by hand is the same gesture: annotate the handler with
`#[utoipa::path(...)]`, then name it inside `paths(...)`.

## `CommonResponses`, declared once

`modifiers(&CommonResponses)` is the whole of the error documentation. It runs over the
finished document and does three things:

- registers the `ProblemDetails` schema — the very type that produces error bodies at
  runtime, so the schema and the response cannot drift apart;
- adds a **422** and a **500** to every operation that does not already declare one.
  Those two are the only responses *any* operation can produce: the runtime validates
  everywhere and can fail everywhere;
- registers `BadRequest`, `Unauthorized`, `Forbidden`, `NotFound` and `Conflict` under
  `components/responses`, ready to be referenced by name from a handler that can actually
  return them.

An operation that documents its own 422 keeps it. A handler knows more about its own case
than the runtime does, and its description is not overwritten.

The alternative would be repeating the same five responses on every handler in the
project, which is exactly the kind of thing that stops being true after the third
feature.

## The two URLs, and the two switches

```rust file=examples/hello-crud/src/openapi.rs region=exposition
```

| URL | What it serves |
|---|---|
| `/docs` | Swagger UI |
| `/api-docs/openapi.json` | the document itself |

`docs.swagger_ui` and `docs.openapi_json` both default to `true` — documentation should
exist from the moment the project is generated; turning it off is a production decision,
not the starting state. See the [configuration guide](./configuration.md) for where to
write them.

One asymmetry is worth knowing, and it is visible in the code above: Swagger UI loads the
document over HTTP and mounts that route itself. Displaying the interface therefore
implies exposing the document, and mounting it a second time would make Axum panic at
startup. **To serve the document alone, turn `docs.swagger_ui` off** — that combination
is the one that generates clients or checks a contract from CI. The reverse combination
does not exist, and asking for it changes nothing.

## Judge for yourself

Start the project and fetch the document:

```bash
curl -s localhost:8080/api-docs/openapi.json | jq '.components.responses | keys'
```

Or read the tests, which assert on a rendered document rather than on the code that
builds it:

```bash
cargo test -p rbs-core openapi::tests
```
