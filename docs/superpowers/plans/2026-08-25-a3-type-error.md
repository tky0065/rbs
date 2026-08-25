# Plan — A3 · Type `Error` et alias `Result`

Tâche : `TODO.md` → lot A, A3. Spec de référence : §5.1 (erreurs).

1. `crates/rbs-core/Cargo.toml` : ajouter `thiserror`, `anyhow`, `validator`, `sea-orm`
   et `axum`, tous hérités du workspace. `sea-orm` reste sur ses features par défaut —
   `DbErr` n'exige ni driver ni runtime, et B9 posera les feature flags.
2. `crates/rbs-core/src/error.rs` : l'énumération de la spec §5.1 à la lettre, dérivée
   `Debug` + `thiserror::Error`. `#[from]` sur `Validation`, `Database` et `Internal` ;
   `StatusCode` pris dans `axum::http` plutôt qu'une dépendance `http` directe.
3. Les messages `#[error(...)]` s'adressent au log serveur : `Database` et `Internal` y
   portent leur source. La règle « ne fuit jamais vers le client » se joue dans
   `IntoResponse`, donc en `A4`.
4. `lib.rs` : `pub mod error;` et ré-export `pub use error::{Error, Result};`.
5. TDD, un test par conversion `From` (`DbErr`, `anyhow::Error`, `ValidationErrors`) plus
   la construction de `Domain` : écrits et vus échouer avant l'énumération.
6. Preuves : `cargo test -p rbs-core error`, `cargo clippy --workspace --all-targets
   -- -D warnings`, `cargo fmt --all --check`.

Hors périmètre : `IntoResponse` et le corps `application/problem+json` (`A4`), le mapping
des variantes vers un statut HTTP (`A4`), les feature flags Cargo (`B9`).
