---
sidebar_position: 7
title: rbs seed
---

# `rbs seed`

Inserts the project's demonstration data by running its `seed` binary. What the seeds
contain, and how they are written, is the [seeds guide](../guides/seeds.md); this page is
the command.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs seed --help
Insère les données de démonstration du projet

Usage: rbs seed [OPTIONS]

Options:
      --force    Insère même sous RBS_ENV=production
  -h, --help     Print help
  -V, --version  Print version
```

Like [`rbs migrate`](./migrate.md), the command wraps a binary of the project rather than
talking to the database itself. rbs gains no SQL client, and the code that inserts stays
where you can read and edit it.

## Running the seeds

```text
$ rbs seed
subscribers : inséré
✓ seeds insérés
```

One line per seed, in the order of the `<rbs:seeds>` anchor, then a summary.

## Nothing to insert

```text
$ rbs seed
✓ aucun seed déclaré — rien à insérer
```

Exit code 0, and cargo is never started. This is not a failure: a project that has
generated no CRUD yet has nothing to seed, and saying so costs nothing. The absence of a
compilation is what makes it instant — and, incidentally, what a test asserts by measuring
the command's duration.

A project whose `src/seeds/` is missing altogether — one created before the directory
existed — gets a message naming the file to create and the `[[bin]]` block to add, rather
than the manifest error cargo would have produced.

## The production refusal

```text
$ RBS_ENV=production rbs seed
erreur : RBS_ENV=production : les seeds sont des données de démonstration, et rbs refuse de les insérer en production — relancez avec --force si c'est bien ce que vous voulez
```

Exit code 1, and the project's binary is **not** launched — cargo does not even start.

The guard lives in the command and not in the generated code, and that is deliberate. A
seed is a file you are meant to edit; a refusal sitting inside it is a refusal you can
delete by accident while rewriting the code around it. Here, nothing you do to your seeds
can remove it.

`--force` is the way through. It has to be typed, which is the whole idea.

:::warning
`RBS_ENV` is read from the environment, not from the project. A shell where it is exported
from an earlier command will refuse, and a production shell where it was never set will
not. The variable is the same one that selects `config/production.toml` — see the
[configuration guide](../guides/configuration.md).
:::

## Failures

| Situation | What happens |
|---|---|
| Not in a project | Refusal naming what it looked for |
| No `src/seeds/` | Message naming the file to create and its `[[bin]]` block |
| No seed declared | `✓`, exit code 0, cargo not started |
| `RBS_ENV=production` without `--force` | Refusal naming `--force`, binary not launched |
| Database unreachable | The seeds binary pings before the first insert and says so |
| A seed fails | The binary's own error; earlier seeds have already been inserted |

That last line is worth reading twice: the seeds are not wrapped in one transaction. A
failure halfway leaves the rows already inserted in place.
