# Extracteur JSON validé — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** qu'un controller généré reçoive un DTO déjà validé, et qu'un corps illisible ou non conforme sorte en réponse RFC 9457 plutôt qu'en 500.

**Architecture:** `ValidatedJson<T>` est un `FromRequest` dont le rejet est directement `Error` : la conversion en réponse est ainsi celle d'A4, sans passe-plat. L'extraction précède la validation — un corps illisible ne peut pas être validé. Une variante `Error::BadRequest` est ajoutée : une requête mal formée n'est pas une erreur métier, et la loger dans `Domain` ferait passer un code du noyau pour un code du projet. La frontière retenue s'écarte d'axum sur un point, assumé et documenté : tout rejet d'extraction devient 400, et 422 reste réservé à l'échec de `validator`, ce qui donne à qui débogue une lecture immédiate — 400 « corps illisible », 422 « corps lu, règles non respectées ».

**Tech Stack:** `axum` (`FromRequest`, `Json`, `JsonRejection`), `validator`, `serde`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.4, §5.1

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Aucun type d'axum dans la signature publique d'`Error` : une mise à jour d'axum ne doit pas rompre `rbs-core`.

---

### Task 1 : extracteur JSON validé

**Files:**
- Create: `crates/rbs-core/src/extract.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/error.rs` (variante `BadRequest` + son mapping + un test)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod extract`, ré-export)

**Interfaces:**
- Consumes: `crate::Error` (A3/A4).
- Produces: `rbs_core::extract::ValidatedJson`, `rbs_core::Error::BadRequest`.

- [ ] **Step 1 : écrire les tests d'abord**

Dans `extract.rs`, un DTO de test dérivant `Deserialize` et `Validate`, monté derrière un routeur :

- `un_corps_valide_est_extrait_tel_quel`
- `un_corps_invalide_repond_422_avec_le_detail_par_champ` : le corps de réponse porte `errors.email`
- `un_json_malforme_repond_400_pas_500`
- `un_content_type_absent_repond_400_pas_500`

Dans `error.rs`, un test du nouveau mapping : `BadRequest` → 400 portant son message.

- [ ] **Step 2 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core extract`
Expected: échec de compilation, `ValidatedJson` inexistant.

- [ ] **Step 3 : ajouter `Error::BadRequest`**

Variante `BadRequest(String)`, `#[error("requête invalide : {0}")]`, et sa ligne dans `IntoResponse` : `(StatusCode::BAD_REQUEST, "Bad Request", Some(message), None)`.

- [ ] **Step 4 : implémenter `extract.rs`**

- `pub struct ValidatedJson<T>(pub T);`
- `impl<T, S> FromRequest<S> for ValidatedJson<T>` : `Json::<T>::from_request` puis `validate()`.
- Le rejet d'axum est converti par son `body_text()`, jamais par son type : garder `JsonRejection` hors de la signature d'`Error`. Commenter le *pourquoi* de la frontière 400/422.

- [ ] **Step 5 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core extract` puis `cargo test -p rbs-core error`
Expected: 4 passed et le module d'erreur toujours vert.

- [ ] **Step 6 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 7 : commit**

Message : `feat(core): ajoute l'extracteur json validé`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
