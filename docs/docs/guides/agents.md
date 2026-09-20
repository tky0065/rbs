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
Codex, Cursor and Copilot read on their own. Claude Code does not: it reads `CLAUDE.md`,
and reaches `AGENTS.md` only through an import declared there. So `rbs new` also writes
`CLAUDE.md`, the one file it generates for a particular tool, and a single line long —
`@AGENTS.md` — so that the handbook keeps one source and nothing in `CLAUDE.md` can drift
from it. That file is yours from the moment it exists: add your own instructions below the
import, and no command will ever rewrite them.

## The two zones rbs owns

Only two parts of `AGENTS.md` belong to rbs, each delimited by an HTML comment:

```text
# <project> — agent handbook

<!-- rbs:guide 1.2.0 -->
… the handbook …
<!-- /rbs:guide -->

<!-- rbs:inventory -->
… the project's state …
<!-- /rbs:inventory -->

## Project notes
```

`rbs:guide` is the handbook itself — the CLI-first rule, a note on this file's own zones,
the command table, recipes, the enforced architecture, the anchor list, what rbs does not
cover, and the commands to run before concluding. Its opening marker carries the version
of the CLI that wrote it, which is what [`rbs upgrade`](../cli/upgrade.md) compares and
rewrites.

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

<!-- rbs:guide 1.2.0 -->
## CLI first
## This file
## Commands
## Recipes
## Enforced architecture
## Anchors
## What rbs does not cover
## Check before you conclude
<!-- /rbs:guide -->

<!-- rbs:inventory -->
- rbs 1.2.0 · postgres database
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
| `rbs generate job`, `rbs generate client`, `rbs migrate`, `rbs seed`, `rbs dev`, `rbs test`, `rbs routes`, `rbs openapi export` | No effect. |

`upgrade` is the only command with a mandate to bring the project back in line with the
CLI, which is why it is also the only one that recreates a deleted file. `add` and
`generate` regenerate only the inventory: they know the feature or entity they just
installed, not whether the CLI itself has moved to a new version — that comparison is
`upgrade`'s alone.

`CLAUDE.md` follows a shorter rule of its own. `rbs new` writes it; `rbs upgrade` writes it
back only when it is missing — the case of every project generated before rbs wrote it —
and no command ever rewrites one that exists, whatever it holds.

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

The output below was captured on a project generated with rbs 1.2.0: the twelve anchors,
the seven variables and the version line are that project's. What it illustrates — the
warning and the line that states its remedy — has not changed.

```text
$ rbs doctor
  ✓ ancres        les 12 points d'insertion sont en place
  ! agents        écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
  ✓ relations     les modèles portent leurs ancres de relation
  ✓ .env          les 7 variables de .env.example sont renseignées
  ✓ versions      projet et rbs-core pris d'un chemin local alignés sur le CLI 1.2.0
  … base          compilation de la crate migration, peut prendre
                  une minute au premier lancement…
  ✓ base          postgres 18.6 répond sur localhost:55502
  ✓ disposition   aucun module ne mélange les deux dispositions
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
✓ redis installée — 4 créés, 6 modifiés
```

The rest of the command still runs to completion — a missing zone in a documentation file
is never a reason to refuse installing a feature. Paste the block back, and the next
command that touches `AGENTS.md` fills it in again.

Deleting the whole file goes further still: `rbs add` and `rbs generate` finish without
even mentioning it. The only command that puts it back is [`rbs upgrade`](../cli/upgrade.md),
because restoring the project to what the current CLI expects is precisely its job:

```text
$ rbs upgrade
rbs 1.2.0 → 1.2.0

plan pour /private/tmp/rbs-demo/blog2

  · Cargo.toml   inchangé
  + AGENTS.md    créé

  1 à créer, 1 inchangé
✓ manifeste aligné sur rbs 1.2.0
```

## Reading a plan as JSON

The plan a command prints before writing is meant for a human: colours, bullets, a count
at the bottom. `rbs add`, `rbs remove`, `rbs generate crud`, `feature`, `client` and `job`,
and `rbs upgrade` take `--json`, and standard output then carries a single JSON document
instead — the plan, or the error. That is what an agent should read.

