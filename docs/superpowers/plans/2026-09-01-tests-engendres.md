# Plan — tests engendrés : convention `#[ignore]` et unicité des valeurs

Tâches 87 et 88 d'`IMPROVE.md`, traitées ensemble : elles produisent le même fichier,
`src/<feature>/tests.rs`.

## 87 — les tests CRUD engendrés joignent la base, ils sont `#[ignore]`

1. **Rouge** — `tests_http.rs::mod tests` : `every_scenario_is_ignored` exige autant de
   `#[ignore = "joint la base du projet"]` que de `#[tokio::test]` dans le rendu.
2. **Vert** — `templates/feature/tests.rs.jinja` : bandeau d'en-tête sur le moule d'
   `auth/tests.rs.jinja:22-24`, puis l'attribut sur les sept scénarios.
3. `crates/rbs-cli/tests/integration_crud.rs:88` → `["test", "--workspace", "--", "--ignored"]`.
4. `templates/agents/{fr,en}.md.jinja` : la commande qui joint la base devient
   `cargo test --workspace -- --ignored`, les deux langues dans le même commit.
5. `examples/` : les quatre `tests.rs` CRUD régénérés par diff, jamais par écrasement.

Hors périmètre, à verser au backlog : le workflow engendré
(`features/ci/.github/workflows/ci.yml.jinja:76`) ne lancera plus les tests CRUD.

## 88 — un `unique` non textuel doit tirer sa valeur

1. **Rouge** — `fields.rs::mod tests` : `unique_is_refused_on_a_bool` attend
   `ErrorKind::UniqueOnBool` sur `actif:bool:unique`.
2. **Vert** — `fields/error.rs` : la variante, son message et sa suggestion sur le moule de
   `RedundantIndex` ; `fields.rs` la lève près de `:511`.
3. **Rouge** — `tests_http.rs::mod tests` : `a_unique_number_is_drawn_at_each_call` exige
   `unique_number()` et l'absence de `42` pour `views:int:unique`.
4. **Vert** — `value()` consulte `champ.unique` : `Int` → `unique_number() as i32`,
   `Float` → `unique_number() as f64 / 10.0`, `Datetime` → décalé de
   `Duration::microseconds(unique_number())`. Les champs non uniques ne bougent pas.
5. **Vert** — la template émet l'aide `unique_number()` sous un drapeau de contexte.

## Vérification

`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --all --check`, puis la suite Docker `-- --ignored --no-fail-fast`
(`integration_crud`, `integration_examples`).
