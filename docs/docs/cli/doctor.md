---
sidebar_position: 5
title: rbs doctor
---

# `rbs doctor`

Diagnoses a generated project through six checks: the anchors,
[`AGENTS.md`](../guides/agents.md), the relations already written into its models, the
`.env`, the versions and the database. Each is independent and returns its verdict without
stopping the others — a diagnosis that halts on the first problem has to be re-run once
per problem.

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
      --json     Rend le rapport en JSON sur la sortie standard, pour un script ou une CI
      --fix      Repose les ancres absentes avant de diagnostiquer
      --force    Repose les ancres même si le working tree Git est sale
  -h, --help     Print help
  -V, --version  Print version
```

Three flags, and only three. `--json` renders the report as a document; `--fix` puts the
missing anchors back before diagnosing, and `--force` lets it write on a dirty working
tree. `--force` only lifts that one guard, and is therefore refused on its own: nothing
else in `doctor` writes, so alone it would be taken and ignored. `--template-dir` and `--yes` are not accepted here: each is declared on the commands
that read it, so passing one is a clap error rather than a flag that is taken and ignored.

## The six checks

| Check | What it looks at |
|---|---|
| `ancres` | The ten Rust comment anchors: `// <rbs:features>` in `src/lib.rs` — or in `src/main.rs`, on a project generated before that library existed — `// <rbs:routes>` and `// <rbs:layers>` in `src/router.rs`, `// <rbs:openapi>` in `src/openapi.rs`, `// <rbs:migration_modules>` and `// <rbs:migrations>` in `migration/src/lib.rs`, `// <rbs:state_champs>` and `// <rbs:state_init>` in `src/state.rs`, `// <rbs:startup>` in `src/main.rs`, `// <rbs:seeds>` in `src/seeds/main.rs` — plus the YAML `# <rbs:services>` in `docker-compose.yml`, eleventh and optional: a project with no compose has none to carry it. |
| `agents` | [`AGENTS.md`](../guides/agents.md): present, its two zones present, the guide's version matching the CLI's, the inventory matching the project, every declared feature backed by a directory — and, only as a warning, a directory under `src/` that nothing declares. Covered on its own below. |
| `relations` | The two anchors a model needs to receive a relation — `// <rbs:relations:table>` and `// <rbs:related:table>`, one pair per entity. Outside the anchor registry above, since which file carries them depends on the project's own features. It only turns red on a model that already has a `belongs_to` or `has_many` but is missing one of its two anchors — a state a hand edit is the likely cause of, since [`rbs generate`](./generate.md) never leaves that behind. |
| `.env` | Every variable declared by `.env.example` is set in `.env`. `.env.example` is the reference because it is versioned and generated alongside the skeleton — a list kept inside the CLI would have been a second truth to keep in sync. |
| `versions` | The rbs recorded in `[package.metadata.rbs]`, the `rbs-core` dependency, and the CLI running the diagnosis. |
| `base` | The driver compiled into the manifest against the URL's scheme, then a TCP connection within three seconds, then the server version — asked of the `migration` crate's binary, since rbs embeds no SQL client. Each engine has its own floor, and each floor has a reason: PostgreSQL 14, the oldest still maintained; MySQL 8.0, for `FOR UPDATE SKIP LOCKED`; SQLite 3.35, for `UPDATE … RETURNING`. |

A missing anchor breaks nothing until a generation happens, which is exactly why `doctor`
looks for it before [`rbs generate`](./generate.md) trips over it.

The driver comes before the connection on purpose. A server that answers proves nothing
when the driver compiled into your binary cannot speak its protocol, and probing the port
first would charge three seconds to a diagnosis that fits in two file reads:

```text
  ✗ base        le manifeste compile `sqlx-postgres` et RBS_DATABASE__URL est une URL `mysql://`
      alignez les deux : la feature `sqlx-mysql` de sea-orm au manifeste, ou une URL `postgres://` dans le .env
```

That is the contradiction [`rbs new`](./new.md) refuses outright, met here after the fact —
on a project whose `.env` was edited later.

## The two warnings

Every other verdict above is pass or fail. `agents` can also warn, on one condition only:
a directory under `src/` that no installed fragment and no feature declared in
`[package.metadata.rbs]` accounts for — code nobody generated.

```text
  ! agents      écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
