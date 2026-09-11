# Rate-limit : `X-Forwarded-For` lu au dernier élément — plan d'implémentation

**Goal:** Derrière un proxy de confiance (`trust_forwarded_for = true`), la clé du compteur
est l'adresse que le proxy appose — le **dernier** élément de `X-Forwarded-For` — et non
le premier, que le client écrit lui-même et peut tirer au hasard à chaque requête.

**Architecture:** `forwarded_for` dans `templates/features/rate-limit/mod.rs.jinja` passe
de `.split(',').next()` à `.rsplit(',').next()`. Aucune clé de configuration nouvelle ; les
commentaires de `forwarded_for`, de `trust_forwarded_for` (`config.rs.jinja`, `feature.toml`)
et le paragraphe de `docs/docs/cli/add.md` (EN + FR) disent ce qu'on lit et pourquoi.

**Spec:** tâche 7 d'`IMPROVE.md`, design validé en chat — tâche *bounded*.

## Étapes (TDD)

- [x] 1. Rouge : dans `tests.rs.jinja`, `behind_a_trusted_proxy_the_forwarded_address_wins`
  attend le dernier élément d'un en-tête à trois adresses ; nouveau test
  `a_forged_address_ahead_of_the_list_does_not_change_the_key` — deux en-têtes aux premiers
  éléments distincts et au même dernier donnent la même adresse. Preuve : les deux tests
  échouent dans `examples/blog-auth` (`cargo test --lib rate_limit`) après report du seul
  fichier de tests.
- [x] 2. Vert : `rsplit(',').next()` dans `mod.rs.jinja` ; commentaires corrigés dans
  `mod.rs.jinja`, `config.rs.jinja`, `feature.toml`.
- [x] 3. Docs : `docs/docs/cli/add.md` et son miroir FR — une phrase sur l'élément lu.
- [x] 4. Régénérer `blog-auth` dans le scratchpad, reporter par diff les fichiers de
  `src/modules/rate_limit/` et `config/default.toml`, jusqu'à `integration_examples` vert.
- [x] 5. Preuves : `cargo test --lib rate_limit` dans `examples/blog-auth` (les tests du
  fragment ne touchent pas la base) ; `integration_auth -- --ignored` sous Docker.
- [x] 6. `cargo fmt --all --check`, `clippy -D warnings`, `cargo test -p rbs-cli --lib`,
  `integration_examples`, `integration_docs`. Commit `fix(rate-limit): …`.
