# `config::section::<T>` — le noyau ouvre sa cascade

Applique §2.2 de `2026-08-27-v0.3-integrations-design.md`, qui fixe la signature.

## Choix internes

- **Emplacement** : fonction libre de `crates/rbs-core/src/config.rs`, posée entre
  `impl Config` et `figment()`, qui reste privée. Pas de re-export à la racine :
  `section` seul, hors du chemin `config::`, ne dirait plus de quoi il est la section.
- **Erreur** : variante `ConfigError::SectionAbsente(String)` plutôt que le
  `missing field` de figment, dont le message anglais détonnerait au milieu des autres.
- **Absence de défauts** : aucun `Serialized::default` n'est opposé à la section ; la
  cascade est celle de `figment()` telle quelle, et l'extraction porte sur la seule clé
  demandée.

## Étapes

1. Écrire les trois tests dans `mod tests` de `config.rs`, sur une struct
   `SectionEtrangere` déclarée dans le module de test — le noyau n'en sait rien.
2. Les voir échouer (la fonction n'existe pas : échec de compilation).
3. Ajouter la variante `SectionAbsente` puis `section::<T>`.
4. Vérifier, formater, commiter.

## Preuves, une par ligne `✓`

| Critère | Commande |
|---|---|
| Section absente → erreur nommant la section | `cargo test -p rbs-core config::tests::une_section_absente_rend_une_erreur_nommant_la_section` |
| Cascade `default.toml` < `{env}.toml` < `RBS_*` | `cargo test -p rbs-core config::tests::pour_une_section_etrangere_le_profil_puis_l_environnement_l_emportent` |
| `#[serde(default)]` de l'appelant respecté | `cargo test -p rbs-core config::tests::les_defauts_serde_de_l_appelant_sont_respectes` |
| Compilée sans flag | `cargo build -p rbs-core --no-default-features` (`default = []`, donc identique à `cargo build -p rbs-core`) |

Qualité : `cargo clippy -p rbs-core --all-targets -- -D warnings` et `cargo fmt --all --check`.
