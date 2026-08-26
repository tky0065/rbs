---
sidebar_position: 3
title: rbs add
---

# `rbs add`

Installs a feature into an existing project. Two are shipped in 0.1.0: `docker` and `ci`.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs add --help
Ajoute une feature à un projet existant : docker, ci

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

## The two features

| Feature | Files | Next step |
|---|---|---|
| `docker` | `.dockerignore`, `Dockerfile`, `docker-compose.yml` | `docker compose up --build` |
| `ci` | `.github/workflows/ci.yml` | `git push` |

```text
$ rbs add docker
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
plan pour /private/tmp/rbs-demo/blog

  + .github/workflows/ci.yml   créé
  ~ Cargo.toml                 modifié

  2 fichiers à écrire
✓ ci installée — 1 fichier

  git push : le workflow s'exécute à la prochaine poussée
```

The `Cargo.toml` line is the fourth, respectively second, file of the plan: the manifest is
where the installation is recorded.

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health", "articles", "comments", "docker", "ci"]
```

Anything else is refused with the list of what is installable:

```text
$ rbs add graphql
erreur : `graphql` n'est pas une feature installable : ci, docker
```

## Idempotence

Run it a second time and the plan reports every file unchanged rather than writing them
again. The command still succeeds — installing something already installed is not a
failure — and it says nothing was touched:

```text
$ rbs add docker
plan pour /private/tmp/rbs-demo/blog

  · .dockerignore        inchangé
  · Dockerfile           inchangé
  · docker-compose.yml   inchangé
  · Cargo.toml           inchangé

  4 inchangés
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
generate`](./generate.md#anchors) that inserts into the project's five comment anchors —
`// <rbs:features>`, `// <rbs:routes>`, `// <rbs:openapi>`, `// <rbs:migration_modules>`
and `// <rbs:migrations>`. The rule is the same for both commands: no AST is ever
rewritten, and a missing anchor makes the command write nothing and print the block to
paste back. [`rbs doctor`](./doctor.md) checks all five.

## Failures

Outside a project:

```text
$ rbs add docker
erreur : aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`
```

Each of these exits with status 1.
