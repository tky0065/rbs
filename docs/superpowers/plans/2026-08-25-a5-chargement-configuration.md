# Plan — A5 · Chargement de configuration

Tâche : `TODO.md` → lot A, A5. Spec de référence : §5.3 (configuration), §3.3 (arborescence
du projet généré : `config/default.toml`, `config/{env}.toml`, `.env.example`).

1. `Cargo.toml` racine : ajouter `dotenvy` aux `[workspace.dependencies]`, et les features
   `toml` + `env` sur `figment` (`test` en `dev-dependencies` pour `figment::Jail`).
2. `crates/rbs-core/src/config.rs` : struct `Config` typée — `env: String`,
   `server { host, port }`, `database { url }`. `database.url` **n'a pas de défaut** :
   c'est ce qui fait échouer le boot quand il manque, et c'est le premier `✓`.
3. `ConfigError` reste local à `config.rs` — thiserror, enveloppe `figment::Error`.
   Ce n'est **pas** une variante de `Error` : une erreur de boot ne devient jamais une
   réponse HTTP, et l'y ajouter obligerait à lui inventer un statut dans l'`IntoResponse`
   d'A4. Ça évite aussi d'exposer `figment::Error` dans l'API publique d'une crate publiée.
4. Les cinq couches de la spec, dans l'ordre : `Serialized::default` (défauts) →
   `Toml::file("config/default.toml")` → `Toml::file("config/{env}.toml")` → `.env` →
   `Env::prefixed("RBS_").split("__")`. Les deux fichiers TOML sont optionnels : ils vivent
   dans le projet généré, pas dans ce dépôt.
5. Le nom du profil est résolu **avant** de choisir le fichier `config/{env}.toml` :
   les couches défauts + `default.toml` + `.env` + environnement sont assemblées une
   première fois pour en extraire la seule clé `env`, avec `development` en repli.
6. La couche `.env` est un provider maison lisant le fichier via `dotenvy::from_path_iter`,
   **sans jamais toucher à l'environnement du processus**. `dotenvy::dotenv()` polluerait
   l'environnement global : les tests fuiteraient les uns dans les autres et l'ordre de
   précédence deviendrait implicite. Les clés y subissent la même transformation que dans
   le provider `Env` (préfixe `RBS_` retiré, minuscules, `__` → `.`, valeur passée par
   `Value::from_str` pour typer les nombres et booléens).
7. TDD : les deux `✓` de la tâche d'abord — champ requis manquant → erreur nommant le
   champ ; `RBS_SERVER__PORT` écrasant le `default.toml` — puis un test de la couche
   `.env` (une couche non testée est une couche non prouvée) : valeur lue depuis `.env`,
   puis cédant devant la vraie variable d'environnement. Tous sous `figment::Jail`, qui
   sérialise les tests et restaure l'environnement — `std::env::set_var` est `unsafe` en
   édition 2024 et casserait l'exécution parallèle.
8. Preuves : `cargo test -p rbs-core`, `cargo clippy --workspace --all-targets
   -- -D warnings`, `cargo fmt --all --check`.

Hors périmètre : le champ `log_format` et la bascule `RBS_LOG_FORMAT` (`A7`), la
construction du pool depuis `database.url` (`B1`), la génération de `config/default.toml`
et de `.env.example` dans le projet de l'utilisateur (`C4`).
