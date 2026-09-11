# Auth : l'horloge des jetons liée en paramètre — plan d'implémentation

**Goal:** Sur SQLite, un jeton de réinitialisation périmé cesse d'être consommable jusqu'à minuit UTC, et la liste des sessions cesse de montrer une session échue le même jour.

**Architecture:** sqlx écrit `DateTimeWithTimeZone` en RFC 3339 (`2026-01-01T…+00:00`) là où `CURRENT_TIMESTAMP` de SQLite rend `2026-01-01 23:59:59` : la comparaison est textuelle, et `'T' > ' '`. Les deux dépôts `auth` remplacent **chaque** `Expr::current_timestamp()` — filtres sur `expires_at` et écritures de `consumed_at`/`revoked_at`, sinon une même colonne mélange deux formats — par un instant lu une fois côté Rust, `Utc::now().fixed_offset()`, lié en paramètre comme `jobs/queue.rs.jinja` le fait déjà.

**Tech Stack:** minijinja (`{@ @}`), SeaORM `ColumnTrait::gt/lt` sur `DateTimeWithTimeZone` et `Expr::value` pour l'écriture, chrono, tests `#[ignore]` joints à la base, un banc SQLite dans `integration_auth.rs`, régénération d'`examples/blog-auth` par diff.

## Étapes

- [x] 1. Tests rouges dans le fragment : `password.rs.jinja` — un jeton émis avec `expires_at` une seconde dans le passé est refusé par `consume`, puis retiré par `purge_expired` qui garde le vivant (un seul test : la purge est globale, deux tests parallèles se la faisaient l'un à l'autre — vu sur PostgreSQL, « le jeton vient d'être émis ») ; `session.rs.jinja` — une session reculée d'une seconde disparaît de `GET /auth/sessions`. Preuve : banc SQLite sur les templates d'origine, `58 passed; 3 failed`, les trois échecs étant les tests neufs (« un jeton échu a été consommé », « une session échue est encore listée »).
- [x] 2. Banc SQLite dans `integration_auth.rs` : `the_auth_tests_of_the_generated_project_pass_on_sqlite`, sur le modèle d'`integration_crud.rs`. Un compose minimal est posé avant `add auth` : sans lui, `add` refuse (« docker-compose.yml est introuvable ») alors que l'ancre `services` est optionnelle — défaut hors périmètre, consigné dans le rapport.
- [x] 3. Correctif : `one_time_token.rs.jinja` (`consume`, `invalidate_pending`, `purge_expired`) et `refresh_token.rs.jinja` (`consume`, `revoke_sessions_of`, `open_sessions_of`, `revoke_session`) lient `Utc::now().fixed_offset()` ; `ExprTrait` retiré des deux imports.
- [x] 4. `examples/blog-auth` régénéré par diff (quatre fichiers, aucun n'étant édité à la main), `cargo clippy --workspace --all-targets -- -D warnings` dans l'exemple → 0, puis `cargo test -p rbs-cli --test integration_examples` → 19 passed.
- [x] 5. Passe lente : `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --ignored` → 8 passed, dont le banc SQLite et le banc PostgreSQL.
- [x] 6. `cargo fmt --all --check` → 0 ; `cargo clippy --workspace --all-targets -- -D warnings` → 0 ; `cargo test -p rbs-cli --lib` → 1137 passed ; `integration_docs` → 13 passed, 1 ignored.