```text
rbs add cors --dry-run --json
```

`--json` and `--dry-run` are independent. With `--dry-run`, nothing is written and
`applique` is `false`; without it, the plan is applied first and the document reports
`applique: true`. A feature already installed, or a project already up to date, returns a
document whose `actions` are empty and whose `applique` is `false`: nothing was written.

What a human would still need to see goes to standard error: a missing `AGENTS.md` zone and
its block, rustfmt's warning, a required reference, generated CRUDs that `auth` leaves open,
migration notes. Success lines and hints are dropped. Standard output stays one document
from the first byte to the last, even while `generate client` compiles the project.

On the project `rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo`
creates, `rbs add cors --dry-run --json` returns this — trimmed: the real document has twelve
actions and carries the full content of every file it creates, and the root is cut:

```json
{
  "commande": "add",
  "racine": "/…/demo",
  "applique": false,
  "actions": [
    {
      "chemin": "src/modules/cors/mod.rs",
      "statut": "a_faire",
      "effet": {
        "type": "creer",
        "contenu": "use std::time::Duration;\n\nuse axum::http::{HeaderName, HeaderValue, Method};\n…"
      }
    },
    {
      "chemin": "src/router.rs",
      "statut": "a_faire",
      "effet": {
        "type": "inserer",
        "ancre": "layers",
        "lignes": [
          ".layer(crate::modules::cors::layer())"
        ]
      }
    },
    {
      "chemin": "Cargo.toml",
      "statut": "a_faire",
      "effet": {
        "type": "patcher_toml",
        "patch": {
          "type": "ajouter_dependance",
          "nom": "tower-http",
          "version": "0.7",
          "features": [
            "cors"
          ],
          "features_par_defaut": true
        }
      }
    }
  ],
  "sautees": [],
  "fichiers": {
    "crees": 4,
    "modifies": 5,
    "supprimes": 0
  }
}
```

The document lists actions, not files: a file two actions touch appears twice —
`src/modules/mod.rs` is created, then receives `pub mod cors;` in its anchor. `fichiers`
counts files, once each: created when they did not exist, modified otherwise, removed when
the plan deletes them — [`rbs remove`](../cli/remove.md) is the command whose `supprimes` is
usually not zero — unchanged ones left out. `statut` is `a_faire` when the action changes
something, `deja_fait` when the
project already carries it, `conflit` when the file exists with a content rbs did not
write — only `--force` overwrites it. `sautees` lists the insertions rbs left for you to
write, each with its `bloc` and its `cause`. `cause.type` is `fichier_absent` when the
optional file that carries the anchor is missing — a project with no compose, say;
`ancre_absente` when the file is there without the anchor — a project generated before the
anchor existed —, with `reparable` set when `rbs doctor --fix` can put it back; `entrainee`
when the insertion names what another skipped one was to declare, whose anchor `par`
gives: that one goes in first.

Each `effet` has one shape per `type`:

| `type` | Fields | Effect |
|---|---|---|
| `creer` | `contenu` | Writes a file whose content is known in full — all of it is in `contenu`. |
| `inserer` | `ancre`, `lignes` | Adds lines inside an anchor, before its closing marker. |
| `reposer_ancre` | `ancre` | Puts a missing anchor back under its hook line. |
| `patcher_toml` | `patch` | Edits `Cargo.toml`, layout preserved. `patch.type` is `inscrire_feature` (`feature`), `ajouter_dependance` (`nom`, `version`, `features`, `features_par_defaut`), `ajouter_feature_a_dependance` (`dependance`, `feature`) or `aligner_sur_version` (`dependance`, `version`). |
| `ajouter_section` | `section`, `contenu` | Appends a section to a TOML document that is not the manifest. |
| `ajouter_variable` | `cle`, `valeur`, `commentaire` | Adds a variable to an environment file; `commentaire` is `null` when there is none. |
| `remplacer_zone` | `zone`, `contenu` | Replaces the body of a zone of `AGENTS.md`, the rest of the file intact. |

