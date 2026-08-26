---
sidebar_position: 3
title: Errors
---

# Errors

Every layer of a feature — repository, service, controller — returns
`rbs_core::Result<T>`, which is `Result<T, rbs_core::Error>`. You choose a variant, you
return it with `?`, and the response is written for you: the right status, an
`application/problem+json` body, and the request id of the line it left in the log.

## The variants and their status

| Variant | Status | `title` | Body |
|---|---|---|---|
| `NotFound(&'static str)` | 404 | `Not Found` | `detail` names the resource |
| `BadRequest(String)` | 400 | `Bad Request` | `detail` carries the cause |
| `Validation(ValidationErrors)` | 422 | `Validation failed` | `errors`, field by field |
| `Unauthorized` | 401 | `Unauthorized` | — |
| `Forbidden` | 403 | `Forbidden` | — |
| `Conflict(String)` | 409 | `Conflict` | `detail` carries the message |
| `Domain { status, code, message }` | yours | the `code` | `detail` carries the message |
| `Database(DbErr)` | 500 | `Internal Server Error` | a fixed sentence |
| `Internal(anyhow::Error)` | 500 | `Internal Server Error` | a fixed sentence |

Three of them are reached without ever being named: `DbErr`, `anyhow::Error` and
`ValidationErrors` all have a `From` impl, so `?` converts them on the way out.
`Domain` is the escape hatch — a business error that picks its own status and a stable
code — and it exists so that a generated project does not stack its own error hierarchy
on top of this one.

`BadRequest` and `Validation` split a boundary that is worth stating: 400 means *I could
not read your body*, 422 means *I read it, and it breaks a rule*.

## The body

Responses follow RFC 9457, with the `application/problem+json` content type. A validation
failure looks like this:

```json
{
  "type": "about:blank",
  "title": "Validation failed",
  "status": 422,
  "errors": {
    "email": ["adresse électronique invalide"]
  },
  "request_id": "01JQ3F8K2P"
}
```

Absent fields are not serialized: `detail`, `errors` and `request_id` disappear when
there is nothing to put in them. `request_id` is filled from the middleware mounted by
the generated router, which is also what stamps the log lines — so a client holding an id
gives you the exact line to look at.

`Database` and `Internal` are the two variants that say nothing. Their source is written
to the server log at `ERROR` level and stops there; the client gets `"une erreur interne
est survenue"` and the request id. That is deliberate: a connection string, a host, a
missing secret are all things an error message will happily hand to whoever asked. Two
tests exist for the sole purpose of failing if a source ever leaks into the body.

## How an error becomes a response

Nothing is wired by hand. `Error` implements Axum's `IntoResponse`, so a handler that
returns `rbs_core::Result<T>` is already a valid Axum handler. The two middlewares that
complete the picture are mounted once, on the router:

```rust file=examples/hello-crud/src/router.rs region=montage
```

Some errors never reach your code at all. `ValidatedJson<T>` deserializes *then*
validates, and turns both failures into the right variant before your controller runs —
a malformed body into `BadRequest`, a body that breaks a `validator` rule into
`Validation`. `Pagination` does the same with an unreadable `?page=` or `?per_page=`.
Out-of-range page sizes, on the other hand, are silently clamped: a bound is not a
mistake the client needs to be told about, but `per_page=abc` is.

The generated tests check both ends of that boundary. An unknown identifier:

```rust file=examples/hello-crud/src/articles/tests.rs region=erreur_404
```

And a body that cannot be parsed at all:

```rust file=examples/hello-crud/src/articles/tests.rs region=corps_illisible
```

## Judge for yourself

Every variant has a test asserting its status, its body, and — for the two internal ones —
what its body must *not* contain:

```bash
cargo test -p rbs-core error::tests
```
