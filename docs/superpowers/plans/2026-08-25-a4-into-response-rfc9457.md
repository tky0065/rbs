# Plan — A4 · `IntoResponse` conforme RFC 9457

Tâche : `TODO.md` → lot A, A4. Spec de référence : §5.1 (erreurs), §5.2 (`request_id`).

1. `crates/rbs-core/Cargo.toml` : ajouter `serde`, `serde_json`, `tokio` et `tracing`,
   hérités du workspace. `tokio` sert uniquement au `task_local!` du `request_id`.
2. `crates/rbs-core/src/request_id.rs` : point de **lecture** du `request_id`.
   `task_local!` privé, `current() -> Option<String>`, `scope(id, future)`. Le point
   d'écriture — le middleware qui génère l'ULID et ouvre le scope — reste à `B3` ; A4
   n'en a besoin que pour tester la présence du champ.
3. `crates/rbs-core/src/error.rs` : struct privée `Problem` (`Serialize`, champs `None`
   omis) portant `type`, `title`, `status`, `detail`, `errors`, `request_id`, puis
   `impl IntoResponse for Error` avec le mapping arrêté au design :
   `NotFound` 404 · `Validation` 422 + `errors` · `Unauthorized` 401 · `Forbidden` 403 ·
   `Conflict` 409 · `Domain` statut porté, `title` = `code`, `detail` = `message` ·
   `Database` et `Internal` 500 générique.
4. Règle non négociable : `Database` et `Internal` journalisent leur source via
   `tracing::error!` et ne la placent jamais dans le corps. C'est le seul endroit où la
   source est lue.
5. TDD : les deux `✓` de la tâche d'abord — `Validation` → 422 avec le détail des champs,
   `Database` → 500 sans le message de la source — puis le mapping des autres variantes,
   le `content-type: application/problem+json`, et le `request_id` présent dans un scope /
   absent hors scope. Écrits et vus échouer avant l'implémentation.
6. Preuves : `cargo test -p rbs-core`, `cargo clippy --workspace --all-targets
   -- -D warnings`, `cargo fmt --all --check`.

Hors périmètre : la génération de l'ULID et le middleware qui ouvre le scope (`B3`), le
renvoi de l'en-tête `x-request-id` au client (`B3`), les formateurs de logs (`A6`, `A7`).