```

It stays a warning rather than a failure on purpose. Writing by hand what rbs does not
generate — an endpoint that is not a CRUD, an external HTTP client, a business rule — is
legitimate and expected; it is the very thing [`AGENTS.md`](../guides/agents.md) tells an
agent to do when it runs into code rbs has no business generating. Failing the command over
it would turn `rbs doctor` red on a perfectly healthy project the moment anyone adds that
kind of code, which would make the check useless in CI. A warning changes neither the exit
status nor the overall verdict: a project with nothing but a warning still exits 0 and is
still reported as healthy — only an actual failure does that.

The second warning belongs to `gardes`, and only exists on a project carrying
[`auth`](../guides/auth.md): a feature whose `create`, `update` or `delete` calls no
`require_role`.

```text
  ! gardes      écritures anonymes : articles, comments
      réservez-les à un rôle : `rbs generate crud <nom> --fields … --role admin` pose le garde à la génération, et `identite.require_role(Role::Admin)?` le pose à la main — voir le guide de l'authentification
```

Same reasoning, twice over. An API that writes without asking who is calling is a
legitimate design — a public catalogue, a service behind a gateway that already
authenticates — so the finding cannot be a failure. And the guard is recognised by that one
call, so a project protecting its writes some other way is named here too.

## Installed features

Each feature that carries configuration adds a line of its own, and the line only exists
on a project that declared the feature. `auth` adds two — its secret, and the `gardes`
check above. `jobs` is the one this milestone added:

```text
  ✗ jobs        config/default.toml ne porte pas de section `[jobs]`
      ajoutez à config/default.toml :
      [jobs]
      max_attempts = 5
      retry_delay_secs = 30
      poll_interval_secs = 1