A malformed command line is not a document: an unknown flag is refused by the argument
parser, in text on standard error, with exit code 2.

### Error codes

Under `--json`, a refusal is a single document on standard output, and the exit code is
that of its family — 1 here, a missing anchor being a fault of the project; see
[exit codes](../cli/doctor.md#exit-codes). rbs adds nothing to standard error for the refusal itself; what came before it stays
there — a warning printed earlier in the run, or the project's compilation under
`generate client`. Here, the `layers` anchor was removed from `src/router.rs`
before `rbs add cors --json --force`:

```json
{
  "erreur": {
    "code": "ancre_absente",
    "message": "ancre // <rbs:layers> introuvable dans src/router.rs",
    "remede": "dans src/router.rs :\n// <rbs:layers>\n// </rbs:layers>",
    "bloc": "// <rbs:layers>\n// </rbs:layers>"
  }
}
```

`message` is the French sentence a human would read, and may change from one release to
the next; `code` does not, and is what a script decides on. `remede` and `bloc` are `null`
when there is nothing to do or nothing to paste; whenever `bloc` is not `null`, `remede`
says where it goes. A code shared by several commands means the same thing in all of them.

| Code | Commands | Meaning |
|---|---|---|
| `pas_un_projet` | all | No `Cargo.toml` carrying `[package.metadata.rbs]` above the current directory. |
| `arbre_sale` | all | The Git working tree has uncommitted changes. Commit, or rerun with `--force`. |
| `fichier_inaccessible` | all | A file of the project or of a template could not be read or written. |
| `manifeste_illisible` | all | The project's `Cargo.toml` could not be read or patched. |
| `ancre_absente` | `add`, `generate`, `remove`, `upgrade` | An anchor is missing from its file. `bloc` holds the two markers to paste, `remede` names the file. |
| `ancre_mal_placee` | `add`, `generate`, `remove`, `upgrade` | An anchor sits below the line it must precede. `bloc` is the block to move up, `remede` names the line. |
| `zone_absente` | `add`, `generate`, `remove`, `upgrade` | A zone of `AGENTS.md` is missing. `bloc` holds its markers. |
| `fichier_absent` | `add`, `generate`, `remove`, `upgrade` | The file that should carry an anchor does not exist. |
| `manifeste_absent` | `add`, `generate`, `remove`, `upgrade` | The `Cargo.toml` a change targets does not exist. |
| `toml_invalide` | `add`, `generate`, `remove`, `upgrade` | A TOML document of the project does not parse. |
| `conflit` | `add`, `generate`, `generate client`, `remove`, `upgrade` | The plan would overwrite files rbs did not write. Rerun with `--force` to overwrite them. |
| `ecriture_impossible` | `add`, `generate`, `generate client`, `remove`, `upgrade` | A write failed; what the plan had already written was undone. |
| `plan_incoherent` | `add`, `generate`, `generate client`, `remove`, `upgrade` | Two actions claim to write the same file whole — a defect of rbs, to report. |
| `feature_inconnue` | `add` | No fragment carries that name. |
| `fragment_sans_manifeste` | `add` | The fragment has no `feature.toml`. |
| `fragment_invalide` | `add`, `remove` | The fragment's `feature.toml` is invalid. |
| `template_absente` | `add`, `remove` | The fragment's manifest declares a template the fragment does not carry. |
| `ancre_inconnue` | `add`, `remove` | The fragment's manifest targets an anchor rbs does not know. |
| `rendu_impossible` | `add`, `generate`, `generate job`, `generate migration`, `remove` | A template does not render. |
| `env_illisible` | `add`, `remove` | The project's `.env` is missing, unreadable, or says nothing of the database. |
| `url_indecomposable` | `add`, `remove` | The database URL cannot be split into user, password and host. |
| `fragment_inconnu` | `remove` | The name is not a fragment: a typo, or a CRUD `rbs generate crud` recorded in the very same list. |
| `feature_exigee` | `remove` | Another installed fragment still requires this one; the message names them all. |
| `migration_ambigue` | `remove` | Two migration files match the fragment's suffix; neither is picked. |
| `nom_invalide` | `generate`, `generate job`, `generate migration` | The feature name, or the one given to `--singular`, is not usable. |
| `champs_invalides` | `generate`, `generate migration` | `--fields` does not parse. |
| `feature_deja_presente` | `generate` | The feature's directory already exists. |
| `relation_invalide` | `generate` | A reference cannot be resolved: target not found, or two relations claiming the same variant. |
| `migration_absente` | `generate` | A referenced entity has no migration in the project. |
| `homonyme` | `generate` | The target model already carries a variant of that name pointing at another entity. |
| `feature_absente` | `generate` | `--has-many` repairs a feature that has to exist first. |
| `role_sans_auth` | `generate` | `--role` requires the `auth` feature. |
| `role_inconnu` | `generate` | The role is not a variant of `src/auth/model.rs`. |
| `upload_sans_storage` | `generate` | `--with-upload` requires the `storage` feature. |
| `storage_hors_modules` | `generate` | `storage` was installed before 1.3.0, under `src/storage/`. |
| `colonne_reservee` | `generate` | `--soft-delete` sets `deleted_at` itself: remove it from `--fields`. |
| `decimal_sous_sqlite` | `generate`, `generate migration` | A `decimal` field on a SQLite project: the driver binds no exact decimal. |
| `enfant_sans_cle` | `generate` | The child named by `--has-many` has no column referencing this table. |
| `ecran_occupe` | `generate` | The table's admin screen would take the demonstration screen's own file and route: rename the table, or pass `--no-admin`. |
| `champs_vides` | `generate migration` | `--fields` declares no column: the rendered migration would alter nothing. |
| `table_sans_module` | `generate migration` | No entity of the project declares that table; the message lists the ones it knows. |
| `colonne_deja_declaree` | `generate migration` | The table already carries a column of that name; the message names the file that attests it. |
| `colonne_obligatoire` | `generate migration` | A column added to a populated table has no value for the rows already there: declare the field `:optional`. |
| `unique_sur_colonne_ajoutee` | `generate migration` | `unique` on an added column: SQLite refuses it, and a generated migration must apply on all three engines. |
| `reference_interdite` | `generate migration` | A `references` field on an added column: SQLite cannot add a foreign key to an existing table. |
| `nom_reserve` | `generate job` | The name is taken: a module of the queue, `jobs` itself, or a crate the queue's code names. |
| `disposition_anterieure` | `generate job` | `jobs` or `scheduler` was installed before 1.3.0, outside `src/modules/`; the message names the move to make. |
| `jobs_absent` | `generate job` | The project has no `jobs` feature; `remede` is `rbs add jobs`. |
| `scheduler_absent` | `generate job` | `--every` requires the `scheduler` feature; `remede` is `rbs add scheduler`. |
| `cron_invalide` | `generate job` | The `--every` expression would not pass the project's startup. |
| `module_deja_declare` | `generate job` | `src/modules/jobs/mod.rs` already declares the module outside its anchor. |
| `fichier_etranger` | `generate job` | The job's file exists and does not define that job. |
| `kind_pris` | `generate job` | Another job already carries that `KIND`. |
| `echeance_existante` | `generate job` | The job already has a due date, under another expression. |
| `sans_bibliotheque` | `generate client` | The project has no `src/lib.rs`. |
| `sans_binaire_openapi` | `generate client` | The project has no `src/bin/openapi.rs`; `remede` gives the file to create. |
| `cargo_introuvable` | `generate client` | `cargo` could not be launched. |
| `projet_ne_compile_pas` | `generate client` | `cargo run --bin openapi` failed. |
| `document_illisible` | `generate client` | What the binary printed is not an OpenAPI document. |
| `client_irrendable` | `generate client` | The document does not translate into TypeScript. |
| `cli_anterieur` | `upgrade` | The project was generated by a newer rbs than this CLI. |
| `agents_illisible` | `upgrade` | `AGENTS.md` could not be rendered. |
| `makefile_illisible` | `upgrade` | The skeleton's `Makefile` could not be rendered. |
