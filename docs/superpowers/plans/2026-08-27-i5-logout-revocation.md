# I5 — Logout et révocation

## But

Fermer une session sans toucher aux autres, et prouver que les autres tiennent.

## Ce qui existe déjà

`logout` est la moitié de `refresh` : même empreinte, même `consommer`, sans réémission.
Rien de neuf dans `repository.rs` — `find_refresh_token` et `consommer` suffisent, ce qui
est le signe que la rotation d'I4 avait la bonne granularité.

## Signature

`logout(db, entree)` reste tel quel : rien n'est signé ici, donc pas d'`AuthConfig`,
contrairement à `login` et `refresh`.

## Contrat

Le controller écrit en I1 rend `204` et déclare `401 jeton inconnu`. On s'y tient plutôt
que de rendre le logout idempotent : ce contrat est déjà publié dans le document OpenAPI,
et un 204 sur un jeton inconnu dirait à un appelant que sa déconnexion a porté alors
qu'elle n'a rien fermé.

## Le seul test qui apporte une garantie neuve

« Les autres sessions restent valides » interdit de révoquer par `user_id` au lieu de
révoquer par ligne. Les deux autres suivent de `consommer`, déjà éprouvée en I4.

## Étapes

1. Tests rouges : 204, refresh révoqué refusé, seconde session intacte.
2. `service.rs` : corps de `logout`.
3. Morsure : faire révoquer toutes les lignes du compte — le troisième test doit tomber.

## Hors périmètre

`me`. L'enregistrement OpenAPI (I7). La scission de `tests.rs`, laissée pour plus tard.
