---
sidebar_position: 1
title: Setting up
---

# Setting up

This is the first tutorial, and the one every page after it assumes you have
already been through: it takes an empty directory to a project named `demo`, running,
with a health check answering on `localhost:8080`. Each tutorial that follows — CRUD,
auth, storage, mail, cache, background jobs, observability, a generated TypeScript
client — picks up the same `demo` from exactly this point, so the project you build here
is the one you will still have at the end of the series.

If the CLI is not installed yet, [Getting started](../getting-started.md) covers
`cargo install rbs-cli` and what `rbs --version` should answer, including the note on
the unrelated Ruby `rbs` sharing the name. This page assumes that step is done.

## What you need

The same as Getting started: **Rust stable**, edition 2024, and **Docker with
Compose** — `rbs new` writes the `docker-compose.yml` this page starts a few steps
down. **PostgreSQL 14 or later**, however you run it, and **curl** for the last
section.

## 1. Create the project

```bash
rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
```

{/* rbs:transcript cmd="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" */}
```text
$ rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
✓ demo créé — 23 fichiers

  cd demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

That line is a success, not an error — [Getting started](../getting-started.md)
explains why the CLI answers in French regardless of your locale. Twenty-three files,
one command: proof the project exists with a `health` feature already wired in, and
nothing else, since `--yes` took every default. [Architecture](../architecture.md)
maps what each of those files is for; this series won't repeat that map.

## 2. Start the database

```bash
cd demo
docker compose up -d --wait
```

{/* rbs:libre raison="sortie de docker compose, qui démarre un conteneur : hors de portée du rejeu" */}
```text
 Container demo-db-1  Started
 Container demo-db-1  Waiting
 Container demo-db-1  Healthy
```

`Healthy` is Compose's own healthcheck passing, which is what `--wait` waits for: proof
the PostgreSQL container is already accepting connections, not just that it started.
That is what makes the next command safe to run immediately.

## 3. Apply the migrations

```bash
rbs migrate up
```

{/* rbs:transcript cmd="rbs migrate up" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" base="oui" extrait="oui" */}
```text
✓ migrations appliquées
```

Nothing has changed shape yet — the command only created the table SeaORM uses to
track which migrations ran. That line is proof the URL in `.env` is right, before
anything else in the project depends on it.

## 4. Run the server

```bash
cargo run
```

{/* rbs:libre raison="journal d'un serveur qui tourne : le rejeu ne lance aucun serveur, et l'heure change à chaque démarrage" */}
```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

A clean start here is proof the whole Axum, SeaORM and utoipa tree compiles together —
if any of it were wired wrong, this line would never have printed. It is also why this
is the slowest command on this page. Once this line prints, `demo` is listening, and the
terminal it ran in is now the server's — leave it running.

## Verify

From a second terminal:

```bash
curl -i http://127.0.0.1:8080/health
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 200 OK
content-type: application/json

{"status":"ok","checks":{"database":"ok"}}
```

A `200` with `"database":"ok"` is proof of two things at once: the server answers, and
it can reach the PostgreSQL container started in step 2. Every tutorial in this series
picks up from exactly this state.

## Going further

- [Getting started](../getting-started.md) walks the same commands further,
  generating a CRUD feature and reading its OpenAPI document.
- [Architecture](../architecture.md) maps every file `rbs new` just wrote to the
  layer it belongs to.
- [`rbs new`](../cli/new.md) covers the flags this page didn't use — an existing
  database, or a project with no server to start.
- [`rbs migrate`](../cli/migrate.md) covers `down` and `status`, the two commands
  this page didn't need yet.
- [Your first resource](./first-resource.md) is the next tutorial: `rbs generate crud`
  turns a `--fields` declaration into an entity, its migration, and every layer between.
