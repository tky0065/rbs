# Génération des tests — plan

**But :** `rbs generate crud` pose `src/<nom>/tests.rs`, un test d'intégration HTTP du CRUD
complet qui passe sans retouche.

**Approche :** template `templates/feature/tests.rs.jinja` et `generate/tests.rs`.
L'application est montée en mémoire — `router::router(AppState::new(db, config))` puis
`tower::ServiceExt::oneshot` — comme `rbs-core` teste déjà ses propres extracteurs.

**Dans la feature, pas dans `tests/`** : le projet généré est un binaire, un test
d'intégration ne peut atteindre ni `crate::router` ni les DTO de la feature. Le fichier est
donc un module `#[cfg(test)]` du binaire, déclaré par le `mod.rs` de la feature.

**La base est celle du projet** : le test lit `Config::load()`, donc le `.env`, et suppose
les migrations appliquées. C'est la séquence que D13 déroulera — générer, migrer, tester.

## Étapes

- [x] `[dev-dependencies]` du squelette : `tower`, `serde_json`, `uuid`. Le patch de
      manifeste appartient à E3 ; ces trois-là servent toute feature, ils entrent dans la
      template de projet. `axum::body::to_bytes` évite `http-body-util`, et sea-orm expose
      `Uuid` sans son générateur.
- [x] Valeurs d'exemple dérivées du champ, dans une vue locale au générateur plutôt que
      sur `Champ` : elles n'intéressent que les tests. Les valeurs textuelles portent un
      suffixe tiré au sort pour qu'une seconde exécution ne bute pas sur un champ `unique`,
      et un `email` reçoit une adresse valide sans quoi la validation de D3 refuse le corps.
- [x] `templates/feature/tests.rs.jinja` puis `generate/essais.rs` — `generate::tests` se serait confondu avec les modules de test, avec tests unitaires de
      rendu et stabilité sous rustfmt.
- [x] `mod.rs` de la feature : `#[cfg(test)] mod tests;` conditionné à la présence de
      tests, la feature vide de D10 n'en portant pas.
- [x] Test d'intégration `testcontainers` : projet neuf, feature posée, migration appliquée,
      `cargo test` du projet. Deux tests lourds ne doivent pas poser une feature de même
      nom : leurs projets partagent le répertoire de compilation, et s'y sont montrés
      capables d'échanger leur code compilé.

## Preuves attendues

- ✓ *Les tests générés passent immédiatement, sans retouche* — le test d'intégration
  ci-dessus, qui lance `cargo test` dans le projet généré sans y toucher.
