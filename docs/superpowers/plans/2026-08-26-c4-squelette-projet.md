# Squelette de projet généré — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** poser sous `templates/project/` les fichiers que `rbs new` déroulera, avec leurs quatre ancres, et prouver ce qui est prouvable avant que la commande n'existe.

**Architecture:** un fichier de template par fichier généré, tous suffixés `.jinja`, le chemin de sortie étant le chemin de la template privé du suffixe. Le suffixe n'est pas cosmétique : un `.gitignore` réellement nommé ainsi retirerait des fichiers du suivi du dépôt rbs, un `Cargo.toml` serait découvert par Cargo, et rust-analyzer tenterait d'analyser des `.rs` qui portent des `{@ … @}`.

Trois variables seulement : `nom_projet`, `nom_crate`, `rbs_core_dep`. La dernière existe parce que `rbs-core` n'est pas publié : écrire `rbs-core = "0.1"` en dur rendrait tout projet généré incompilable, donc les critères des tâches d'intégration inatteignables.

Les quatre ancres sont l'interface que consommeront les commandes de génération : `<rbs:features>` dans `src/features/mod.rs`, `<rbs:routes>` dans `src/router.rs`, `<rbs:openapi>` dans `src/openapi.rs`, `<rbs:migrations>` dans `migration/src/lib.rs`. `src/openapi.rs` est un fichier à part plutôt qu'une section de `router.rs` : c'est `router.rs` que le développeur rouvre le plus souvent, et l'alourdir d'un `#[derive(OpenApi)]` le rendrait moins lisible.

Ce que la tâche ne peut pas prouver : que le projet généré compile. Aucun rendu complet n'est possible avant que `rbs new` existe. Le test se limite donc à ce qu'un fichier de template doit garantir en permanence — nommage, ancres, rendu sans variable manquante.

**Tech Stack:** minijinja 2 (délimiteurs `{@ @}`), le `Renderer` du CLI.

**Spec:** `docs/superpowers/specs/2026-08-26-squelette-projet-design.md`, et `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.3, §3.4, §4.5, §5.4

## Global Constraints

- Aucun fichier de `crates/rbs-core/` n'est modifié : la section `[docs]` de `Config` est écrite ailleurs, et les templates l'utilisent comme si elle existait.
- Aucune commande du CLI n'est implémentée, et les templates ne sont pas encore embarquées dans le binaire.
- Versions des dépendances du projet généré figées en dur, alignées sur le `[workspace.dependencies]` du dépôt.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : validation des templates

**Files:**
- Create: `crates/rbs-cli/src/templates.rs`
- Modify: `crates/rbs-cli/src/main.rs` (déclaration du module)

- [ ] **Step 1 : écrire les tests d'abord**

Quatre tests lisant `templates/project/` depuis `CARGO_MANIFEST_DIR` : chaque fichier porte le suffixe `.jinja` ; chacune des quatre ancres est présente une fois ouverte et une fois refermée, dans le fichier attendu et dans cet ordre ; chaque template se rend avec un contexte portant les trois variables — `UndefinedBehavior::Strict` fait de ce test la preuve qu'aucune variable non déclarée ne traîne ; le rendu du manifeste produit `name = "mon-api"`.

Run: `cargo test -p rbs-cli`
Expected: échec — le répertoire `templates/project/` n'existe pas.

---

### Task 2 : les templates

**Files:**
- Create: `templates/project/Cargo.toml.jinja`, `.env.example.jinja`, `.gitignore.jinja`
- Create: `templates/project/config/{default,development}.toml.jinja`
- Create: `templates/project/src/{main,router,state,openapi}.rs.jinja`
- Create: `templates/project/src/features/mod.rs.jinja`
- Create: `templates/project/src/features/health/{mod,controller}.rs.jinja`
- Create: `templates/project/migration/Cargo.toml.jinja`, `migration/src/lib.rs.jinja`

- [ ] **Step 1 : la racine du projet**

Manifeste avec `[workspace] members = ["migration"]`, `[package.metadata.rbs]` portant `health` comme seule feature installée, et les dépendances figées. `.env.example` et `.gitignore`.

- [ ] **Step 2 : la configuration**

`config/default.toml` porte les réglages du pool et la section `[docs]` ; `config/development.toml` ne porte que ses surcharges.

- [ ] **Step 3 : le corps de l'application**

`main.rs` en une vingtaine de lignes — logs, configuration, base, routeur, écoute, dans cet ordre. `state.rs` porte l'`AppState` du projet, qui délègue à `CoreState`. `router.rs` monte les features puis les deux middlewares. `openapi.rs` porte le document et ne monte Swagger UI et le JSON que si la configuration les autorise.

- [ ] **Step 4 : la feature vitrine et la crate de migration**

`features/health/` en deux fichiers : la route est montée par le projet, la logique reste dans `rbs-core`. `migration/` est une crate SeaORM standard dont le vecteur de migrations est vide.

- [ ] **Step 5 : prouver le critère**

Run: `cargo test -p rbs-cli`
Expected: tout vert, dont les quatre tests de templates.

---

### Task 3 : vérification et commit

- [ ] **Step 1 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 2 : commit**

Message : `feat(cli): ajoute le squelette des projets générés`, corps portant le *pourquoi* du suffixe `.jinja` et des ancres, puis un intertitre `Vérifications :`.
