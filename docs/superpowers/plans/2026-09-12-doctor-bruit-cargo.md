# `rbs doctor` sans le bruit de cargo — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Quand `cargo::run` capture la sortie standard d'un binaire du projet (`doctor` → `migration version`, `migrate status`), la progression de cargo (`Updating / Locking / Compiling`) n'atteint plus le terminal ; elle n'est rejouée sur stderr qu'en cas d'échec, pour qu'une erreur de compilation reste lisible. `migrate up`/`down`, `seed`, `dev` ne changent pas.

**Architecture:** `run` ne pipait que stdout, stderr étant hérité pour montrer la compilation. En mode capture, stderr passe aussi en `Stdio::piped()` ; sur statut non nul, `run` la rejoue telle quelle avant de rendre `Error::Statut(code)`. Rejouer plutôt que porter la stderr dans l'erreur : la sortie d'échec reste octet pour octet celle d'aujourd'hui (flux cargo, puis la ligne de verdict de l'appelant), et ni `Error::Migration { code }` ni `Error::Seeds { code }` n'ont à grandir d'un champ que leur `Display` noierait ou perdrait. Point d'injection : `run_with(programme, …, rejeu: &mut dyn Write)`, privé, appelé par `run` avec `"cargo"` et `io::stderr()` ; les tests posent un faux `cargo` (script shell) et un `Vec<u8>` en puits.

**Tech Stack:** Rust `std::process`, tests `#[cfg(all(test, unix))]` avec `tempfile` (déjà en dev-dependency), script `sh`.

**Backlog :** IMPROVE.md, tâche 24.

## Global Constraints

- `capturer == false` : rien ne change, stdout et stderr hérités.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.
- Preuve de bout en bout : `integration_doctor` (test lent `the_slow_check_announces_itself_before_the_finding`, sous Docker) redirigé vers le scratchpad.

---

### Task 1: la couture, puis la capture

**Files:**
- Modify: `crates/rbs-cli/src/cargo.rs`

- [x] **Step 1: Couture + tests rouges** — `run_with(programme, root, arguments, variables, capturer, rejeu)` sans encore piper stderr ; tests : (a) succès en capture → `Ok("utile\n")`, puits vide ; (b) échec en capture → `Err(Statut(1))`, puits contenant « Compiling faux ». Vu rouge sur (b) : « la sortie d'erreur de cargo doit être rejouée : "" » ; (a) déjà vert.
- [x] **Step 2: Fix** — stderr en `Stdio::piped()` quand `capturer`, rejouée sur statut non nul ; commentaire de `run` réécrit.
- [x] **Step 3: Vert** — `cargo::` 2 passés, `--lib` 1160 passés, clippy et fmt propres.
- [x] **Step 4: Bout en bout** — `integration_doctor --include-ignored` : 7 passés dont `the_slow_check_announces_itself_before_the_finding`, `EXIT=0`. Ce test prouve que `doctor` fonctionne encore avec stderr capturée et que l'annonce précède le constat ; il n'affirme pas l'absence des lignes `Compiling` dans le rapport — ce point n'est prouvé que par le test unitaire au faux `cargo`.
