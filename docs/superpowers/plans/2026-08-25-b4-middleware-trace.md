# Middleware de trace — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** un span par requête portant méthode, chemin, statut et latence, dont le `request_id` se propage à tout log émis pendant la requête.

**Architecture:** un `from_fn` maison, symétrique de B3, plutôt que `tower_http::TraceLayer` : le projet a écrit son propre `FormatEvent` parce que celui de `tracing-subscriber` était trop verbeux, et brancher `TraceLayer` réintroduirait un format non contrôlé dont le span ignore le `request_id`. Le span `requete` porte ce qui est connu à l'entrée — `request_id`, `method`, `path` — et les deux formateurs remontant déjà les champs des spans parents (prouvé en A7), tout log du handler hérite du `request_id` sans effort. Statut et latence n'existant qu'à la sortie, ils partent dans un événement `request` final.

**Tech Stack:** `axum` (`middleware::from_fn`), `tracing` (`info_span`, `Instrument`), `tower` (dev).

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.2

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Ne pas fournir de helper composant les deux middlewares : le routeur généré relève de C4.

---

### Task 1 : middleware de trace

**Files:**
- Create: `crates/rbs-core/src/trace.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/logs/mod.rs` et `logs/aide.rs` (`capture` élargi en `pub(crate)`)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod trace`)

**Interfaces:**
- Consumes: `request_id::current` (B3), `logs::aide::capture` (A6, élargi).
- Produces: `rbs_core::trace::middleware`.

- [ ] **Step 1 : élargir le montage de test**

`logs::aide` passe de `pub(super)` à `pub(crate)`, et `mod aide` devient `pub(crate) mod aide` : le montage sert désormais les tests de tout le crate, pas seulement ceux des deux formateurs.

- [ ] **Step 2 : écrire les tests d'abord**

Dans `trace.rs`, un routeur portant les deux middlewares, interrogé sous un abonné jetable. Le test est synchrone : `capture` prend une closure `FnOnce()`, et le futur y est exécuté par un runtime local.

- `un_log_emis_dans_un_handler_porte_le_request_id_de_sa_requete` : le handler émet `tracing::info!`, la ligne JSON correspondante doit porter le même `request_id` que l'en-tête de réponse.
- `l_evenement_final_porte_la_methode_le_chemin_le_statut_et_la_latence`.
- `une_reponse_d_erreur_est_tracee_avec_son_statut` : un handler renvoyant 500 est tracé `status = 500`.

- [ ] **Step 3 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core trace`
Expected: échec de compilation, `trace::middleware` inexistant.

- [ ] **Step 4 : implémenter `trace.rs`**

- Lire méthode et chemin avant de consommer la requête.
- `info_span!("requete", request_id = %…, method = %…, path = %…)` ; le `request_id` vient de `request_id::current()`, d'où la contrainte d'ordre à documenter sur le module.
- `next.run(request).instrument(span)` et mesure par `Instant::now()`.
- Événement final `tracing::info!(status, latency_ms, "request")` émis **dans** le span, avec une latence en millisecondes à une décimale.

- [ ] **Step 5 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core trace`
Expected: 3 passed.

- [ ] **Step 6 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 7 : commit**

Message : `feat(core): ajoute le middleware de trace par requête`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
