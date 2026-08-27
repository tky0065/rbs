# Trait `Storage` et backend fichiers

## Ce qui est déposé

Un fragment `templates/features/storage/`, sur le moule d'`auth` :

- `mod.rs.jinja` → `src/storage/mod.rs` : `StorageError`, `StorageConfig`, le trait
  `Storage` (quatre méthodes), `normaliser`, `construire` et `depuis_config`.
- `fichiers.rs.jinja` → `src/storage/fichiers.rs` : `StockageFichiers`, sur `tokio::fs`.
- `tests.rs.jinja` → `src/storage/tests.rs` : les deux tests du critère.
- `feature.toml` : ancres `features`, `state_champs`, `state_init` ; section `[storage]`
  dans `config/default.toml` ; dépendances `async-trait` et `thiserror` ; feature `fs`
  activée sur `tokio`.

`FEATURES_CONNUES` (`new.rs:23`) gagne `"storage"`, et rien d'autre.

## Décisions

- Le trait est dyn-compatible via `async-trait` : `AppState` porte
  `Arc<dyn Storage>`, sans quoi le choix du backend par la configuration exigerait de
  rendre `AppState` générique — donc de contaminer toute signature de handler.
- `normaliser` vit dans `mod.rs` et sert **aux deux** backends : la clé est résolue
  composant par composant et refusée dès qu'un `..` passe au-dessus de la racine. Un
  refus fondé sur une sous-chaîne laisserait passer `a/../../b`.
- `supprimer` est idempotent des deux côtés : `DeleteObject` de S3 l'est, et deux
  backends non substituables feraient échouer `N3`.
- `construire(StorageConfig)` est séparée de `depuis_config()` pour que le choix du
  backend s'éprouve sans toucher à la cascade de configuration.

## Ordre

1. `tests.rs.jinja` d'abord ; `rbs new` + `rbs add storage` + `cargo test` → échec.
2. `mod.rs.jinja` et `fichiers.rs.jinja`, puis les mêmes commandes → vert.
3. `cargo clippy --workspace --all-targets -- -D warnings` et `rustfmt --check` sur le
   projet généré.
4. Morsures : retirer le refus de `Component::ParentDir`, puis casser `existe`.
