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
$ rbs new --help
Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health

Usage: rbs new [OPTIONS] <NOM>

Arguments:
  <NOM>  Nom du projet, qui est aussi celui du répertoire créé

Options:
      --database-url <URL>     URL de la base PostgreSQL, à défaut de quoi la question est posée
      --with <FEATURES>        Features à installer sans passer par les questions, séparées par des virgules
      --core-path <CHEMIN>     Crate `rbs-core` locale à utiliser au lieu de la version publiée
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

`<NOM>` is both the Cargo package name and the directory name. It must start with an ASCII
letter and hold only letters, digits, `-` and `_`.

## Flags

| Flag | Effect |
|---|---|
| `--database-url <URL>` | Connection URL written to the project's `.env` as `RBS_DATABASE__URL`. Absent, the question is asked — or the default is taken under `--yes`. |
| `--with <FEATURES>` | Features to install at creation, comma-separated. What 0.1.0 does with it is described below. |
| `--core-path <CHEMIN>` | Points the generated manifest at a local `rbs-core` checkout instead of the published crate. The path is canonicalised, so Cargo resolves it from the new project rather than from where you stood. |
| `--template-dir <CHEMIN>` | Renders the project from a directory of templates instead of the ones embedded in the binary. |
| `-y`, `--yes` | Asks nothing: takes the defaults and runs. |

`--template-dir` and `--yes` are global — every command accepts them — but `--yes` is read
only by `rbs new`, the one command that asks anything, and `--template-dir` only by
`rbs new` and [`rbs add`](./add.md).

## Creating a project

```text
$ rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --core-path /private/tmp/rbs-core --yes
✓ blog créé — 16 fichiers

  cd blog
  cargo run          # la base visée est dans .env
```

The sixteen files:

```text
blog/.env
blog/.env.example
blog/.gitignore
blog/Cargo.toml
blog/config/default.toml
blog/config/development.toml
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

`git init` runs last. Should it fail, the project is still complete: the command says so
on stderr instead of failing.

`--core-path` is what the walkthrough on these pages uses, because `rbs-core` 0.1.0 is not
on crates.io yet. Drop it and the manifest gets `rbs-core = "0.1.0"` from the registry.

## What carries idempotence

The generated `Cargo.toml` holds an rbs section, and that section is the only place where
rbs keeps state about a project:

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health"]
```

`version` is the rbs that generated the project — [`rbs doctor`](./doctor.md) compares it
to its own. `features` grows as [`rbs generate`](./generate.md) and [`rbs add`](./add.md)
install things, and it is what turns a second run of the same command into a no-op rather
than a duplicate. A state file of its own would have drifted from the repository the first
time someone forgot to commit it; the manifest is already versioned.

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

## Templates from disk

`--template-dir` replaces the embedded skeleton with a directory laid out the same way: one
`.jinja` template per file to write, the suffix stripped on output. Below, a copy of the
skeleton with one line appended to its `.env.jinja`:

```text
$ rbs new maison --template-dir /private/tmp/rbs-demo/mes-templates --core-path /private/tmp/rbs-core --yes
✓ maison créé — 16 fichiers

  cd maison
  cargo run          # la base visée est dans .env

$ tail -2 maison/.env
RUST_LOG=info,maison=debug
MAISON=1
```

## `--with` in this version

`--with` names features to install at creation. rbs knows three — `auth`, `ci` and
`docker` — and refuses all of them here: it installs them through [`rbs add`](./add.md)
instead, and says so rather than recording in `[package.metadata.rbs]` a feature it did
not lay down.

```text
$ rbs new site --with auth --yes
erreur : `auth` ne s'installe pas à la création : créez le projet sans `--with`, puis `rbs add auth`
```

A name that is no feature at all is refused with the list of those that are:

```text
$ rbs new site --with graphql --yes
erreur : `graphql` n'est pas une feature rbs — disponibles : docker, ci, auth
```

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
