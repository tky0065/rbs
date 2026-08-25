# Helpers OpenAPI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** que la documentation d'une API générée décrive ses réponses d'erreur communes sans qu'un seul handler ait à les répéter.

**Architecture:** un `utoipa::Modify` accroché une fois sur le `#[derive(OpenApi)]` du projet, qui parcourt les opérations du document et complète celles auxquelles il manque 422 et 500 — les deux réponses que toute opération peut produire, le noyau validant partout et pouvant défaillir partout. Un handler ayant déclaré la sienne la conserve. Le corps d'erreur RFC 9457, jusqu'ici privé dans `error.rs`, devient un type public porteur de `ToSchema` : sans lui la doc annoncerait un 500 sans dire à quoi il ressemble. Il vit désormais dans `openapi.rs` et `error.rs` l'utilise, de sorte que ce qui décrit le corps et ce qui le produit ne puissent pas diverger. Les autres erreurs (400, 401, 403, 404, 409) sont déclarées une fois dans `components/responses`, référençables par nom.

**Tech Stack:** `utoipa` (`Modify`, `ToSchema`, `openapi::Ref`).

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.4

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Ne pas monter Swagger UI ici : le routeur du projet relève de C4.

---

### Task 1 : réponses d'erreur communes

**Files:**
- Create: `crates/rbs-core/src/openapi.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/error.rs` (`Problem` déplacé et réutilisé)
- Modify: `crates/rbs-core/src/pagination.rs` (`ToSchema` sur `Page` et `Meta`)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod openapi`, ré-exports)
- Modify: `crates/rbs-core/Cargo.toml` (dépendance `utoipa`)

**Interfaces:**
- Consumes: le corps RFC 9457 d'A4.
- Produces: `rbs_core::openapi::{ReponsesCommunes, ProblemDetails}`.

- [ ] **Step 1 : écrire les tests d'abord**

- `le_document_decrit_422_et_500_sans_annotation_par_handler` (le ✓ de la tâche)
- `une_reponse_declaree_par_le_handler_n_est_pas_ecrasee`
- `les_reponses_communes_sont_referencables_par_nom`
- `le_schema_du_probleme_decrit_les_champs_rfc_9457`

- [ ] **Step 2 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core openapi`
Expected: échec de compilation, `ReponsesCommunes` inexistant.

- [ ] **Step 3 : déplacer le corps RFC 9457**

`Problem` quitte `error.rs` pour `openapi.rs` sous le nom `ProblemDetails`, devient public et dérive `ToSchema`. `error.rs` l'importe ; ses champs restent `pub(crate)` en écriture pour que seule la conversion `IntoResponse` les remplisse.

- [ ] **Step 4 : implémenter le modifier**

- `pub struct ReponsesCommunes;` et son `impl Modify`.
- Enregistrer les réponses nommées dans `components.responses`, puis compléter chaque opération de chaque path avec 422 et 500 quand elles manquent. Commenter le *pourquoi* du « quand elles manquent » : un handler qui documente son propre 422 sait mieux.

- [ ] **Step 5 : ajouter `ToSchema` sur `Page`**

Comme annoncé en B6, une fois `utoipa` disponible.

- [ ] **Step 6 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core openapi`
Expected: 4 passed.

- [ ] **Step 7 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 8 : commit**

Message : `feat(core): ajoute les réponses d'erreur communes du document openapi`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
