# Le cache typé

`get`, `set`, `set_ttl`, `invalider`, `invalider_prefixe`, sérialisation par serde.

## Forme retenue

- Les cinq méthodes vivent sur `Cache` et rendent `rbs_core::Result`, `Error::Internal`
  portant la cause : un handler les enchaîne par `?` sans hiérarchie d'erreurs à lui.
- Quatre fonctions privées portent tout ce qui ne joint pas le serveur — `encoder`,
  `decoder`, `motif`, `a_supprimer`. C'est là que vivent les décisions du module, et
  c'est ce que `src/cache/tests.rs` éprouve sans Redis.
- `decoder` reçoit `Option<Vec<u8>>` : le `nil` d'une clé absente ou expirée devient
  `None`, jamais une erreur de désérialisation.
- `invalider_prefixe` refiltre côté client les clés que `SCAN MATCH` a rendues. Le motif
  est un glob interprété par le serveur ; une suppression est irréversible, et le préfixe
  se revérifie là où il est sûr. `motif` échappe par ailleurs les métacaractères, sans
  quoi `rbs add redis` livrerait un `invalider_prefixe("a*")` qui emporte `abc`.
- `serde_json` s'ajoute aux dépendances du fragment : « sérialisation par serde » demande
  un format, et la crate n'entre pas dans le noyau.

## Étapes

1. `tests.rs.jinja` d'abord, déclaré par `#[cfg(test)] mod tests;` — quatre tests pour
   les trois critères. Génération d'un projet, `cargo test` : rouge.
2. Les quatre fonctions privées, puis les cinq méthodes par-dessus.
3. `cargo test`, `cargo clippy --workspace --all-targets -- -D warnings` et `rustfmt`
   sur le projet généré.
4. Une morsure par ligne `✓ Test :`, chacune choisie pour ne faire tomber que le sien.
