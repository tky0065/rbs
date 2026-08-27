# I4 — Refresh avec rotation

## But

Consommer un jeton de rafraîchissement pour en émettre un nouveau, et prouver que
l'ancien ne vaut plus rien.

## Ce qui existe déjà

`login` pose la ligne de `refresh_tokens` avec `token::empreinte`, et `emettre()` sait
produire une paire. La rotation n'ajoute que « consommer l'ancien » : `refresh` réutilise
`emettre` sans le modifier.

## La course

`consommer` fait un `UPDATE … SET revoked_at = now() WHERE id = ? AND revoked_at IS NULL`
et rend le nombre de lignes touchées. Un pas au-delà du critère, pour trois lignes : deux
`refresh` simultanés du même jeton franchiraient tous deux la lecture avant que l'un ait
écrit, et repartiraient chacun avec une paire valide. Le `WHERE revoked_at IS NULL` fait
de la consommation un compare-and-swap, et le perdant reçoit un 401 comme les autres.

## Le rejeu

Un jeton déjà tourné qu'on représente rend 401, et rien d'autre. Révoquer toute la famille
— la parade recommandée contre le vol de jeton — déconnecterait l'utilisateur légitime sur
un double-clic, et dépasse ce que la tâche demande.

## Accès à la base depuis les tests

Deux critères l'exigent : lire la colonne stockée, et fabriquer une ligne déjà expirée.
`connexion()` s'ajoute à côté d'`application()` plutôt que de changer la signature de
celle-ci, qui obligerait à toucher les sept tests déjà écrits.

## Étapes

1. Tests rouges dans `tests.rs.jinja` : nouvelle paire, ancien refusé, expiré refusé,
   la table porte l'empreinte et jamais le jeton.
2. `repository.rs` : `find_refresh_token`, `consommer`.
3. `service.rs` : corps de `refresh`. Ligne introuvable, révoquée ou expirée rendent le
   même `Unauthorized` — les distinguer renseignerait sur l'état des sessions.
4. `controller.rs` : `state.auth()` passé à `refresh`, comme à `login`.
5. `mod.rs` : `#![allow(dead_code)]` retiré si `me` et `logout` ne le retiennent plus.

## Hors périmètre

`logout` et `me`. La détection de réutilisation. L'enregistrement OpenAPI (I7).

## À rouvrir

`tests.rs.jinja` passe à ~330 lignes, au-delà des ~200 que la convention pose comme
signal de scission. Gardé en un fichier ici — c'est un fichier de tests ; la question se
repose à I6 si l'on dépasse 400 lignes.
