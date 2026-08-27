# I1 — Manifeste d'auth et squelette des templates

## But

Un fragment `auth` qui s'installe par `rbs add auth` et dont le projet résultant compile,
sans qu'aucun handler soit encore implémenté. La tranche est verticale : elle traverse le
moule de bout en bout — fichiers, ancres, migration, patchs de manifeste, de configuration
et d'environnement — pour que I2 à I7 n'aient plus qu'à remplir des corps.

## Écart au TODO

Le fragment dépose dans `src/auth/`, non `src/features/auth/` : l'ancre `features` insère
`mod <module>;` en tête de `main.rs`, et `generate crud` dépose déjà dans `src/<module>/`.
Un `src/features/mod.rs` partagé serait un fichier que deux fragments se disputeraient,
contre l'idempotence de H6. La ligne descriptive du lot I est corrigée en conséquence.

## Étapes

1. Test unitaire rouge : le manifeste du fragment `auth` vise `features`, `routes`,
   `openapi`, et déclare une migration — donc les quatre ancres.
2. `feature.toml` et les sept templates. Les `#[utoipa::path]` sont écrits en entier dès
   maintenant ; les corps rendent `Error::Domain { NOT_IMPLEMENTED }`.
3. Migration `create_users` réduite à `id`, `created_at`, `updated_at`. I2 l'étoffe.
4. Test d'intégration `integration_auth.rs`, `#[ignore]` : `rbs new`, `rbs add auth`,
   puis `cargo check --all-targets` du projet généré — `--all-targets` sans quoi
   `tests.rs` n'est jamais compilé.

## Les cinq chemins, fixés ici

`POST /auth/register`, `POST /auth/login`, `POST /auth/refresh`, `POST /auth/logout`,
`GET /auth/me`. I7 les enregistre, J2 les joue.

## Hors périmètre

Les colonnes `email`, `password_hash`, `role` ; la table `refresh_tokens` ; tout corps de
handler. À connaître pour I2 : `Manifeste.migration` est un `Option`, donc une seule
migration par fragment — la seconde table exigera d'étendre le moule, et deux migrations
planifiées dans la même seconde porteraient aujourd'hui le même horodatage.
