# Plan — A2 · CI minimale

Tâche : `TODO.md` → lot A, A2. Dépend de A1 (workspace), cochée.

1. `.github/workflows/ci.yml` : un job `ci` sur `ubuntu-latest`, déclenché sur `push`
   vers `main` et sur `pull_request`. Linux uniquement — la matrice est `F10`.
2. Durcissement minimal : `permissions: contents: read`, `concurrency` annulant les
   runs superposés d'une même branche.
3. Étapes : `actions/checkout@v7`, `dtolnay/rust-toolchain@stable` (`rustfmt`,
   `clippy`), `Swatinem/rust-cache@v2`, puis `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
   Épinglage par tag majeur ; le passage au SHA relève de `F10`.
4. Preuve du workflow : `actionlint` (schéma Actions, pas seulement la syntaxe YAML).
5. Preuve du garde-fou : injection d'un warning clippy réel puis d'un fichier mal
   formaté, exécution des commandes exactes du job, code de sortie non nul constaté,
   retrait et re-vérification.
6. La moitié « le PR est bloqué » du critère n'est pas prouvable sans dépôt distant ni
   check requis : la case reste `- [ ]`, annotée `PARTIEL`, renvoi à `F13`.

Hors périmètre : matrice multi-OS, service PostgreSQL (aucun test ne le requiert avant
`C8`), publication de couverture, épinglage par SHA.
