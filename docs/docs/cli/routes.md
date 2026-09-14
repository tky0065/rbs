---
sidebar_position: 2.6
title: rbs routes
---

# `rbs routes`

Lists the project's routes — method, path, `operation_id` and guard — in one command, as a
table for a human or as JSON for a script or an agent.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs routes --help
Liste les routes du projet : méthode, chemin, operation_id et garde

Usage: rbs routes [OPTIONS]

Options:
      --json     Rend les routes en JSON sur la sortie standard, pour un script ou un agent
  -h, --help     Print help
  -V, --version  Print version
```

## The table

On a project created with `rbs new routes-api --with auth`:

```text
$ rbs routes
MÉTHODE  CHEMIN                     OPERATION_ID              GARDE
POST     /auth/change-password      auth_change_password      bearer
POST     /auth/forgot-password      auth_forgot_password      public
POST     /auth/login                auth_login                public
POST     /auth/logout               auth_logout               public
GET      /auth/me                   auth_me                   bearer
POST     /auth/refresh              auth_refresh              public
POST     /auth/register             auth_register             public
POST     /auth/resend-verification  auth_resend_verification  public
POST     /auth/reset-password       auth_reset_password       public
GET      /auth/sessions             auth_list_sessions        bearer
DELETE   /auth/sessions             auth_revoke_sessions      bearer
DELETE   /auth/sessions/{id}        auth_revoke_session       bearer
POST     /auth/verify-email         auth_verify_email         public
GET      /health                    health                    public
```

Rows are sorted by path, then by method in the order GET, POST, PUT, PATCH, DELETE; any
other verb comes after them. An operation without an `operation_id` shows `-` in that
column.

`GARDE` is `bearer` when the operation declares a non-empty `security` — or, when it
declares none, when the document declares one globally — and `public` otherwise. An
explicit empty `security` on an operation makes it public even under a global requirement,
as OpenAPI specifies. A handler generated under [`auth`](../guides/auth.md) carries
`security(("bearer" = []))` in its annotation, which is what this column reads.

## `--json`

An array of objects with four keys, always present — `operation_id` is `null` when the
operation has none. It is the only thing on standard output: the project's compilation
goes to standard error.

```text
$ rbs routes --json
[
  {
    "methode": "POST",
    "chemin": "/auth/change-password",
    "operation_id": "auth_change_password",
    "garde": "bearer"
  },
  {
    "methode": "POST",
    "chemin": "/auth/forgot-password",
    "operation_id": "auth_forgot_password",
    "garde": "public"
  },
  …
]
```

The block is cut after two entries; the command prints them all.

## Where the routes come from

From the OpenAPI document, not from the router: the document carries, for every operation,
its `operation_id` and the token requirement its annotation declares — what the router does
not say. The document is obtained the way [`rbs openapi export`](./openapi.md) and
[`rbs generate client`](./client.md) obtain it, by running the project's `openapi` binary,
so a route mounted without an `#[utoipa::path]` does not show up. It is missing from the
document too, and from every client generated from it.

The failures are those of `rbs openapi export`, word for word. Under `--json`, the error and
its remedy both go to standard error, and the exit code is 1.
