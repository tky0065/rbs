---
sidebar_position: 13
title: AGENTS.md
---

# AGENTS.md

An agent dropped into an rbs project has no way to know rbs exists. It sees Rust files
and writes Rust files: it hand-writes the six files of a feature, forgets the migration,
skips the anchors, and breaks the one-directional dependency the architecture relies on.
The CLI is right there, and nothing tells the agent to reach for it.

`rbs new` answers this by writing `AGENTS.md` at the project root — the manual for rbs,
written for an agent rather than a human. `AGENTS.md` is a plain, tool-neutral format that
Claude Code, Codex, Cursor and Copilot already read on their own; rbs generates no file
tied to a particular tool.

## The two zones rbs owns

Only two parts of `AGENTS.md` belong to rbs, each delimited by an HTML comment:

```text
# <project> — agent handbook

<!-- rbs:guide 1.1.0 -->
… the handbook …
<!-- /rbs:guide -->

<!-- rbs:inventory -->
… the project's state …
<!-- /rbs:inventory -->

## Project notes
```

`rbs:guide` is the handbook itself — the CLI-first rule, the command table, recipes,
the enforced architecture, the anchor list, what rbs does not cover, and the commands to
run before concluding. Its opening marker carries the version of the CLI that wrote it,
which is what [`rbs upgrade`](../cli/upgrade.md) compares and rewrites.

`rbs:inventory` is the project's own state, recomputed from scratch on every write: the
rbs version and database engine, the fragments installed, the entities generated, and the
anchors the project actually carries. It stays short and factual on purpose, so an agent
does not have to explore the tree to learn what it already contains.

**Everything outside these two zones belongs to you, and rbs never rewrites it** — not the
title, not the `## Project notes` section `rbs new` leaves empty, not a heading you add of
your own. The same rule already governs the code rbs generates: this file is meant to be
edited, and the markers are the only promise rbs makes about what it will touch.

A real project's zones, generated in English, read like this:

```text
# blog — agent handbook

<!-- rbs:guide 1.1.0 -->
## CLI first
## Commands
## Recipes
## Enforced architecture
## Anchors
## What rbs does not cover
## Check before you conclude
<!-- /rbs:guide -->

<!-- rbs:inventory -->
- rbs 1.1.0 · postgres database
- Fragments installed: none
- Generated entities: none
- Project anchors: features (src/lib.rs), routes (src/router.rs), openapi (src/openapi.rs), migration_modules (migration/src/lib.rs), migrations (migration/src/lib.rs), state_champs (src/state.rs), state_init (src/state.rs), startup (src/main.rs), seeds (src/seeds/main.rs), services (docker-compose.yml)
<!-- /rbs:inventory -->

## Project notes
```

## Who writes what

| Command | Effect on `AGENTS.md` |
|---|---|
| `rbs new` | Writes the whole file: guide, inventory, title, and an empty notes section. |
| `rbs add <feature>` | Regenerates the inventory zone. |
| `rbs generate crud\|feature` | Regenerates the inventory zone. |
| `rbs upgrade` | Regenerates both the guide and the inventory; recreates the file if it went missing. |
| `rbs doctor` | Changes nothing — it only reports. |
| `rbs migrate`, `rbs seed`, `rbs dev` | No effect. |

`upgrade` is the only command with a mandate to bring the project back in line with the
CLI, which is why it is also the only one that recreates a deleted file. `add` and
`generate` regenerate only the inventory: they know the feature or entity they just
installed, not whether the CLI itself has moved to a new version — that comparison is
`upgrade`'s alone.

## Choosing the language

`rbs new --lang fr|en` picks the language the handbook is written in. Without the flag,
rbs falls back to the environment: `LC_ALL` first, then `LANG` — a value starting with
`fr` gives French, any other non-empty value gives English, and no value at all gives
French, the language of the rbs repository itself.

The choice is recorded in the manifest, not re-derived on every command:

```toml
[package.metadata.rbs]
lang = "en"
```

Without this key, [`rbs add`](../cli/add.md) and [`rbs upgrade`](../cli/upgrade.md) would
have to guess the project's language from the environment of whoever happens to run them
— rewriting an English guide into French the day someone on the team runs the command from
a French locale. Reading it from the manifest instead means the file stays in the language
the project was created in, independent of who touches it next.

## What `rbs doctor` checks

[`rbs doctor`](../cli/doctor.md) runs an `agents` check alongside its others:

| It finds | Verdict |
|---|---|
| `AGENTS.md` missing | failure — `rbs upgrade` recreates it |
| The `rbs:guide` or `rbs:inventory` zone missing | failure — the block to paste is shown |
| The guide's version different from the CLI's | failure — `rbs upgrade` rewrites the guide |
| The rendered inventory different from the one on disk | failure — `rbs upgrade` recomputes it |
| A feature declared in the manifest with no matching `src/<name>/` | failure — `rbs add <name>`, or drop the line from the manifest |
| A directory under `src/` that no fragment and no declared feature accounts for | **warning** |

That last line is the CLI-first rule made checkable: a directory nothing in the manifest
explains is code nobody generated. It stays a warning rather than a failure on purpose —
writing by hand what rbs does not cover is legitimate and expected, the very point of the
"what rbs does not cover" section of the guide. Turning that into a failure would make
`rbs doctor` red on a perfectly healthy project the moment someone adds a webhook handler
or an external HTTP client by hand, which is exactly the kind of code this tool has no
business generating.

A warning does not change the exit status or the final verdict: a project with nothing
but a warning still exits 0 and is still reported as healthy overall — only an actual
failure does that.

```text
$ rbs doctor
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running `target/debug/migration version`
  ✓ ancres      les 10 points d'insertion sont en place
  ! agents      écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 4 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  ✓ base        postgres 18.6 répond sur localhost:55502
✓ le projet est sain
```

## When a zone goes missing

Deleting a marker is treated the same way as deleting one of the code anchors: the
command that would have written there writes nothing, and shows the exact block to paste
back instead.

```text
$ rbs add redis
[…]
attention : AGENTS.md ne porte pas la zone `rbs:inventory` — collez ce bloc pour la rétablir :

<!-- rbs:inventory -->
<!-- /rbs:inventory -->
✓ redis installée — 3 fichiers
```

The rest of the command still runs to completion — a missing zone in a documentation file
is never a reason to refuse installing a feature. Paste the block back, and the next
command that touches `AGENTS.md` fills it in again.

Deleting the whole file goes further still: `rbs add` and `rbs generate` finish without
even mentioning it. The only command that puts it back is [`rbs upgrade`](../cli/upgrade.md),
because restoring the project to what the current CLI expects is precisely its job:

```text
$ rbs upgrade
rbs 1.1.0 → 1.1.0

plan pour /private/tmp/rbs-demo/blog2

  · Cargo.toml   inchangé
  + AGENTS.md    créé

  1 fichier à écrire, 1 inchangé
✓ manifeste aligné sur rbs 1.1.0
```
