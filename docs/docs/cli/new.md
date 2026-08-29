---
sidebar_position: 1
title: rbs new
---

# `rbs new`

Creates a project that runs as it is: a Cargo workspace, a `migration` crate, a `/health`
route, a `.env` and a Git repository. Nothing is compiled and no database is contacted —
the command writes files and stops.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs new -h
Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health

Usage: rbs new [OPTIONS] <NAME>

Arguments:
  <NAME>  Nom du projet, qui est aussi celui du répertoire créé

Options:
      --database-url <URL>     URL de connexion, à défaut de quoi la question est posée
      --database <MOTEUR>      Moteur de base sur lequel le projet tournera [default: postgres] [possible values: postgres, mysql, sqlite]
      --with <FEATURES>        Features à installer sans passer par les questions, séparées par des virgules
      --core-path <CHEMIN>     Crate `rbs-core` locale à utiliser au lieu de la version publiée
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help (see more with '--help')
  -V, --version                Print version
```

`<NAME>` is both the Cargo package name and the directory name. It must start with an ASCII
letter and hold only letters, digits, `-` and `_`.

## Flags

| Flag | Effect |
|---|---|
| `--database-url <URL>` | Connection URL written to the project's `.env` as `RBS_DATABASE__URL`. Absent, the question is asked — or the default is taken under `--yes`. |
| `--database <MOTEUR>` | The engine the project will run on: `postgres`, `mysql` or `sqlite`. Defaults to `postgres`. |
| `--with <FEATURES>` | Features to install at creation, comma-separated. Installed for real — see below. |
| `--core-path <CHEMIN>` | Points the generated manifest at a local `rbs-core` checkout instead of the published crate — the mode rbs is developed in, described [below](#building-against-a-local-core). |
| `--template-dir <CHEMIN>` | Renders the project from a directory of templates instead of the ones embedded in the binary. |
| `-y`, `--yes` | Asks nothing: takes the defaults and runs. |

`--template-dir` and `--yes` are global — every command accepts them — but `--yes` is read
only by `rbs new`, the one command that asks anything, and `--template-dir` only by
`rbs new` and [`rbs add`](./add.md).

## Choosing the engine

```text
$ rbs new blog --database sqlite --yes
```

Manifests, `.env.example`, the compose file and the configuration all follow the value
chosen. `sea-orm` gets the matching `sqlx-*` feature, and the generated migration avoids
anything that has no equivalent on the other two.

An unknown value is refused before anything is written:

```text
$ rbs new blog --database oracle
error: invalid value 'oracle' for '--database <MOTEUR>'
  [possible values: postgres, mysql, sqlite]

For more information, try '--help'.
```

Without the flag, `postgres` stays the default, and a manifest with no `database` key reads
back as a PostgreSQL project — no project created before this flag existed changes
behaviour.

:::warning
`--database` and `--database-url` must agree. Asking for `--database mysql` with a
`postgres://` URL is a refusal, raised during the verification phase and therefore before
the first file is written.

The same contradiction reached after the fact — by editing the `.env` of an existing
project — is what [`rbs doctor`](./doctor.md) reports, naming both values.
:::

SQLite is the one that changes the shape of the project rather than a line of it: no
compose file, no server to wait for in [`rbs dev`](./dev.md), and a URL with neither host
nor port.

## Creating a project

```text
$ rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --yes
✓ blog créé — 17 fichiers

  cd blog
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

The seventeen files:

```text
blog/.env
blog/.env.example
blog/.gitignore
blog/Cargo.toml
blog/config/default.toml
blog/config/development.toml
blog/docker-compose.yml
blog/migration/Cargo.toml
blog/migration/src/lib.rs
blog/migration/src/main.rs
blog/src/health/controller.rs
blog/src/health/mod.rs
blog/src/main.rs
blog/src/openapi.rs
blog/src/router.rs
blog/src/seeds/main.rs
blog/src/state.rs
```

`docker-compose.yml` is the generated compose, covered below — its port here is `55432`,
taken from the URL rather than the engine's own `5432`.

`git init` runs last. Should it fail, the project is still complete: the command says so
on stderr instead of failing.

The manifest depends on `rbs-core` from crates.io, at the version of the CLI that wrote
it. Nothing has to be built or checked out first.

## What carries idempotence

The generated `Cargo.toml` holds an rbs section, and that section is the only place where
rbs keeps state about a project:

```text
[package.metadata.rbs]
version = "1.0.0"
features = ["health"]
database = "postgres"
```

`version` is the rbs that generated the project — [`rbs doctor`](./doctor.md) compares it
to its own, and [`rbs upgrade`](./upgrade.md) is what moves it. `database` is the engine
the project was created for. `features` grows as [`rbs generate`](./generate.md) and
[`rbs add`](./add.md) install things, and it is what turns a second run of the same
command into a no-op rather than a duplicate. A state file of its own would have drifted
from the repository the first time someone forgot to commit it; the manifest is already
versioned.

## The three questions

Without `--yes`, and for each answer no flag supplied, `rbs new` asks for the project name,
the PostgreSQL URL — default `postgres://postgres:postgres@localhost:5432/<name>`, with
dashes turned into underscores — and the features to install.

`--yes` short-circuits before the first question is printed, which is what keeps the
command usable in CI. Without a terminal and without `--yes`, it names the flags that
would have replaced the questions:

```text
$ rbs new sans-tty < /dev/null
erreur : aucun terminal interactif pour poser les questions : relancez avec `--yes` pour prendre les défauts, ou donnez les réponses en flags — le nom en argument, `--database-url` et `--with`
```

