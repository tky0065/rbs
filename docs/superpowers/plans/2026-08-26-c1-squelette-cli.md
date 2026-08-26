# Squelette du CLI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** poser la surface de commandes complète du binaire `rbs` — les six commandes de la spec §4.1 visibles dans `--help`, avec leurs flags structurants — sans implémenter aucune d'elles.

**Architecture:** trois fichiers. `cli.rs` porte toute la déclaration `clap` derive et rien d'autre : c'est le fichier qu'on relit pour savoir ce que le CLI accepte. `ui.rs` isole `console` derrière quatre helpers, pour que le reste du code n'ait jamais à savoir si la sortie est un TTY. `main.rs` se limite au parse et au dispatch, comme le `main.rs` que C4 générera dans les projets utilisateurs.

Les six commandes sont déclarées dès maintenant, y compris celles qu'implémenteront C7, D10–D12 et E7. Le critère de la tâche porte sur `--help` : un help qui ne montrerait que les commandes déjà codées ne le remplit pas. Chaque bras non implémenté sort en code 2 avec un message nommant la commande — un code distinct de 1 pour qu'un script sache différencier « pas encore là » d'un échec réel.

**Tech Stack:** clap 4 (derive), console.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.1, §4.3

## Global Constraints

- Aucune écriture disque, aucun rendu de template, aucun prompt : C2, C5 et C6 s'en chargent.
- Pas de `assert_cmd` ici — le test bout-en-bout est C8, qui compile un projet généré.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : déclaration des commandes

**Files:**
- Modify: `crates/rbs-cli/Cargo.toml` (dépendances clap, console, anyhow)
- Create: `crates/rbs-cli/src/cli.rs`
- Create: `crates/rbs-cli/src/ui.rs`
- Modify: `crates/rbs-cli/src/main.rs`

- [ ] **Step 1 : écrire les tests d'abord**

Trois tests dans `cli.rs` : `debug_assert()` de clap ; le help long nomme les six commandes ; `rbs g crud x` parse vers la même variante que `rbs generate crud x`.

Run: `cargo test -p rbs-cli`
Expected: échec de compilation — `Cli` n'existe pas encore.

- [ ] **Step 2 : déclarer la surface**

`Cli` + `Commands` (`new`, `add`, `generate`, `migrate`, `doctor`), sous-énumérations `GenerateCommands` et `MigrateCommands`. Descriptions reprises de la spec §4.1. Flags : `--with`, `--yes`, `--fields`, `--template-dir`, `--force`.

- [ ] **Step 3 : helpers de sortie**

`step`, `success`, `warn`, `error` au-dessus de `console::style`. Aucune détection de TTY maison : `console` s'en charge et respecte `NO_COLOR`.

- [ ] **Step 4 : dispatch**

`main.rs` parse puis oriente. Chaque bras appelle un stub qui affiche la commande visée et sort en code 2.

- [ ] **Step 5 : prouver le critère**

Run: `cargo run -p rbs-cli -- --help`
Expected: les six commandes listées avec une description utile chacune.

- [ ] **Step 6 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 7 : commit**

Message : `feat(cli): pose la surface de commandes du binaire rbs`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
