# `rbs add redis` — manifeste, section `[cache]`, pool dans l'état

Premier fragment à exercer d'un coup les trois pièces du moule : `[[dependances]]`,
`config::section::<T>` et les deux ancres d'état.

## Forme retenue

- Le fragment vit en `crates/rbs-cli/templates/features/redis/` et dépose `src/cache/`
  (§2.6 de la conception : `mod redis;` rendrait `use redis::Client` ambigu, E0659).
- `src/cache/config.rs` porte `Config`, dont **les défauts sont dans la struct**
  (`#[serde(default = "…")]`) et non dans le noyau ; il est chargé par
  `rbs_core::config::section::<Config>("cache")`.
- `src/cache/mod.rs` porte `Cache`, qui enveloppe le `deadpool_redis::Pool`. Sa
  construction est **faillible et synchrone** : `create_pool` n'ouvre aucune connexion.
- L'accesseur `impl AppState { pub fn cache(&self) -> &Cache }` vit dans `cache/mod.rs`,
  comme `impl HasAuth for AppState` vit dans `auth/mod.rs` : il arrive avec la feature et
  repart avec elle, et il est ce qui lit le champ inséré dans `state.rs`.

## Étapes

1. Tests d'abord, dans `tests/integration_add.rs` : deux tests, un par critère vérifiable
   sans compiler le projet — ce que le fragment écrit, et l'inertie du second `add`.
2. `feature.toml` : deux `[[dependances]]`, trois `[[ancres]]` (`features`,
   `state_champs`, `state_init`), une `[[config]]` `[cache]`.
3. `mod.rs.jinja` et `config.rs.jinja`.
4. `"redis"` ajouté à `FEATURES_CONNUES` (`new.rs:23`), sans rien réordonner.
5. Preuve du premier critère hors test automatisé : `rbs new` puis `rbs add redis` sur un
   projet réel, puis `cargo clippy --workspace --all-targets -- -D warnings` et
   `rustfmt --edition 2024 --check` sur `src/main.rs` — le niveau exigé depuis `I1`.
6. Une morsure sur le critère `✓ Test :`.
