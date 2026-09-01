---
sidebar_position: 6
title: rbs dev
---

# `rbs dev`

Starts the project in one command: the services it needs, the pending migrations, then the
server, restarted on every change. It is the command you leave running.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs dev --help
Démarre le projet : services, migrations, serveur relancé à chaque changement

Usage: rbs dev

Options:
  -h, --help     Print help
  -V, --version  Print version
```

No flags of its own. What it does depends entirely on what the project declares.

## The plan

`rbs dev` shows what it is about to do before doing it, like every command that touches an
existing project:

```text
  base        127.0.0.1:1
  migrations  rbs migrate up
  serveur     cargo run, relancé à chaque changement
```

Up to four steps, in order:

1. **`docker compose up -d`**, if and only if the project has a `docker-compose.yml` —
   written by [`rbs new`](./new.md) for most projects, whether or not `docker` is
   installed. A project with neither never has a compose file looked for;
2. **waiting for the database** to accept a connection. Skipped for SQLite, which has no
   server to wait on — its URL has neither host nor port;
3. **[`rbs migrate up`](./migrate.md)**, so that a schema change pulled from a colleague
   applies without a second command;
4. **the server**, `cargo run`, restarted on every change under `src/`.

A project with a compose — the default, for most — shows the extra step first:

```text
  compose     docker-compose.yml
  base        localhost:15432
  migrations  rbs migrate up
  serveur     cargo run, relancé à chaque changement
```

`docker compose up -d` is called with no `--profile` — the same command
[`rbs new`'s](./new.md) own hint names. That brings up `db` and
whatever [`redis`](../guides/cache.md) or [`mail`](../guides/mail.md) added, all outside
any profile, but never `api` or `migrate`: [`rbs add docker`](./add.md) put both of those
under the `app` profile precisely so that `rbs dev` — which runs the server itself, from
source, on every save — never has an image to build. `docker compose --profile app up
--build` is the other path, the one for running the project as its own container instead
of as `cargo run`.

## Two waiting times, not one

The database gets 30 seconds when `rbs dev` has just brought the compose stack up, and 3
when it was supposed to be running already.

The asymmetry is the point. A container that has just started legitimately takes tens of
seconds to accept connections. A database that was meant to be up and is not will never
come up on its own — and thirty seconds of silence to learn you forgot to start PostgreSQL
are thirty seconds wasted.

```text
erreur : rien ne répond sur 127.0.0.1:1 : la base du projet n'est pas démarrée

démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez RBS_DATABASE__URL dans le .env du projet
```

The message names the host and the port it tried, and the two ways out. It is not a panic
trace: a database that is not running is an ordinary Tuesday, not a bug in rbs.

## The watch

A change under `src/` restarts the server. A change under `target/` does not — and that
one is not a filter on events but a refusal to descend into the directory at all. A build
script writing into `target/debug/build/…/out/` while the server restarts is exactly the
loop this avoids.

The hard part is neither the debounce nor the filtering. It is killing the server: a
`cargo run` killed without its child leaves the port occupied, and the gesture differs on
Linux, macOS and Windows. The child is started in its own process group and the whole group
is signalled, which is what makes the port free by the next restart — asserted by a test
that runs on all three platforms of the CI.

## Failures

| Situation | What happens |
|---|---|
| No `.env`, or no database URL in it | Refusal naming the file and the variable |
| URL with no host | Refusal naming the URL — a URL rbs cannot dial is a URL to fix |
| Nothing listening | The message above, after the applicable timeout |
| Migration fails | The migration binary's own error, and `rbs dev` stops there |
| Not in a project | Refusal naming what it looked for, like every other command |

The server itself is not supervised: once `cargo run` is up, its output is yours, and
`Ctrl-C` stops the whole thing.
