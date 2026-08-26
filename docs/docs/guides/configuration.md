---
sidebar_position: 2
title: Configuration
---

# Configuration

A generated project reads its settings from five layers merged in a fixed order. You
change a port, a pool size or a database URL by writing in whichever layer matches how
long the change should live: a file for the project, an environment variable for the
machine.

## The five layers

Each layer overrides the previous one:

1. **Built-in defaults**, carried by `rbs-core`.
2. **`config/default.toml`** — settings shared by every environment.
3. **`config/{RBS_ENV}.toml`** — settings of the active profile. `RBS_ENV` defaults to
   `development`.
4. **`.env`** — only keys prefixed with `RBS_`, and only in this file's own reading.
   rbs never exports them into the process environment, so the precedence between the
   last two layers stays explicit.
5. **Environment variables** prefixed with `RBS_`.

In a variable name, `__` separates nesting levels: `RBS_DATABASE__URL` feeds
`database.url`, `RBS_SERVER__PORT` feeds `server.port`. Both TOML files are optional —
a project configured entirely through the environment loads just as well.

The profile itself is resolved in two passes: the profile-independent layers are merged
once to read `env` out of them, and that value then names the `config/{env}.toml` of the
final assembly. `RBS_ENV` therefore works from the environment *and* from `.env`.

Here is `config/default.toml` as `rbs new` writes it:

```toml file=examples/hello-crud/config/default.toml
```

And the profile file, which only carries what differs:

```toml file=examples/hello-crud/config/development.toml
```

`.env` is where the database URL lives, next to the two logging variables:

```bash file=examples/hello-crud/.env.example
```

## Every setting

| Key | Variable | Default |
|---|---|---|
| `env` | `RBS_ENV` | `development` |
| `server.host` | `RBS_SERVER__HOST` | `127.0.0.1` |
| `server.port` | `RBS_SERVER__PORT` | `8080` |
| `database.url` | `RBS_DATABASE__URL` | **none — required** |
| `database.max_connections` | `RBS_DATABASE__MAX_CONNECTIONS` | `10` |
| `database.min_connections` | `RBS_DATABASE__MIN_CONNECTIONS` | `0` |
| `database.connect_timeout_secs` | `RBS_DATABASE__CONNECT_TIMEOUT_SECS` | `5` |
| `database.acquire_timeout_secs` | `RBS_DATABASE__ACQUIRE_TIMEOUT_SECS` | `5` |
| `database.idle_timeout_secs` | `RBS_DATABASE__IDLE_TIMEOUT_SECS` | `600` |
| `database.max_lifetime_secs` | `RBS_DATABASE__MAX_LIFETIME_SECS` | `1800` |
| `docs.swagger_ui` | `RBS_DOCS__SWAGGER_UI` | `true` |
| `docs.openapi_json` | `RBS_DOCS__OPENAPI_JSON` | `true` |

`database.url` is the only key without a default. Nothing sensible can be guessed for it,
so its absence stops the process at startup with a message naming the field.

### Why `docs.swagger_ui` and `docs.openapi_json` are two settings

The two needs are not symmetrical. Turning the interface off while keeping the document
is what you do to generate clients or to check a contract from CI; the reverse has no
use. One boolean could not express that, so there are two. The
[OpenAPI guide](./openapi.md) covers what each one actually mounts.

## Failing to start is not an HTTP response

Loading returns `Result<Config, ConfigError>`, and `ConfigError` is a type of its own —
deliberately *not* the runtime's `Error`. A runtime error knows how to become an
`application/problem+json` response; a startup error has no client to answer. It reaches
`main`, which propagates it:

```rust file=examples/hello-crud/src/main.rs region=demarrage
```

`Config::load()` reads from the current directory, so a project is launched from its own
root — that is where `config/` and `.env` are.

## Judge for yourself

Override a setting without touching a file:

```bash
RBS_SERVER__PORT=9090 cargo run
```

The precedence rules above are each covered by a test, and the module is small enough to
read in one sitting:

```bash
cargo test -p rbs-core config::tests
```
