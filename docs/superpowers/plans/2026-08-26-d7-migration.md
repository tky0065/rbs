# Génération de la migration — plan

**But :** produire `migration/src/mAAAAMMJJ_HHMMSS_create_<table>.rs` correspondant aux
mêmes champs que l'entité de D2.

**Approche :** template `templates/feature/migration.rs.jinja` et `generate/migration.rs`.
`up` construit `Table::create()` avec `id` en `uuid()` porteur de `.extra("DEFAULT
uuidv7()")`, les colonnes déclarées via `methode_migration` déjà fournie par D1, les
timestamps, puis les index et contraintes uniques. `down` est un `Table::drop()`.

**PostgreSQL 18 minimum** : `uuidv7()` n'y est native qu'à partir de cette version, choix
assumé par la spec §3.6 plutôt que contourné par une fonction maison.

## Étapes

- [ ] Horodatage du nom de fichier injecté par l'appelant, jamais lu depuis l'horloge dans
      le générateur : un rendu doit être reproductible en test.
- [ ] `templates/feature/migration.rs.jinja` puis `generate/migration.rs`, avec tests
      unitaires sur le rendu — `DEFAULT uuidv7()`, colonnes, index, `unique`, `down`.
- [ ] Test d'intégration `testcontainers` sur PostgreSQL 18 : appliquer la migration,
      insérer sans `id`, relire, dérouler puis rejouer.

## Preuves attendues

- ✓ *Migration réversible* — le critère écrit nomme `rbs migrate up|down`, qui est D11 et
  n'existe pas. Substitution validée par le porteur du projet le 2026-08-26 : la preuve
  passe par `Migrator::up` / `Migrator::down` du projet généré, ce qui exerce la même
  migration sans l'enveloppe CLI.
- ✓ *`DEFAULT uuidv7()` sur `id`, un `INSERT` sans `id` reçoit un UUIDv7 valide dont
  l'horodatage de tête est celui de l'insertion* — même test d'intégration.
