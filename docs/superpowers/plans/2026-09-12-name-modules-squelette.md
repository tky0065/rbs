# Noms de feature réservés au squelette : `modules`, `seeds`, `bin`, `lib` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs generate crud modules` est refusé avant tout écrit, avec le même diagnostic que `main` ou `health` ; de même `seeds`, `bin` et `lib`.

**Architecture:** `generate::name::MODULES_DU_SQUELETTE` ne connaît que les cinq modules déclarés par `src/main.rs`. Or le squelette pose aussi `src/seeds/`, `src/bin/` et `src/lib.rs`, et `src/modules/` est le point de montage des fragments — créé au premier `rbs add`, donc absent d'un projet frais, si bien que le garde-fou « `src/<module>` existe déjà » de `generate::command` ne le voit pas : `generate crud modules` écrit `src/modules/mod.rs` en CRUD, et tout `rbs add` suivant échoue sur « ancre `<rbs:modules>` introuvable ». `lib` est du même ordre : `src/lib/mod.rs` à côté de `src/lib.rs` est ambigu pour rustc. La liste passe à neuf entrées ; le commentaire dit pourquoi chacune y est.

**Tech Stack:** Rust, tests unitaires de `crates/rbs-cli/src/generate/name.rs` (`cargo test -p rbs-cli --lib generate::name`).

**Backlog :** IMPROVE.md, tâche 22.

## Global Constraints

- Un commentaire explique le *pourquoi* : « `health` est un répertoire, les quatre autres des fichiers » devient faux et se réécrit.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: réserver les quatre noms

**Files:**
- Modify: `crates/rbs-cli/src/generate/name.rs` (constante, commentaire, test `a_skeleton_module_is_rejected_by_naming_it`)

- [x] **Step 1: Test rouge** — étendre la boucle de `a_skeleton_module_is_rejected_by_naming_it` à `modules`, `seeds`, `bin`, `lib`. Attendu rouge : « modules » doit être refusé. — vu rouge : « modules » doit être refusé
- [x] **Step 2: Fix** — `MODULES_DU_SQUELETTE: [&str; 9]`, commentaire réécrit.
- [x] **Step 3: Vert** — `cargo test -p rbs-cli --lib generate::name`, puis `--lib` complet, clippy, fmt. — `generate::name` 5 passés, `--lib` 1158 passés, clippy et fmt propres
