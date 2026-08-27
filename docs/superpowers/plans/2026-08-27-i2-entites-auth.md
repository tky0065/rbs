# I2 — Entités et migrations d'auth

## But

Les deux tables de la feature, l'enum `Role`, et la preuve que le schéma se crée puis se
défait en rendant la base à son état initial.

## Écart au TODO

**Une seule migration** `create_auth_tables`, non deux. Le moule ne pose qu'une migration
par fragment, `rbs migrate down` n'en annule qu'une, et les deux tables arrivent et
repartent avec la feature : les séparer rendrait `add auth` et `migrate down`
dissymétriques sans rien acheter. L'ordre de création — `users` avant la clé étrangère de
`refresh_tokens` — est alors garanti par construction.

## Schéma

`users` : `id` uuid PK `uuidv7()`, `email` varchar NOT NULL **unique**, `password_hash`
varchar NOT NULL, `role` varchar NOT NULL défaut `'user'`, `created_at`, `updated_at`.

`refresh_tokens` : `id` uuid PK `uuidv7()`, `user_id` uuid NOT NULL FK `users(id)` ON
DELETE CASCADE, `token_hash` varchar NOT NULL **indexé**, `expires_at` NOT NULL,
`revoked_at` NULL, `created_at`, `updated_at`.

`revoked_at` est nullable parce qu'I4 doit refuser un jeton après rotation et I5 laisser
les autres sessions valides : les deux se lisent sur la ligne, sans la supprimer.

## Étapes

1. Test rouge, `#[ignore]` sous testcontainers : `up`, interrogation du schéma par `psql`
   dans le conteneur, `down`, disparition des tables.
2. Migration `create_auth_tables`.
3. `model.rs` : `pub mod user`, `pub mod refresh_token`, `enum Role` en VARCHAR via
   `DeriveActiveEnum` — un rôle de plus ne demandera aucune migration.
4. `repository.rs` suivi sur les nouveaux chemins de modules.

## Hors périmètre

Tout corps de service. Le repository ne gagne que ce que les entités imposent.
