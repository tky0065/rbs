# Moteur de rendu — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** rendre une template minijinja sans qu'aucun des langages générés — Rust, TOML, YAML, shell — n'ait à être échappé au cas par cas.

**Architecture:** un fichier, `template.rs`, portant une struct `Renderer` qui possède l'`Environment`. L'environnement est construit une fois : c'est lui qui porte les trois réglages non par défaut, et un environnement par appel les laisserait diverger.

Les délimiteurs de variables passent de `{{ }}` à `{@ @}`. Le choix est contraint par ce que rbs génère : `{{ }}` casse sur les `format!("{{}}")` de Rust, `${ }` sur les `${VAR}` de docker-compose et les `${{ secrets.X }}` de GitHub Actions, `[[ ]]` sur les `[[bin]]` des Cargo.toml, `<< >>` sur les heredocs `<<'EOF'`. `{@ @}` n'entre en collision avec rien de tout cela. Les blocs `{% %}` et les commentaires `{# #}` restent inchangés : ils n'ont jamais posé le problème.

Deux autres réglages s'écartent des défauts. `UndefinedBehavior::Strict`, parce qu'une variable oubliée doit faire échouer le rendu plutôt que laisser un trou silencieux dans un fichier généré que l'utilisateur découvrira à la compilation. Et l'auto-échappement désactivé, parce qu'on génère du Rust et du TOML : `&` et `<` doivent ressortir tels quels.

`SyntaxConfigBuilder` est derrière la feature `custom_syntax` de minijinja, absente par défaut.

**Tech Stack:** minijinja 2 (`custom_syntax`), serde.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.3

## Global Constraints

- Aucun chargement de template depuis le disque ni depuis `include_dir` : le moteur ne connaît que des sources en mémoire.
- Aucune version de dépendance modifiée, aucune autre feature activée.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : le moteur

**Files:**
- Modify: `Cargo.toml` (feature `custom_syntax` sur minijinja)
- Modify: `crates/rbs-cli/Cargo.toml` (dépendances minijinja, serde)
- Create: `crates/rbs-cli/src/template.rs`
- Modify: `crates/rbs-cli/src/main.rs` (`mod template;`)

- [ ] **Step 1 : écrire les tests d'abord**

Cinq tests : un `format!("{{}}")` traverse le rendu intact ; un `${{ secrets.TOKEN }}` aussi ; une variable non fournie fait échouer le rendu ; `&` et `<` ne sont pas échappés ; une variable fournie est substituée.

Run: `cargo test -p rbs-cli`
Expected: échec de compilation — `Renderer` n'existe pas encore.

- [ ] **Step 2 : construire l'environnement**

`SyntaxConfig::builder().variable_delimiters("{@", "@}")`, `UndefinedBehavior::Strict`, auto-échappement à `None`.

- [ ] **Step 3 : la méthode de rendu**

Une seule : source de la template + contexte sérialisable → `String`.

- [ ] **Step 4 : vérifier l'ensemble**

Run: `cargo test -p rbs-cli`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 5 : commit**

Message : `feat(cli): ajoute le moteur de rendu des templates`, corps portant le *pourquoi* des délimiteurs et un intertitre `Vérifications :`.
