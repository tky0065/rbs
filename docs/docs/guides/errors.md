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

| Variant | Status | `title` en | `title` fr | Body |
|---|---|---|---|---|
| `NotFound(&'static str)` | 404 | `Not Found` | `Introuvable` | `detail` names the resource |
| `BadRequest(String)` | 400 | `Bad Request` | `Requête invalide` | `detail` carries the cause |
| `Validation(ValidationErrors)` | 422 | `Validation failed` | `Validation échouée` | `errors`, field by field |
| `Unauthorized` | 401 | `Unauthorized` | `Authentification requise` | — |
| `Forbidden` | 403 | `Forbidden` | `Accès interdit` | — |
| `Conflict(String)` | 409 | `Conflict` | `Conflit` | `detail` carries the message |
| `Domain { status, code, message }` | yours | the `code` | the `code` | `detail` carries the message |
| `Database(DbErr)` | 500 | `Internal Server Error` | `Erreur interne` | a fixed sentence |
| `Internal(anyhow::Error)` | 500 | `Internal Server Error` | `Erreur interne` | a fixed sentence |

`title` is the one column above that changes with [`[server] lang`](#the-language-of-the-body):
`Domain`'s is not — it is always the `code` you chose, whichever language the rest of the
body is in.

Three of them are reached without ever being named: `DbErr`, `anyhow::Error` and
`ValidationErrors` all have a `From` impl, so `?` converts them on the way out.
`Domain` is the escape hatch — a business error that picks its own status and a stable
code — and it exists so that a generated project does not stack its own error hierarchy
on top of this one.

`BadRequest` and `Validation` split a boundary that is worth stating: 400 means *I could
not read your body*, 422 means *I read it, and it breaks a rule*.

## The body

Responses follow RFC 9457, with the `application/problem+json` content type. A validation
failure looks like this, on a project with `[server] lang = "en"`:

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

`title` moved with `[server] lang` above; the field message under `errors` did not — it is
whichever string the `validator` rule on that DTO field was given, and that string is not
rbs's to translate (more on the distinction [below](#the-language-of-the-body)).

`Database` and `Internal` are the two variants that say nothing. Their source is written
to the server log at `ERROR` level and stops there; the client gets a fixed sentence —
`"an internal error occurred"` in English, `"une erreur interne est survenue"` in French,
see [below](#the-language-of-the-body) — and the request id. That is deliberate: a
connection string, a host, a missing secret are all things an error message will happily
hand to whoever asked. Two tests exist for the sole purpose of failing if a source ever
leaks into the body.

## The language of the body

Two settings named `lang`, at two different moments, decide two different things.

At *run time*, `[server] lang` — `fr` by default, or `en`; `RBS_SERVER__LANG` overrides
it — decides what `rbs-core` itself writes into the response: the `title` of every
`problem+json` body, the fixed `detail` of the 404 (`{resource} not found` /
`{resource} introuvable`) and of the 500, and the common response descriptions of the
OpenAPI document — the six named responses under `components/responses` and the 422/500
added to every operation. That is `Error::parts` in `crates/rbs-core/src/error.rs`,
`crates/rbs-core/src/openapi.rs`, and the resolution in `crates/rbs-core/src/lang.rs`:

```toml
[server]
lang = "en"
```

At *generation time*, the project language — chosen once by `rbs new --lang` and recorded
as `lang` under `[package.metadata.rbs]` in `Cargo.toml` — decides the language in which
`rbs add` and `rbs generate crud` write the messages they hand to the client. Those become
plain string literals in your generated code, and `[server] lang` does not touch them
afterwards: the `lang` context read in `crates/rbs-cli/src/add/mod.rs` and the
`.speaking(metadonnees.lang)` call in `crates/rbs-cli/src/generate/command.rs` pick one of
two literals at render time, once and for all.

Those generated messages, file by file:

| File | Message |
|---|---|
| `src/auth/repository/user.rs` | `ADRESSE_PRISE`, the 409 of a duplicate registration |
| `src/modules/rate_limit/mod.rs` | the 429 message |
| `src/modules/webhooks/service.rs` | the 400 of an empty event pattern |
| `src/modules/webhooks/target.rs` | the three URL refusals, 400 |
| `src/modules/webhooks/repository.rs` | `NotFound("subscription")` / `"abonnement"` |
| a generated CRUD's `repository.rs` | the 409 of a duplicate unique value |
| a generated CRUD's `filter.rs` | the 400 of an unknown sort column |
| a generated CRUD's `controller.rs` and `service.rs`, under `--with-upload` | `NotFound("content")` / `"contenu"` |

What does not follow either setting:

- `Domain`'s own `title` — the `code` you chose, not a word rbs picked for you;
- the codes a `validator` rule reports in `errors` (`email`, `length`, …) — no generated
  DTO gives one a `message`, so these stay the bare codes, meant to be matched on rather
  than read;
- the `message` you pass to `BadRequest`, `Conflict`, `Domain` and the rest yourself.

What stays French regardless: log lines — except the webhooks URL refusals, whose text is
also what gets logged for a blocked delivery
(`crates/rbs-cli/templates/features/webhooks/delivery.rs.jinja`), so that one line follows
the generation language instead. Also unaffected: code comments; the `mail`/`auth` emails
(subjects and the HTML templates under `templates/mail/`); the per-operation descriptions
and summaries written into the generated handlers (`#[utoipa::path(… description = …)]`
and doc comments); and the schema descriptions carried by `rbs-core`'s own types
(`ProblemDetails`'s fields, `Page`, `CursorPage`, the filter schemas).

Switching an existing project to English takes two edits, both by hand: `lang = "en"`
under `[server]` in `config/default.toml` (run time) and `lang = "en"` under
`[package.metadata.rbs]` in `Cargo.toml` (so a future `add`/`generate` writes English) —
then translate by hand the messages already generated, listed above. `rbs upgrade`
rewrites none of them.

A divergence to watch on an older project: one generated before 1.5.0 without `--lang`
recorded `[package.metadata.rbs] lang` from the locale — `en` for any non-French locale,
including the `C.UTF-8` of CI runners and Docker images (see `from_locale` in
`crates/rbs-cli/src/lang.rs`) — while its runtime, with no `[server] lang` set, speaks
French. From 1.5.0, `add` and `generate` write their messages in the metadata's language:
align the two keys, one way or the other, before generating into such a project.

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
