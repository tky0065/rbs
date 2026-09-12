# `--singular` et les invariables de la singularisation — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs generate crud news` ne produit plus `CreateNew` ni `let new = …`, et l'utilisateur peut imposer la forme singulière quand l'heuristique se trompe sur un mot que la liste ne connaît pas.

**Architecture:** Deux verrous, du plus simple au plus général. `to_singular` consulte une liste `INVARIABLES` de mots entiers — `news`, `series`, `species` — avant de retirer un `s` final ; elle sert aussi aux variantes de relation (`relations.rs`). `Feature` reçoit un champ `singular: Option<String>` posé par `.with_singular(Option<String>)` ; `singular()` le rend quand il est là et retombe sur `to_singular` sinon, si bien que `entity()`, la sérialisation vers les templates et tout ce qui en dérive suivent sans autre changement. Le flag `--singular <NOM>` existe sur `crud` et sur `feature`, les deux passant par `Feature::fresh` ; sa valeur est validée par `name::validate` (snake_case, ni mot-clé Rust, ni module du squelette) avant tout rendu, car elle devient un nom de type et de variable.

**Tech Stack:** clap (derive), minijinja (`singular` déjà sérialisé), tests unitaires sans Docker ; `integration_docs` rejoue les deux transcriptions `--help` de la page `generate`.

## Global Constraints

- Aucune template touchée, aucun exemple n'emploie `news` ni `--singular` : `integration_examples` n'a pas à être relancé.
- Documentation bilingue dans le même commit : transcriptions régénérées depuis la sortie réelle du binaire, ligne `--singular` dans les deux tableaux de flags.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: les invariables

**Files:**
- Modify: `crates/rbs-cli/src/generate/feature.rs`

- [x] **Step 1 : test rouge** — `an_invariable_plural_keeps_its_s` : `news`, `series`, `species`, et `latest_news` (dernier mot seul).
- [x] **Step 2 : `INVARIABLES`** consultée avant le retrait du `s`.

### Task 2: `--singular` de bout en bout

**Files:**
- Modify: `crates/rbs-cli/src/generate/feature.rs` (`singular: Option<String>`, `with_singular`, `singular()`)
- Modify: `crates/rbs-cli/src/generate/command.rs` (`Options.singular`, validation, construction de `Feature`)
- Modify: `crates/rbs-cli/src/lib.rs` (`GenerateArgs.singular`, les deux sous-commandes)
- Modify: `crates/rbs-cli/src/cli.rs` (flag sur `Crud` et `Feature`, tests de parsing)

- [x] **Step 1 : tests rouges** — `feature.rs` : `with_singular(Some("news_item"))` rend `entity() == "NewsItem"` et sérialise `singular == "news_item"` ; `cli.rs` : `generate crud news --singular news_item` et `generate feature news --singular news_item` sont acceptés ; `command.rs` : un `--singular` qui n'est pas en snake_case est refusé avant écriture.
- [x] **Step 2 : implémentation** dans les quatre fichiers.

### Task 3: documentation

**Files:**
- Modify: `docs/docs/cli/generate.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/generate.md`

- [x] **Step 1 : transcriptions** `rbs generate crud --help` et `rbs generate feature --help` régénérées depuis `cargo run -q -p rbs-cli --bin rbs -- …`.
- [x] **Step 2 : ligne `--singular`** dans le tableau des flags de `crud`, et la phrase de `feature` qui énumère les flags.
- [x] **Step 3 : vérifications** — `cargo test -p rbs-cli --lib`, `--test integration_docs`, clippy, fmt.
