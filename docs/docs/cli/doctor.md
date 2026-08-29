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
| `ancres` | The nine Rust comment anchors: `// <rbs:features>` in `src/main.rs`, `// <rbs:routes>` in `src/router.rs`, `// <rbs:openapi>` in `src/openapi.rs`, `// <rbs:migration_modules>` and `// <rbs:migrations>` in `migration/src/lib.rs`, `// <rbs:state_champs>` and `// <rbs:state_init>` in `src/state.rs`, `// <rbs:startup>` in `src/main.rs`, `// <rbs:seeds>` in `src/seeds/main.rs` — plus the YAML `# <rbs:services>` in `docker-compose.yml`, tenth and optional: a project with no compose has none to carry it. |
| `.env` | Every variable declared by `.env.example` is set in `.env`. `.env.example` is the reference because it is versioned and generated alongside the skeleton — a list kept inside the CLI would have been a second truth to keep in sync. |
| `versions` | The rbs recorded in `[package.metadata.rbs]`, the `rbs-core` dependency, and the CLI running the diagnosis. |
| `base` | The driver compiled into the manifest against the URL's scheme, then a TCP connection within three seconds, then the server version — asked of the `migration` crate's binary, since rbs embeds no SQL client. Each engine has its own floor, and each floor has a reason: PostgreSQL 14, the oldest still maintained; MySQL 8.0, for `FOR UPDATE SKIP LOCKED`; SQLite 3.35, for `UPDATE … RETURNING`. |

A missing anchor breaks nothing until a generation happens, which is exactly why `doctor`
looks for it before [`rbs generate`](./generate.md) trips over it.

The driver comes before the connection on purpose. A server that answers proves nothing
when the driver compiled into your binary cannot speak its protocol, and probing the port
first would charge three seconds to a diagnosis that fits in two file reads:

```text
  ✗ base       le manifeste compile `sqlx-postgres` et RBS_DATABASE__URL est une URL `mysql://`
      alignez les deux : la feature `sqlx-mysql` de sea-orm au manifeste, ou une URL `postgres://` dans le .env
```

That is the contradiction [`rbs new`](./new.md) refuses outright, met here after the fact —
on a project whose `.env` was edited later.

## Installed features

Each feature that carries configuration adds a line of its own, and the line only exists
on a project that declared the feature. `jobs` is the one this milestone added:

```text
  ✗ jobs       config/default.toml ne porte pas de section `[jobs]`
      ajoutez à config/default.toml :
      [jobs]
      max_attempts = 5
      retry_delay_secs = 30
      poll_interval_secs = 1
```

A feature declared in `[package.metadata.rbs]` whose section has vanished from the
configuration is a project that compiles and fails at startup — which `doctor` can say
cold, before you start it. A section commented out does not count as a section.

## A healthy project

```text
$ rbs doctor
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running `target/debug/migration version`
  ✓ ancres     les 10 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core 1.1.0 alignés sur le CLI 1.1.0
  ✓ base       postgres 17.10 répond sur localhost:55446
  ✓ jobs       la configuration de la file est en place
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
  ✓ versions   projet et rbs-core 1.1.0 alignés sur le CLI 1.1.0
  ✗ base       rien ne répond sur localhost:55446
      lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env
  ✓ jobs       la configuration de la file est en place
attention : le projet demande votre attention
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
   Compiling migration v0.1.0 (…/demo/migration)
error[E0425]: cannot find value `url_de_la_base` in this scope
  --> migration/src/main.rs:70:13
   |
70 |     let _ = url_de_la_base;
   |             ^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `migration` (bin "migration") due to 1 previous error
  ✓ ancres     les 10 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core 1.1.0 alignés sur le CLI 1.1.0
  ✗ base       localhost:55446 répond, mais sa version reste inconnue : la crate migration a échoué (code 101)
      vérifiez que `cargo run -p migration -- version` aboutit
  ✓ jobs       la configuration de la file est en place
attention : le projet demande votre attention
```

## Outside a project

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Exit status 1.
