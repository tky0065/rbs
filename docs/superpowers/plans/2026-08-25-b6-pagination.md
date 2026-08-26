# Pagination — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** des paramètres `page` / `per_page` bornés qu'un client ne peut pas détourner, et une enveloppe de réponse identique d'une feature à l'autre.

**Architecture:** `Pagination` est un `FromRequestParts` sans état — donc montable sur n'importe quel routeur, y compris sans `AppState` — dont les bornes sont des constantes publiques du noyau. Le plafonnement est silencieux, la faute de frappe ne l'est pas : `per_page=5000` est ramené au maximum, mais `per_page=abc` donne un 400, parce qu'ignorer en silence un paramètre que le client croit avoir passé lui fait débugger une pagination qui « ne marche pas ». `offset()` est exposé pour que les repositories générés ne recalculent pas `(page - 1) * per_page` chacun de leur côté.

**Tech Stack:** `axum` (`FromRequestParts`, `Query`), `serde`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.2

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Ne pas tirer `utoipa` ici : le derive `ToSchema` sur `Page<T>` viendra avec B7.

---

### Task 1 : pagination bornée et enveloppe

**Files:**
- Create: `crates/rbs-core/src/pagination.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod pagination`, ré-exports)

**Interfaces:**
- Consumes: `crate::Error` (A3, variante `BadRequest` de B5).
- Produces: `rbs_core::pagination::{Pagination, Page, PAGE_PAR_DEFAUT, PAR_PAGE_PAR_DEFAUT, PAR_PAGE_MAX}`.

- [ ] **Step 1 : écrire les tests d'abord**

- `les_valeurs_par_defaut_s_appliquent_sans_parametre`
- `per_page_au_dela_du_maximum_est_plafonne_sans_erreur` (le ✓ de la tâche)
- `page_zero_est_ramenee_a_la_premiere_page`
- `un_parametre_non_numerique_repond_400`
- `l_offset_suit_la_page_demandee`
- `l_enveloppe_porte_les_donnees_et_leur_meta`, dont `total_pages` valant 0 sur un ensemble vide

- [ ] **Step 2 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core pagination`
Expected: échec de compilation, `Pagination` inexistant.

- [ ] **Step 3 : implémenter**

- Constantes `PAGE_PAR_DEFAUT` (1), `PAR_PAGE_PAR_DEFAUT` (20), `PAR_PAGE_MAX` (100).
- Une struct interne `Parametres { page: Option<u64>, per_page: Option<u64> }` désérialisée par `Query`, puis bornée : `page.max(1)`, `per_page.clamp(1, PAR_PAGE_MAX)`.
- `FromRequestParts` dont le rejet est `Error::BadRequest` : commenter le *pourquoi* du traitement asymétrique (plafonnement muet / faute de frappe signalée).
- `Page<T>` sérialisant `data` et `meta`, `Meta` portant `page`, `per_page`, `total`, `total_pages` calculé par `div_ceil`.

- [ ] **Step 4 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core pagination`
Expected: 6 passed.

- [ ] **Step 5 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 6 : commit**

Message : `feat(core): ajoute la pagination bornée et son enveloppe de réponse`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
