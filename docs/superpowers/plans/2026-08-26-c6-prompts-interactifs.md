# Prompts interactifs — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** poser les trois questions de `rbs new` — nom, base, features — chacune doublée d'un flag, et garantir que `--yes` n'ouvre aucun prompt.

**Architecture:** un module `prompts` qui rend une `OptionsProjet` à partir des flags et de `yes`. Le point structurant : `--yes` court-circuite **avant** tout appel à `inquire`, il ne le configure pas. `inquire` échoue de lui-même sans TTY ; l'appeler pour lui dire ensuite « prends le défaut » rendrait le CLI inutilisable en CI. La résolution passe donc par un trait `Questions` dont l'implémentation `inquire` n'est atteinte que sur les questions réellement posées — ce qui rend « n'ouvre aucun prompt » vérifiable par un test unitaire, avec un questionneur espion, et pas seulement observable de l'extérieur. Les échecs d'`inquire` sont traduits en conseil actionnable : sans terminal, le message nomme `--yes` et les flags plutôt que de remonter un descripteur fermé.

**Tech Stack:** inquire 0.9, clap.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.1

## Global Constraints

- `--yes` rend la résolution purement calculatoire : aucune entrée-sortie.
- Toute question a son flag : `nom` positionnel, `--database-url`, `--with`.
- `rbs new` n'est pas implémentée ici : le module précède son appelant.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : le flag manquant

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs` (`Commands::New`)
- Modify: `crates/rbs-cli/Cargo.toml` (`inquire`)

- [ ] **Step 1 : ajouter `--database-url`**

Seule des trois questions à n'avoir aucun équivalent en flag. Rien d'autre ne change dans `cli.rs`.

- [ ] **Step 2 : vérifier la déclaration**

Run: `cargo test -p rbs-cli cli::`
Expected: les tests clap existants passent toujours.

### Task 2 : la résolution

**Files:**
- Create: `crates/rbs-cli/src/prompts.rs`
- Modify: `crates/rbs-cli/src/main.rs` (`mod prompts;`)

- [ ] **Step 1 : écrire les tests d'abord**

Un questionneur espion comptant les questions posées : avec `yes`, le compteur reste à zéro et les défauts sortent. Un flag fourni prend le pas sur le défaut, pour chacune des trois questions. La traduction de `InquireError::NotTTY` nomme `--yes`.

Run: `cargo test -p rbs-cli`
Expected: échec de compilation — le module n'existe pas.

- [ ] **Step 2 : implémenter**

`OptionsProjet`, le trait `Questions`, son implémentation `inquire`, la traduction des erreurs, et la résolution `flag → défaut si yes → question`.

- [ ] **Step 3 : vérifier l'ensemble**

Run: `cargo test -p rbs-cli`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 4 : commit**

Message : `feat(cli): résout les options de création par flags ou par questions`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
