---
sidebar_position: 5
title: rbs doctor
---

# `rbs doctor`

Diagnoses a generated project through four checks: the anchors, the `.env`, the versions
and the database. Each is independent and returns its verdict without stopping the others —
a diagnosis that halts on the first problem has to be re-run once per problem.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs doctor --help
Diagnostique le projet : ancres, .env, base joignable, versions

Usage: rbs doctor [OPTIONS]

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

No flag of its own. The two global options are accepted because clap propagates them, and
neither does anything here.

## The four checks

| Check | What it looks at |
|---|---|
| `ancres` | The nine insertion points: `// <rbs:features>` in `src/main.rs`, `// <rbs:routes>` in `src/router.rs`, `// <rbs:openapi>` in `src/openapi.rs`, `// <rbs:migration_modules>` and `// <rbs:migrations>` in `migration/src/lib.rs`, `// <rbs:state_champs>` and `// <rbs:state_init>` in `src/state.rs`, `// <rbs:startup>` in `src/main.rs`, `// <rbs:seeds>` in `src/seeds/main.rs`. |
| `.env` | Every variable declared by `.env.example` is set in `.env`. `.env.example` is the reference because it is versioned and generated alongside the skeleton — a list kept inside the CLI would have been a second truth to keep in sync. |
| `versions` | The rbs recorded in `[package.metadata.rbs]`, the `rbs-core` dependency, and the CLI running the diagnosis. |
| `base` | A TCP connection within three seconds, then the server version — asked of the `migration` crate's binary, since rbs embeds no SQL client. PostgreSQL 18 is the minimum: `uuidv7()`, which generated migrations use for the default primary key, does not exist before it. |

A missing anchor breaks nothing until a generation happens, which is exactly why `doctor`
looks for it before [`rbs generate`](./generate.md) trips over it.

## A healthy project

```text
$ rbs doctor
  ✓ ancres     les 7 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✓ base       PostgreSQL 18.6 répond sur localhost:55432
✓ le projet est sain
```

Exit status 0.

## A project with problems

Below, the same project with `// <rbs:openapi>` deleted from `src/openapi.rs`,
`RBS_LOG_FORMAT` removed from `.env`, and PostgreSQL stopped:

```text
$ rbs doctor
  ✗ ancres     openapi manque dans src/openapi.rs
      dans src/openapi.rs :
      // <rbs:openapi>
      // </rbs:openapi>
  ✗ .env       RBS_LOG_FORMAT absente du .env
      ajoutez au .env :
      RBS_LOG_FORMAT=pretty
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✗ base       rien ne répond sur localhost:55432
      démarrez PostgreSQL, ou corrigez l'URL du .env
```

Three failures, one check still green, and every failing line carries what to do about it —
the anchor block to paste back, the `.env` line to add, the server to start.

Exit status 1. A diagnosis that finds something is not a failure of the command, but a
script has to be able to tell it apart from a healthy project, so the status differs.

## Reachable but unreadable

The two halves of the `base` check fail separately. Here the host answers on the port, but
the version could not be read because the `migration` crate did not build — the remedy
names the command to run by hand:

```text
$ rbs doctor
  ✓ ancres     les 7 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✗ base       localhost:5432 répond, mais sa version reste inconnue : la crate migration a échoué (code 1)
      vérifiez que `cargo run -p migration -- version` aboutit
```

## Outside a project

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Exit status 1.
