# `--with-upload` sur un projet d'avant 1.3.0 — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs generate crud --with-upload` refuse, avant toute écriture, un projet dont le manifeste déclare `storage` mais où le fragment vit encore en `src/storage/` — le rendu importe `crate::modules::storage`, et le projet engendré ne compilerait plus.

**Architecture:** Un second contrôle dans `generate::command::plan_for`, juste après `UploadSansStorage` : `root.join("src/modules/storage/mod.rs").exists()`. Une nouvelle variante `Error::UploadStorageHorsModules` nomme le chemin attendu et prescrit ce que la note 1.3.0 du CHANGELOG prescrit déjà pour `jobs` et `cache` — déplacer le répertoire sous `src/modules/` et corriger ses `use` —, car `rbs upgrade` ne déplace aucun module (« moving them would mean rewriting `use` statements the CLI does not own »). Le chemin n'est pas résolu comme `crate_path` : la template écrit `crate::modules::storage` en dur et la tâche ne réécrit pas les templates.

**Tech Stack:** `thiserror`, `std::fs`, test unitaire sur `crate::fixtures::Project` (aucun Docker).

## Global Constraints

- Aucune template touchée : `integration_examples` n'a pas à être relancé.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: le refus et son test

**Files:**
- Modify: `crates/rbs-cli/src/generate/command.rs` (variante d'erreur, contrôle, test)
- Modify: `docs/docs/cli/generate.md` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/generate.md` (la phrase du tableau qui décrit la vérification de `--with-upload`)

- [x] **Step 1 : test rouge** — `upload_on_a_storage_fragment_left_at_the_root_is_refused_and_names_the_expected_path` : projet créé avec `storage`, `src/modules/storage` renommé en `src/storage`, `with_upload: true` → l'erreur nomme `src/modules/storage/mod.rs`, et rien n'est écrit.
- [x] **Step 2 : variante `UploadStorageHorsModules`** et contrôle après `UploadSansStorage`.
- [x] **Step 3 : docs** — compléter la phrase « checked before anything is written » des deux tableaux.
- [x] **Step 4 : vérifications** — `cargo test -p rbs-cli --lib generate::command`, clippy, fmt, `integration_docs`.
