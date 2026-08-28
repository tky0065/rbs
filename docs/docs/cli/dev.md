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

Usage: rbs dev [OPTIONS]

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
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

1. **`docker compose up -d`**, if and only if the `docker` feature is declared in
   `[package.metadata.rbs]`. A project without it never has a compose file looked for;
2. **waiting for the database** to accept a connection. Skipped for SQLite, which has no
   server to wait on — its URL has neither host nor port;
3. **[`rbs migrate up`](./migrate.md)**, so that a schema change pulled from a colleague
   applies without a second command;
4. **the server**, `cargo run`, restarted on every change under `src/`.

## Two waiting times, not one

The database gets 30 seconds when `rbs dev` has just brought the compose stack up, and 3
when it was supposed to be running already.

The asymmetry is the point. A container that has just started legitimately takes tens of
seconds to accept connections. A database that was meant to be up and is not will never
come up on its own — and thirty seconds of silence to learn you forgot to start PostgreSQL
are thirty seconds wasted.

```text
erreur : rien ne répond sur 127.0.0.1:1 : la base du projet n'est pas démarrée

démarrez-la — `docker compose up -d` si la feature docker est installée — ou corrigez RBS_DATABASE__URL dans le .env du projet
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
