---
sidebar_position: 3.5
title: rbs remove
---

# `rbs remove`

Uninstalls a feature [`rbs add`](./add.md) installed: its files, the lines it inserted at
each anchor, its migration, and — where no other installed feature still claims them —
its dependencies. It reverses the fragment's manifest section by section, in the opposite
order from the one that installed it, and follows the same two rules `add` does: no AST is
ever rewritten, and nothing is written until the whole plan is known to succeed.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

{/* rbs:transcript cmd="rbs remove --help" */}
```text
$ rbs remove --help
Retire une feature installée : ses fichiers, ses ancres, sa migration et ses dépendances

Utilisation : rbs remove [OPTIONS] <FEATURE>

Arguments :
  <FEATURE>  Feature à retirer

Options :
      --force                  Retire même si un fichier a été modifié, ou si le working tree Git est sale
      --dry-run                Affiche le plan sans rien écrire
      --json                   Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -h, --help                   Affiche l'aide
  -V, --version                Affiche la version
```

| Flag | Effect |
|---|---|
| `--force` | Removes even though a file has diverged from a fresh render of the fragment, or the Git working tree is dirty. |
| `--dry-run` | Prints the plan and stops. Nothing is written. |
| `--json` | Prints the plan — or the error — as one JSON document on standard output instead of the coloured text. [The agents guide](../guides/agents.md#reading-a-plan-as-json) has the document and the error codes. |
| `--template-dir <CHEMIN>` | Reads the fragment's manifest from a directory holding one subdirectory per feature, instead of the ones embedded in the binary — the same directory `add` would have installed from. |

Without `--template-dir`, the same fifteen names `add` installs are the only ones
`remove` accepts. A CRUD `rbs generate crud` wrote is not one of them, even though its
name sits in `[package.metadata.rbs] features` next to the real fragments — `remove`
refuses it exactly as it refuses a name that was never a feature at all.

## Removing a feature

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" */}
```text
$ rbs remove cors
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  - src/modules/cors/config.rs   supprimé
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 3 à supprimer
✓ cors retirée — 5 modifiés, 3 supprimés
  tower-http appartient au squelette, jamais retirée

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

Markers in the plan join the three [`add`](./add.md#idempotence) already uses: `-` removed,
`~` modified, `·` unchanged, `!` conflicting. The plan reconstructs the exact files `add`
had written, so it can tell a line the fragment itself inserted from one
the developer added next to it, and remove only the former; `src/modules/mod.rs` here loses
its `pub mod cors;` but keeps whatever else a later fragment mounted there.

`tower-http` is named rather than removed because it belongs to the skeleton itself — every
project depends on it before any feature is installed — one of the things
[never removed](#what-is-never-removed) below.

## The schema keeps its tables

A fragment's migration is retrieved by its file suffix, since no manifest keeps the
timestamp a migration was created with, and removed the same way any other file is:

{/* rbs:transcript cmd="rbs remove jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add jobs && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m jobs" dans="demo" extrait="oui" */}
```text
$ rbs remove jobs
  - migration/src/m20260917_141948_create_jobs.rs   supprimé
  ~ migration/src/lib.rs                            modifié

✓ jobs retirée — 6 modifiés, 14 supprimés

  la migration est retirée du projet, mais le schéma garde ses tables : `rbs migrate down` devait passer avant

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

`remove` never opens a connection: it edits the project's files, nothing in its database.
Dropping the tables a migration created is `rbs migrate down`'s job, and it has to run
**before** the removal — once the migration file is gone, `sea-orm-cli` has nothing left to
read to write the `DOWN` half. A schema left behind this way is invisible to
[`rbs doctor`](./doctor.md): none of its checks ever query the database for which
migrations have actually been applied there — `base` only confirms that the driver
compiled into the project matches the URL's scheme, that a connection answers within
three seconds, and the server's version.

A second migration matching the same suffix — a rename, a hand-made copy — is refused
rather than guessed at: `remove` will not pick one of two candidates on the developer's
behalf.

## Fragments that still need it

{/* rbs:transcript cmd="rbs remove mail" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add auth && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m auth && rbs add webhooks && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m webhooks" dans="demo" */}
```text
$ rbs remove mail
erreur : `mail` est encore exigée par auth, webhooks : retirez-les d'abord, dans l'ordre de votre choix
```

`webhooks` does not itself require `mail` — it requires `auth`, which requires `mail`. The
dependants are named by transitive closure, to a point fixed rather than one hop: naming
only `auth` here would have the developer remove it, then hit a second, identical refusal
against `webhooks` that this command already knew was coming. Nothing is written while
either dependant is still installed; remove them first, in whichever order suits you.

## What is never removed

Five things a removal leaves untouched, all named in the report rather than acted on:

{/* rbs:transcript cmd="rbs remove docker" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add docker && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m docker" dans="demo" */}
```text
$ rbs remove docker
docker : Dockerfile multi-étapes, .dockerignore et services de déploiement

plan pour …/demo

  - Dockerfile           supprimé
  - .dockerignore        supprimé
  ~ docker-compose.yml   modifié
  ~ Makefile             modifié
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  4 à modifier, 2 à supprimer
✓ docker retirée — 4 modifiés, 2 supprimés
  docker-compose.yml n'est pas retiré : posé seulement s'il manquait, le retrait ne peut pas savoir si ce fragment en est l'auteur
  config/production.toml n'est pas retiré : posé seulement s'il manquait, le retrait ne peut pas savoir si ce fragment en est l'auteur
  POSTGRES_USER n'est pas retirée de .env, à faire à la main si elle ne sert plus
  POSTGRES_PASSWORD n'est pas retirée de .env, à faire à la main si elle ne sert plus
  POSTGRES_DB n'est pas retirée de .env, à faire à la main si elle ne sert plus

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

- **Files written only when they were missing.** `docker`'s manifest marks
  `docker-compose.yml` and `config/production.toml` this way: `add` writes them only on a
  project that had none, so a project that already carried one keeps it as it found it. A
  fragment written this way never claims authorship, and removal has no way to tell whether
  this install wrote the file or merely found it — `mail` and `redis` both insert their own
  service into the very same compose, and deleting it here would carry their work away too.
- **Environment variables.** `.env` is gitignored — the one write in a removal that no
  `git checkout` would ever undo — so the decision to drop a line from it is left to the
  developer, never taken for them. `.env.example`, versioned, is not touched either: it
  still documents the variable for whoever reads the project next.
- **Dependencies another installed fragment still declares.** Every other installed
  fragment's manifest is read before planning a single removal, so a crate two fragments
  both need survives as long as either does.
- **A dependency's feature another installed fragment still needs**, distinct from the
  crate itself: `auth`, `scheduler`, `rate-limit` and `redis` all turn on tokio's `time`
  feature, so removing one of them while another is still installed leaves it on.
- **Dependencies the skeleton itself declares**, `tower-http` above being one: `rbs new`
  puts them in `Cargo.toml` before any feature exists, and no removal claims them.

## An unknown name

{/* rbs:transcript cmd="rbs remove graphql" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs remove graphql
erreur : `graphql` n'est pas un fragment : api-keys, audit, auth, ci, cors, docker, frontend, frontend-admin, jobs, mail, observability, rate-limit, redis, scheduler, storage, webhooks
```

Checked before the project's manifest is even read: a name that was never a fragment does
not become "already absent" for having been typed on a project that never installed it —
it stays refused.

## Idempotence

Removing something not installed is not a failure — the same rule [`add`](./add.md#idempotence)
follows, mirrored:

{/* rbs:transcript cmd="rbs remove docker" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs remove docker
✓ docker n'est pas installée — rien à faire
```

`[package.metadata.rbs] features` is what the command reads, exactly as `add` does: a name
absent from that list has nothing planned against it, whatever files happen to sit on disk.

## A dirty working tree

`remove` edits `Cargo.toml`, so — like `add` — it refuses to run over uncommitted changes:

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors && rbs add ci" dans="demo" */}
```text
$ rbs remove cors
erreur : le working tree n'est pas propre : AGENTS.md, Cargo.toml — commitez, ou relancez avec --force
```

Untracked files are not counted: nothing here is about to create one. `--force` runs
anyway.

## Conflicts

A file the fragment would remove but whose content no longer matches what a fresh render of
it produces is neither deleted nor silently left in place. The plan marks it `!`, and the
command stops, exactly as `add`'s own conflict does for a file it would overwrite:

{/* rbs:libre raison="exige de modifier à la main src/modules/cors/config.rs, ce que le rejeu ne sait pas faire sans shell" */}
```text
$ rbs remove cors
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  ! src/modules/cors/config.rs   conflit — relancer avec --force
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 2 à supprimer, 1 en conflit
erreur : src/modules/cors/config.rs — relancer avec --force pour les écraser
```

This is what protects a hand-edited file from disappearing without a trace: `remove`
recomputes what `add` would have written today and compares it to what is actually on
disk, byte for byte. Anything that diverges — a line added, a value changed — marks that
one file `!`, but withholds the whole plan: without `--force`, nothing is written at
all — not `config.rs`, not the two deletions, not the five edits around them.
`--force` writes the whole plan anyway, `config.rs` included, the same plan shown first:

{/* rbs:libre raison="exige la même modification à la main de src/modules/cors/config.rs" */}
```text
$ rbs remove cors --force
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  ! src/modules/cors/config.rs   conflit — relancer avec --force
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 2 à supprimer, 1 en conflit
✓ cors retirée — 5 modifiés, 3 supprimés
  tower-http appartient au squelette, jamais retirée

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

Both blocks above are captured for real, on a project where `config.rs` was hand-edited
after `cors` was installed — what's common with `add`'s own conflict is the missing
guard, not the scenario it comes from. Neither carries a `{/* rbs:transcript */}` marker:
these two blocks come from a scenario the automatic capture cannot replay — appending a
line to a file Git already tracks.

A conflict does not always mean the file was edited. `remove` renders the fragment as it
would render *today*, and some fragments do not render the same way on every project:
`rate-limit`'s counter is written against Redis when the project carries `redis`, in memory
otherwise. Installing `rate-limit`, then `redis` — nothing makes the two arrive together,
`rate-limit` requiring no other feature — leaves `rate-limit`'s own files diverging from a
fresh render, and a removal marks them `!` though no one has touched them. `--force` is the
answer there too.

## The compiler is the oracle

`remove` never searches the project's own code for a reference to the feature it just took
out — a call to `webhooks::emit`, a `mail::Service` still held on `AppState`, a `use`
left dangling. It only knows what the fragment itself declared: its files, its anchor
lines, its migration, its dependencies. Everything the developer wrote *against* the
feature is invisible to it, and that is exactly what `cargo build` is for — the line every
successful removal prints:

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" extrait="oui" */}
```text
  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

The compiler names every remaining reference, one error at a time, precisely — there is no
static analysis in `remove` to get partially right instead.

## Reading a plan as JSON

`remove` is one of the commands [`--json` reads](../guides/agents.md#reading-a-plan-as-json)
as a single document instead of coloured text. `fichiers` carries a third counter next to
`add`'s two — `supprimes`, for what a removal actually does most of:

{/* rbs:transcript cmd="rbs remove cors --dry-run --json" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" extrait="oui" */}
```text
$ rbs remove cors --dry-run --json
  "fichiers": {
    "crees": 0,
    "modifies": 5,
    "supprimes": 3
  }
```

What a human would still need to see — the migration warning, what was left in place —
goes to standard error, exactly as it does for a warning `add` prints: the JSON document on
standard output carries the plan alone.

## Failures

Outside a project:

{/* rbs:transcript cmd="rbs remove cors" */}
```text
$ rbs remove cors
erreur : aucun projet rbs ici : `rbs remove` s'exécute dans un projet créé par `rbs new`
```

Exit status 2: the call is what needs correcting — see [exit codes](./doctor.md#exit-codes).

A project that has just had a feature removed diagnoses clean: [`rbs doctor`](./doctor.md)
reads the same manifest this command writes to, and the `AGENTS.md` inventory is refreshed
in the very same plan — a removal never leaves the two disagreeing about what the project
still carries.