## Building against a local core

By default the generated manifest depends on `rbs-core` from the registry, which is what
a project wants. `--core-path` replaces that dependency with a path to a local checkout
of the crate:

```text
$ rbs new blog --core-path /private/tmp/rbs-core --yes
✓ blog créé — 17 fichiers

  cd blog
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux

$ grep rbs-core blog/Cargo.toml
rbs-core = { path = "/private/tmp/rbs-core", default-features = false, features = ["postgres"] }
```

This is the mode rbs is developed in, and the only reason to reach for the flag: a change
to the core is proved by generating a project against it, before the version carrying it
is published. The path is canonicalised into the manifest, so Cargo resolves it from the
project rather than from the directory the command ran in.

Two commands then read the manifest differently. [`rbs doctor`](./doctor.md) reports the
core as taken from a local path instead of naming a version, and
[`rbs upgrade`](./upgrade.md) leaves the dependency alone — a path has no version to
raise.

## Templates from disk

`--template-dir` replaces the embedded skeleton with a directory laid out the same way: one
`.jinja` template per file to write, the suffix stripped on output. Below, a copy of the
skeleton with one line appended to its `.env.jinja`:

```text
$ rbs new maison --template-dir /private/tmp/rbs-demo/mes-templates --yes
✓ maison créé — 17 fichiers

  cd maison
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux

$ tail -2 maison/.env
RUST_LOG=info,maison=debug
MAISON=1
```

## `--with` installs

`--with` names features to install at creation, comma-separated. rbs knows seven —
`auth`, `ci`, `docker`, `jobs`, `mail`, `redis` and `storage` — and installs every one
named, in the same pass that writes the project:

```text
$ rbs new site --with auth --yes
✓ site créé — 17 fichiers
  + auth     9 fichiers, 1 migration

  cd site
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

The order installed is derived from the names rather than from the order they were typed
in — alphabetical, the same order [`rbs add`](./add.md) lists the seven in:

```text
$ rbs new with-demo --database-url postgres://rbs:secret@localhost:5432/with_demo --with storage,auth,docker --yes
✓ with-demo créé — 17 fichiers
  + auth     9 fichiers, 1 migration
  + docker   2 fichiers
  + storage  4 fichiers

  cd with-demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

`storage,auth,docker` was typed; `auth`, then `docker`, then `storage` were installed, and
that is the order `[package.metadata.rbs]` records them in — the same one a second `rbs
add` of any of them would leave untouched.

A name that is no feature at all is refused before the first file is written:

```text
$ rbs new site --with graphql --yes
erreur : `graphql` n'est pas une feature rbs — disponibles : auth, ci, docker, jobs, mail, redis, storage
```

## The generated compose

Unless one of the two cases below applies, `rbs new` writes a `docker-compose.yml` next to
the project, holding the database its URL describes — the identifiers, the database name
and the published port all read from it, none of them retyped:

```yaml
name: blog

services:
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: rbs
      POSTGRES_PASSWORD: rbs
      POSTGRES_DB: blog
    # Le port publié est celui du .env : c'est ce qui rend `docker compose up -d` suivi
    # de `cargo run` vrai sans recopier une valeur d'un fichier à l'autre. Le conflit
    # avec un PostgreSQL déjà installé sur la machine se règle en changeant les deux.
    ports:
      - "55432:5432"
    # PostgreSQL 18 place ses données sous /var/lib/postgresql/18/docker : c'est le
    # répertoire parent qui se monte, et non le /var/lib/postgresql/data des versions
    # précédentes, qui ne persisterait rien.
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rbs -d blog"]
      interval: 2s
      timeout: 3s
      retries: 30

  # <rbs:services>
  # </rbs:services>

volumes:
  pgdata:
```

`docker compose up -d` starts it. The `# <rbs:services>` anchor is where [`rbs
add`](./add.md) inserts the services `docker` brings, and it is one of the ten anchors
[`rbs doctor`](./doctor.md) checks — nine on a project with no compose to carry a tenth.

Two cases write nothing:

- **a SQLite project** — there is no server to run, and its URL has neither host nor port
  to carry into a compose;
- **a URL whose host is not local** — the container would only duplicate a database
  already reachable elsewhere.

```text
$ rbs new sqlite-demo --database sqlite --yes
✓ sqlite-demo créé — 16 fichiers

  cd sqlite-demo
  cargo run          # la base visée est dans .env
```

Sixteen files, not seventeen: the count is how you tell, since nothing in the output names
the compose by absence.

A project created before rbs 1.1.0 has no compose either, and running
[`rbs upgrade`](./upgrade.md) does not add one — it only rewrites
`[package.metadata.rbs]`. [`rbs add docker`](./add.md) writes a whole compose in that
case, deployment services included.

## Failures

Everything checkable is checked before rendering begins, and rendering finishes before the
first file is written. A refused name, an unavailable feature or a template that does not
render all leave the disk exactly as they found it.

An occupied directory:

```text
$ rbs new blog --yes
erreur : /private/tmp/rbs-demo/blog existe déjà : choisissez un autre nom, ou retirez ce répertoire
```

A name that could not be a Cargo package:

```text
$ rbs new 4chan --yes
erreur : `4chan` n'est pas un nom de projet utilisable : lettres, chiffres, `-` et `_`, en commençant par une lettre
```

Each of these exits with status 1 and writes nothing.