```

A feature declared in `[package.metadata.rbs]` whose section has vanished from the
configuration is a project that compiles and fails at startup — which `doctor` can say
cold, before you start it. A section commented out does not count as a section.

## A machine-readable report

`--json` writes the same findings as a single document on standard output — nothing else
goes there, no colour, no glyphs — so a CI step can name the check that failed instead of
grepping for a cross. The exit code keeps the meaning it already had: 0 when the project is
healthy, 1 when a check failed.

```text
$ rbs doctor --json
{
  "sain": false,
  "checks": [
    {
      "name": "ancres",
      "status": "ok",
      "detail": "les 11 points d'insertion sont en place"
    },
    {
      "name": "base",
      "status": "erreur",
      "detail": "rien ne répond sur 127.0.0.1:5499",
      "remede": "lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env"
    }
  ]
}
```

`status` is `ok`, `avertissement` or `erreur` — the three states the text rendering draws as
`✓`, `!` and `✗`. `remede` is present only on the checks that carry one. `sain` is false as
soon as one check failed, which is the same condition as exit code 1.

```bash
rbs doctor --json | jq -r '.checks[] | select(.status != "ok") | "\(.name): \(.detail)"'
```

## Why it sometimes takes a minute

The `base` check runs the project's own migration binary, which means cargo builds the
`migration` crate first — a minute or more on a cold target directory. `doctor` announces
that line before it blocks rather than after, so a silent wait is never mistaken for a
hang:

```text
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
   Compiling sea-orm v2.0.2
   Compiling migration v0.1.0 (/tmp/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.57s
  ✓ base        postgres 18.6 répond sur 127.0.0.1:5432
✓ le projet est sain
```

The announcement is a line of the text rendering only; `--json` never carries it.

## A healthy project

```text
$ rbs doctor
  ✓ ancres      les 11 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
     Running `target/debug/migration version`
  ✓ base        postgres 18.6 répond sur localhost:55501
  ✓ jobs        la configuration de la file est en place
✓ le projet est sain
```

Exit status 0.

## A project with problems

Below, the same project with `// <rbs:openapi>` deleted from `src/openapi.rs`,
`RBS_LOG_FORMAT` removed from `.env`, and PostgreSQL stopped:

```text
$ rbs doctor
  ✗ ancres      openapi manque dans src/openapi.rs
      dans src/openapi.rs :
      // <rbs:openapi>
      // </rbs:openapi>
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✗ .env        RBS_LOG_FORMAT absente du .env
      ajoutez au .env :
      RBS_LOG_FORMAT=pretty
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  ✗ base        rien ne répond sur localhost:55501
      lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env
  ✓ jobs        la configuration de la file est en place
attention : le projet demande votre attention
```

Three failures, four checks still green, and every failing line carries what to do about
it — the anchor block to paste back, the `.env` line to add, the server to start.

Exit status 1. A diagnosis that finds something is not a failure of the command, but a
script has to be able to tell it apart from a healthy project, so the status differs.

## Putting the anchors back

An anchor is two comment lines, and nothing tells you where they used to be once they are
gone. `--fix` puts them back: each anchor declares the line it sits under — `.merge(docs)`
for `// <rbs:layers>`, `core: CoreState::new(db, config),` for `// <rbs:state_init>` — and
the block goes back beneath that line, in the column it had.

Repairing comes before diagnosing, so the `ancres` check of the same report counts what
has just been put back rather than reporting red on a project the command has just set
straight.

Below, a project whose `// <rbs:openapi>` and `// <rbs:state_init>` were deleted:

```text
$ rbs doctor --fix --force
plan pour /private/tmp/rbs-demo/demo

  ~ src/openapi.rs   modifié
  ~ src/state.rs     modifié

  2 fichiers à écrire

✓ 2 ancres reposées : openapi, state_init

  ✓ ancres      les 10 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 4 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core 1.1.0 alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running `target/debug/migration version`
  ✓ base        sqlite 3.51 répond sur demo.db
✓ le projet est sain
```

Exit status 0. The plan is shown before a byte is written, like every other command that
touches an existing project, and the write goes through the same journal: should one of
the two files fail, the other is put back as it was.

How exact the placement is can be read straight off Git, on a project whose anchors were
deleted after the commit:

```text
$ git diff --stat
```

Nothing. The two blocks came back to the byte where the skeleton had put them.

## A dirty working tree

```text
$ rbs doctor --fix
erreur : le working tree n'est pas propre : src/openapi.rs, src/state.rs — commitez, ou relancez avec --force
```

Exit status 1. Same guard as [`rbs add`](./add.md), [`rbs generate`](./generate.md) and
[`rbs upgrade`](./upgrade.md): what the repair writes has to stay distinguishable from your
own work in the next `git diff`. Commit, or pass `--force`.

The guard comes after the plan, not before it: a project with no anchor to put back has
nothing to protect, and `rbs doctor --fix` on a healthy project must be able to answer from
a working tree full of work in progress.

```text
$ rbs doctor --fix
✓ aucune ancre à reposer
```

## When it declines

A hook line that the file does not carry — or carries twice — no longer says where the
block goes. `--fix` then leaves the anchor where it is not, names it, and the check below
prints the block to paste, exactly as it did before:

```text
$ rbs doctor --fix --force
plan pour /private/tmp/rbs-demo/demo

  ~ src/seeds/main.rs   modifié

  1 fichier à écrire

✓ 1 ancre reposée : seeds
attention : layers n'a pas été reposée — la ligne d'accroche `.merge(docs)` est introuvable dans src/router.rs

  ✗ ancres      layers manque dans src/router.rs
      dans src/router.rs :
      // <rbs:layers>
      // </rbs:layers>
```

Exit status 1, since a check failed. The other anchor was still put back: an abstention is
per anchor, not for the whole run.

Declining is the point, not a shortcoming. `// <rbs:layers>` sits *inside* `trace` and
`request_id` — a `.layer()` wraps what precedes it — so a layer added at that anchor sees
the request id and its own short answers, a 429 or a refused preflight, stay in the trace.
Put the same anchor back two lines lower and none of that holds any more, and nothing would
say so until someone reads a log. An anchor put back in the wrong place costs more than an
anchor left missing.

The same applies to an anchor whose two tags did not disappear together: the tag left
behind does not say where the other one was — between them was everything the anchor held.

Under `--json`, the repair has its own object, so a script does not have to deduce from a
verdict turned green that something was written:

```text
$ rbs doctor --fix --force --json
{
  "sain": false,
  "reparation": {
    "reposees": [],
    "laissees": [
      {
        "ancre": "layers",
        "raison": "la ligne d'accroche `.merge(docs)` est introuvable dans src/router.rs"
      }
    ]
  },
  "checks": [
    {
      "name": "ancres",
      "status": "erreur",
      "detail": "layers manque dans src/router.rs",
      "remede": "dans src/router.rs :\n// <rbs:layers>\n// </rbs:layers>"
    }
  ]
}
```

`reparation` is absent without `--fix`, and the plan is never printed under `--json`:
standard output carries the document and nothing else.

## Reachable but unreadable

The two halves of the `base` check fail separately. Here the host answers on the port, but
the version could not be read because the `migration` crate did not build — the remedy
names the command to run by hand:

```text
$ rbs doctor
  ✓ ancres      les 11 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
   Compiling migration v0.1.0 (/private/tmp/rbs-demo/demo/migration)
error[E0425]: cannot find value `url_de_la_base` in this scope
  --> migration/src/main.rs:16:13
   |
16 |     let _ = url_de_la_base;
   |             ^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `migration` (bin "migration") due to 1 previous error
  ✗ base        localhost:55501 répond, mais sa version reste inconnue : la crate migration a échoué (code 101)
      vérifiez que `cargo run -p migration -- version` aboutit
  ✓ jobs        la configuration de la file est en place
attention : le projet demande votre attention
```

## Outside a project

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Exit status 1.
