# I6 — Guard `require_role`, et le corps de `me`

## But

Refuser une route à qui n'a pas le rôle, sans confondre « pas identifié » et « identifié
mais sans droit ».

## Écart au TODO

La tâche ne parle que du guard. Le corps de `me` y est joint : aucune tâche d'I3 à I7 ne
l'écrit, alors que la route est montée depuis I1 et que I7 l'enregistrera dans le document
OpenAPI. Publier un contrat annonçant 200 sur une route qui rend 501 serait un mensonge du
document. Cinq lignes, tout existe déjà.

## Forme du guard

Un trait d'extension sur `Identity`, et non un layer. `Identity` vient du noyau, qui ignore
l'enum `Role` : c'est le projet qui les réunit. Le layer protégerait au niveau du routeur —
impossible à oublier — mais `from_fn_with_state` n'accepte pas de paramètre supplémentaire :
il faudrait une closure au type de retour difficile à nommer, ou une fonction par rôle, ce
qui figerait l'enum que I2 a justement rendu extensible sans migration.

Un rôle que l'enum ne connaît pas — jeton signé par une version antérieure — rend
`Forbidden` plutôt que de paniquer.

## Le 401 et non 403

Gratuit, et c'est le point : l'extractor `Identity` rejette avant que le corps du handler
s'exécute. Le test le prouve au lieu de le supposer.

## La route protégée des tests

Le test compose la sienne plutôt que d'en faire monter une au fragment : rien ne s'ajoute à
l'API livrée, et le test vaut démonstration de l'usage.

Créer un admin demande de promouvoir le compte en base — l'inscription rend toujours un
`user`, par défaut de la table — puis de se reconnecter pour obtenir un jeton portant le
nouveau rôle.

## Étapes

1. Tests rouges : 401 sans jeton, 403 en `user`, 200 en `admin`, `me` rend le profil.
2. `guard.rs`, déclaré dans `feature.toml` et dans `mod.rs`.
3. `service.rs` : corps de `me`. `a_ecrire` perd son dernier appelant et disparaît, avec
   l'import qu'elle seule utilisait.
4. Morsure : comparer sur `Identity.role` brut plutôt que par l'enum, et rendre 403 au lieu
   de 401 — les deux doivent tomber.

## Hors périmètre

L'enregistrement OpenAPI (I7). La scission de `tests.rs`, qui passe à ~520 lignes.
