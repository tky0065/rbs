# Feature flags Cargo — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** réserver les noms des quatre extensions prévues en v0.2, sans écrire une ligne de leur code ni figer les crates qu'elles utiliseront.

**Architecture:** quatre flags vides et une section `[features]` explicite avec `default = []`. Déclarer dès maintenant `auth = ["dep:argon2", "dep:jsonwebtoken"]` figerait le choix des crates un an avant le code qui s'en sert. Les flags sont documentés dans `lib.rs` : un flag vide non documenté est indiscernable d'un oubli, et le lecteur doit savoir qu'aucun n'a d'effet en v0.1.

**Tech Stack:** Cargo.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.1, ROADMAP v0.2

## Global Constraints

- Aucune dépendance optionnelle ajoutée : les flags restent vides.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.

---

### Task 1 : déclaration des flags

**Files:**
- Modify: `crates/rbs-core/Cargo.toml` (`[features]`, `[package.metadata.docs.rs]`)
- Modify: `crates/rbs-core/src/lib.rs` (documentation des flags)

- [ ] **Step 1 : déclarer les flags**

`default = []`, puis `auth`, `redis`, `mail`, `storage`, tous vides. `all-features = true` pour docs.rs, réglage qu'on oublie systématiquement après coup.

- [ ] **Step 2 : documenter**

Tableau dans la doc de crate : un flag par ligne, ce qu'il activera, et la mention qu'aucun n'a d'effet en v0.1.

- [ ] **Step 3 : prouver le critère**

Run: `cargo build --all-features` puis `cargo build --no-default-features`
Expected: les deux terminent sans erreur.

- [ ] **Step 4 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 5 : commit**

Message : `build(core): déclare les feature flags des extensions à venir`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
