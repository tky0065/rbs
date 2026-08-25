# Plan — A1 · Workspace Cargo

Tâche : `TODO.md` → lot A, A1. Spec de référence : §3.1 (structure) et §8 (dépendances).

1. Supprimer `src/main.rs` et `src/` à la racine ; retirer le bloc `[package]` du
   `Cargo.toml` racine, qui devient un manifeste virtuel.
2. `[workspace]` : `members = ["crates/rbs-core", "crates/rbs-cli"]`, `resolver = "3"`
   (un manifeste virtuel n'hérite pas du resolver de l'édition).
3. `[workspace.package]` : `version`, `edition = "2024"`, `rust-version`. Pas de
   `license` (décision de `F8`) ni de `repository` (aucun remote, cf. `F13`).
4. `[workspace.dependencies]` : les dépendances de la spec §8, dernières versions
   stables, résolution du graphe complet vérifiée hors dépôt avant écriture.
5. `crates/rbs-core/src/lib.rs` : doc de crate + `#![warn(missing_docs)]`, aucun item.
   `crates/rbs-cli` : `[[bin]] name = "rbs"` et un `main` placeholder (`C1` fera clap).
6. Preuves : `cargo metadata --no-deps` listant exactement deux membres,
   `cargo build --workspace`, absence de `src/` et de `[package]` à la racine,
   `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings`.

Hors périmètre : toute logique dans les deux crates, la CI (`A2`), le squelette du
CLI (`C1`).
