# `[[dependances]]` au manifeste de fragment

Design : `2026-08-27-v0.3-integrations-design.md` §2.3. Précédent : `H5`, qui a donné son
appelant à `AjouterFeatureADependance`.

## Décisions internes

- `default_features` par défaut à `true` dans le schéma : un fragment qui ne dit rien veut
  ce que `cargo add` fait. Seule la valeur `false` s'écrit dans le `Cargo.toml`, sous la
  clé Cargo `default-features`.
- `Dependance::declaration` ne rend une chaîne nue que si la dépendance n'a ni feature ni
  `default-features` à porter ; sinon une table inline, comme aujourd'hui.
- Les `[[dependances]]` sont patchées **avant** les `[cargo.<crate>]` : activer une feature
  suppose la dépendance déclarée.

## Étapes

1. `manifeste.rs` : `DependanceDeclaree { nom, version, features, default_features }` et
   `Manifeste.dependances`, `deny_unknown_fields` comme ses voisines.
2. `metadata.rs` : `Dependance` gagne `default_features: bool` ; `declaration` et
   `ajouter_dependance` l'écrivent et le posent sur une déclaration déjà présente.
3. `installation.rs` : boucle sur `manifeste.dependances` → `PatchToml::AjouterDependance`.
4. `plan/action.rs` : `#[allow(dead_code)]` retiré de `PatchToml`, les trois variantes ayant
   désormais un appelant.

## Preuves

| Critère | Commande |
|---|---|
| Version, features et `default-features` arrivent dans `[dependencies]` | `cargo test -p rbs-cli manifeste::tests` puis `cargo test -p rbs-cli --test integration_add -- une_dependance_declaree` |
| Commentaires et mise en forme survivent | `cargo test -p rbs-cli add::installation::tests` |
| Une dépendance déjà déclarée n'est pas dupliquée | `cargo test -p rbs-cli add::installation::tests` |
| Qualité | `cargo clippy --workspace --all-targets -- -D warnings` · `cargo fmt --all --check` |
