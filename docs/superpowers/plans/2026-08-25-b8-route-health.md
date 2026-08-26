# Route `/health` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** qu'un orchestrateur puisse distinguer une application vivante d'une application coupée de sa base, sans avoir à interroger une route métier.

**Architecture:** la spec porte ici la même tension qu'en B2 — « health check » est listé côté noyau, `features/health/` côté projet généré — tranchée de la même façon : `rbs-core` porte le handler et la logique, le projet génère la feature qui la monte et reste libre d'en changer la route. Le handler est générique sur `HasCoreState`, donc montable sur l'`AppState` d'un projet comme sur un `CoreState` nu. La décision est séparée du transport par une fonction pure : sans base démarrée, seule la branche 503 est atteignable par une requête réelle, et laisser l'autre moitié non couverte jusqu'à D13 serait un trou inutile.

**Tech Stack:** `axum`, `sea-orm` (`ping`), `serde`, `utoipa`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.2, §3.3

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Aucun test ne requiert Docker : un `DatabaseConnection` déconnecté fait échouer `ping`.
- Le détail d'une erreur base ne part jamais au client, comme en A4.

---

### Task 1 : route de santé

**Files:**
- Create: `crates/rbs-core/src/health.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod health`)

**Interfaces:**
- Consumes: `crate::state::HasCoreState` (B2), `sea_orm::DatabaseConnection::ping` (B1).
- Produces: `rbs_core::health::{handler, routes, Sante}`.

- [ ] **Step 1 : écrire les tests d'abord**

- `une_base_indisponible_repond_503_pas_200` (le ✓ de la tâche), requête réelle sur un pool déconnecté
- `une_base_saine_donne_200_et_un_statut_ok`, sur la fonction pure
- `une_base_injoignable_donne_503_et_nomme_le_controle_en_echec`, sur la fonction pure
- `le_detail_de_l_erreur_base_ne_fuit_pas_dans_la_reponse`

- [ ] **Step 2 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core health`
Expected: échec de compilation, `health::handler` inexistant.

- [ ] **Step 3 : implémenter**

- `Sante { status, checks: Checks { database } }`, sérialisable et `ToSchema`.
- `fn etat(ping: Result<(), DbErr>) -> (StatusCode, Sante)` : la logique, sans transport. Une erreur y est journalisée puis abandonnée — le client n'obtient que `unreachable`.
- `pub async fn handler<S: HasCoreState>(State(state): State<S>) -> Response`.
- `pub fn routes<S: HasCoreState>() -> Router<S>` montant `GET /health`.

- [ ] **Step 4 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core health`
Expected: 4 passed.

- [ ] **Step 5 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 6 : commit**

Message : `feat(core): ajoute la route de santé et sa vérification de la base`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
