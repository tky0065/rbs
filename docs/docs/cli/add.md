---
sidebar_position: 3
title: rbs add
---

# `rbs add`

Installs a feature into an existing project. Six are shipped: `auth`, `ci`, `docker`,
`mail`, `redis` and `storage`.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs add --help
Ajoute une feature à un projet existant : auth, ci, docker, mail, redis, storage

Usage: rbs add [OPTIONS] <FEATURE>

Arguments:
  <FEATURE>  Feature à installer

Options:
      --force                  Applique les modifications même si le working tree Git est sale
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

| Flag | Effect |
|---|---|
| `--force` | Applies even though the Git working tree is dirty, and overwrites files reported as conflicting. |
| `--template-dir <CHEMIN>` | Reads the fragments from a directory holding one subdirectory per feature, instead of the ones embedded in the binary. |
| `-y`, `--yes` | Global, and inert here: `rbs add` asks nothing. |

## The six features

| Feature | Files | Next step |
|---|---|---|
| `docker` | `.dockerignore`, `Dockerfile`, `docker-compose.yml` | `docker compose up --build` |
| `ci` | `.github/workflows/ci.yml` | `git push` |
| `auth` | eight files under `src/auth/`, one migration, and edits to four project files | copy the secret, then `rbs migrate up` |
| `redis` | three files under `src/cache/` | start a Redis at the `[cache]` URL |
| `mail` | five files under `src/mail/`, and a sample template | set `[mail]`, a local SMTP by default |
| `storage` | four files under `src/storage/` | ignore `./storage`, or switch the backend to `s3` |

The last three are the bricks of the [cache](../guides/cache.md),
[mail](../guides/mail.md) and [storage](../guides/storage.md) guides. Each mounts no
route: they arrive on your `AppState`, and what calls them is yours to write.

```text
$ rbs add docker
docker : Dockerfile multi-étapes, .dockerignore et compose de développement

plan pour /private/tmp/rbs-demo/blog

  + .dockerignore        créé
  + Dockerfile           créé
  + docker-compose.yml   créé
  ~ Cargo.toml           modifié

  4 fichiers à écrire
✓ docker installée — 3 fichiers

  docker compose up --build
```

```text
$ rbs add ci
ci : workflow GitHub Actions : fmt, clippy et tests sur PostgreSQL

plan pour /private/tmp/rbs-demo/blog

  + .github/workflows/ci.yml   créé
  ~ Cargo.toml                 modifié

  2 fichiers à écrire
✓ ci installée — 1 fichier

  git push : le workflow s'exécute à la prochaine poussée
```

```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                        créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository.rs                                 créé
  + src/auth/service.rs                                    créé
  + src/auth/controller.rs                                 créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests.rs                                      créé
  + migration/src/m20260827_152039_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/main.rs                                            modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié

  16 fichiers à écrire
✓ auth installée — 9 fichiers

  recopiez RBS_AUTH__SECRET de .env.example vers votre .env, puis rbs migrate up
```

`auth` is the one feature whose next step is not optional: the fragment writes
`RBS_AUTH__SECRET` into `.env.example` only, and a project whose `.env` does not carry
it refuses to start. The [authentication guide](../guides/auth.md) picks up there.

In each plan the `Cargo.toml` line is where the installation is recorded:

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health", "docker", "ci", "auth"]
```

Anything else is refused with the list of what is installable:

```text
$ rbs add graphql
erreur : `graphql` n'est pas une feature installable : auth, ci, docker, mail, redis, storage
```

## Idempotence

Installing something already installed is not a failure. The manifest is what the command
reads: a feature listed in `[package.metadata.rbs]` short-circuits before a plan is even
drawn.

```text
$ rbs add docker
✓ docker est déjà installée — rien à faire
```

Idempotence rests on those metadata, not on the presence of the files. Remove the feature
from the manifest and the files are still there — the plan reports them unchanged, and
writes back only the manifest line that went missing:

```text
$ rbs add docker
docker : Dockerfile multi-étapes, .dockerignore et compose de développement

plan pour /private/tmp/rbs-demo/blog

  · .dockerignore        inchangé
  · Dockerfile           inchangé
  · docker-compose.yml   inchangé
  ~ Cargo.toml           modifié

  1 fichier à écrire, 3 inchangés
✓ docker installée — 3 fichiers

  docker compose up --build
```

Markers in the plan read: `+` created, `~` modified, `·` unchanged, `!` conflicting.

## A dirty working tree

`rbs add` edits `Cargo.toml`, so it refuses to run over uncommitted changes:

```text
$ rbs add ci
erreur : le working tree n'est pas propre : Cargo.toml — commitez, ou relancez avec --force
```

Untracked files are not counted: they are exactly what the command is about to create. Past
five names the list is abbreviated. `--force` runs anyway, which is what the message
suggests.

## Conflicts

A file that exists with content the fragment does not match is neither merged nor silently
overwritten. The plan marks it `!`, and the command stops:

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features
plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  · Cargo.toml   inchangé

  1 inchangé, 1 en conflit
erreur : Dockerfile — relancer avec --force pour les écraser
```

`--force` overwrites, having shown the same plan first:

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features --force
plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  · Cargo.toml   inchangé

  1 inchangé, 1 en conflit
✓ docker installée — 1 fichier

  docker compose up --build

$ cat Dockerfile
FROM scratch
```

Should applying the plan fail halfway, what has already been written is restored: a partial
install leaves no half-installed feature behind.

## Templates from disk

`--template-dir` expects a directory holding one subdirectory per feature — `docker/`,
`ci/`, whatever you add — each with its `.jinja` templates, the suffix stripped on output.
It replaces the embedded catalogue rather than adding to it, so a directory that does not
hold the requested feature is a directory where no feature exists:

```text
$ rbs add docker --template-dir /nexistepas
erreur : `docker` n'est pas une feature installable : aucune n'est disponible
```

That is also why an empty catalogue is refused here rather than at render time: it would
otherwise produce an empty plan, and a command that succeeds without doing anything.

## Anchors

`rbs add` writes whole files and edits the manifest; it is [`rbs
generate`](./generate.md#anchors) that inserts into the project's seven comment anchors —
`// <rbs:features>`, `// <rbs:routes>`, `// <rbs:openapi>`, `// <rbs:migration_modules>`,
`// <rbs:migrations>`, `// <rbs:state_champs>` and `// <rbs:state_init>`. The rule is the
same for both commands: no AST is ever rewritten, and a missing anchor makes the command
write nothing and print the block to paste back. [`rbs doctor`](./doctor.md) checks all
seven.

## Failures

Outside a project:

```text
$ rbs add docker
erreur : aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`
```

Each of these exits with status 1.
