# Les parcours d'`auth` qui enchaînent des écritures les font en une transaction

Date : 2026-09-12
Portée : `crates/rbs-cli/templates/features/auth/{repository,service,tests}/`, un test rapide
d'`integration_auth`, `examples/blog-auth/src/auth/`, `CHANGELOG` en deux langues.
Hors portée : `login` et `register` (une seule écriture chacun), `request_reset` et
`verification::request` (voir « Ce qui reste hors transaction »), toute reprise des
fragments `webhooks` ou `jobs`, le guide `auth` (aucune de ses pages ne décrit
l'enchaînement des écritures, et les régions qu'il cite sont dans les contrôleurs, que
rien ici ne touche).

## Le problème

Vérifié dans le code du 12 septembre : aucun `transaction`, `ConnectionTrait` ni
`TransactionTrait` sous `features/auth/`, et chaque fonction des trois dépôts prend
`&DatabaseConnection`. Les services enchaînent pourtant plusieurs écritures, chacune sur
sa propre connexion du pool, sans rien qui les lie :

| Parcours | Écritures, dans l'ordre | Ce qu'un échec au milieu laisse |
|---|---|---|
| `password::reset` | `consume` → `set_password` → `invalidate_pending` → `close_every_session` | Le jeton est brûlé et le mot de passe n'a pas changé : le lien reçu ne vaut plus rien, et l'utilisateur doit en redemander un. |
| `password::change` | `set_password` → `invalidate_pending` → `close_every_session` → `issue` | Le mot de passe est changé et les sessions — celles d'un compte peut-être compromis — restent ouvertes. |
| `session::refresh` | `rotate` → `issue` | La session a tourné et aucune paire n'a été rendue : le client tient un jeton mort et n'en a pas reçu de neuf. Son prochain essai est un *rejeu* qui ferme tout le compte. |
| `verification::verify` | `consume` → `mark_verified` | Le jeton est brûlé et l'adresse n'est pas vérifiée. |
| `session::revoke_sessions` | `revoke_sessions_of` → `stamp_sessions_revoked` | Les rafraîchissements sont fermés et les jetons d'accès vivent encore un quart d'heure — ce que le commentaire de `close_every_session` promet précisément d'empêcher. |

Le fragment `jobs` fait déjà l'inverse : `enqueue<C: ConnectionTrait>` est appelable avec la
transaction du métier, et c'est toute la raison d'avoir mis la file en base.

## La règle

**Une suite d'écritures dont l'interruption laisse un état pire que l'un ou l'autre de ses
deux bouts se fait dans une transaction.** Les cinq parcours du tableau y passent. Les
dépôts n'ouvrent jamais de transaction : ils la reçoivent, ou reçoivent la connexion, sans
le savoir.

## 1. Les dépôts prennent `&impl ConnectionTrait`

Toutes les fonctions de `repository/{user,one_time_token,refresh_token}.rs.jinja` passent
de `db: &DatabaseConnection` à `db: &impl ConnectionTrait`. `DatabaseConnection` et
`DatabaseTransaction` l'implémentent tous deux : un appelant qui passait la connexion
compile sans changement — les tests d'un CRUD engendré sous `auth`, qui appellent
`auth::repository::create(&db, …)`, et `examples/blog-auth/src/posts/tests.rs` avec eux.

Rien hors du fragment ne dépend de l'ancienne signature : `rbs-core` ne connaît pas les
dépôts du projet (les deux crates sont indépendantes), et les tests d'`integration_auth`
ne cherchent que le préfixe `pub async fn <nom>`, que la forme générique conserve.

`issue` et `close_every_session`, dans `service/mod.rs.jinja`, prennent la même forme :
les trois parcours transactionnels les appellent avec la transaction, et `login` avec la
connexion.

*Rejeté* — `<C: ConnectionTrait>(db: &C)` comme dans `jobs::enqueue` : `jobs` a un second
paramètre de type (`J: Job`) qui justifie la clause `where` ; ici `impl Trait` en position
d'argument dit la même chose sur la ligne de la signature, et c'est ce que l'utilisateur
lira en premier.

*Rejeté* — ne changer que les fonctions appelées dans une transaction : la moitié d'un
dépôt en `&DatabaseConnection` et l'autre en `impl ConnectionTrait` demanderait à chaque
lecteur de savoir laquelle est laquelle, pour n'économiser aucune ligne.

## 2. Les services ouvrent la transaction

`db.begin().await?` — `TransactionTrait` importé dans les trois fichiers de service —
puis les écritures avec `&transaction`, puis `transaction.commit().await?`. Un `?` qui
sort avant le commit abandonne la transaction, que SeaORM annule en la détruisant :
aucune branche d'erreur n'a de `rollback` à écrire.

### La première instruction de chaque transaction est une écriture

C'est la contrainte qui décide de ce qui reste hors de la transaction. SQLite ouvre une
transaction en mode différé : une lecture y prend un verrou partagé, et la première
écriture qui suit doit le promouvoir — ce qu'il refuse (`SQLITE_BUSY`) si une autre
connexion a écrit entre-temps, sans attendre le `busy_timeout`. Une transaction qui
*commence* par écrire prend le verrou d'écriture d'emblée et ses lectures suivantes sont
cohérentes avec lui. PostgreSQL n'a pas ce problème, mais le banc SQLite
(`the_auth_tests_of_the_generated_project_pass_on_sqlite`) joue les tests du fragment en
parallèle, et c'est là que le trou se verrait.

D'où, parcours par parcours :

- **`change`** : `find` et la vérification Argon2 du mot de passe courant restent avant ;
  le hachage du nouveau mot de passe aussi — Argon2 coûte des dizaines de millisecondes,
  et un verrou d'écriture ne se tient pas pendant un calcul. Puis `begin` →
  `set_password` → `invalidate_pending` → `close_every_session` → `find` (le
  rechargement que `issue` exige, une lecture de ses propres écritures) → `issue` →
  `commit`.
- **`reset`** : `find` du jeton reste avant, le hachage aussi. Puis `begin` → `consume`
  → `set_password` → `invalidate_pending` → `close_every_session` → `commit`. Un
  `consume` qui rend `false` sort en `Unauthorized` : la transaction n'a rien écrit et
  se défait d'elle-même.
- **`refresh`** : `find_refresh_token` reste avant. Puis `begin` → `rotate`, et selon
  le résultat : `Done` → `find` → `issue` → `commit` ; `Replayed` →
  `close_every_session` → **`commit`** → `Unauthorized` — la révocation du compte est
  ce qu'un rejeu doit laisser derrière lui, elle se committe avant l'erreur ; `Closed` →
  `Unauthorized`, rien n'a été écrit, la transaction se défait.
- **`verify`** : `find` du jeton avant. Puis `begin` → `consume` → `mark_verified` →
  `commit`.
- **`revoke_sessions`** : `begin` → `close_every_session` → `commit`.

*Rejeté* — faire ouvrir la transaction par `close_every_session` elle-même : elle prend
maintenant un `ConnectionTrait`, qui ne sait pas commencer une transaction, et les trois
parcours qui l'appellent depuis la leur n'en voudraient pas d'une seconde imbriquée.

*Rejeté* — un `rollback` explicite dans les branches d'erreur, comme `dequeue` dans
`jobs` : `dequeue` annule sur un chemin *nominal* (aucune ligne élue), où l'abandon serait
lu comme un oubli. Ici les chemins qui n'atteignent pas le commit sont des erreurs, et le
`?` qui les porte dit assez.

## 3. Ce qui reste hors transaction

- **`login`, `register`, `logout`, `revoke_session`** : une seule écriture chacun.
- **`request_reset` et `verification::request`** : `invalidate_pending` puis `issue`. Un
  échec entre les deux laisse le compte *sans* lien en attente, ce que la demande
  suivante répare — l'état intermédiaire n'est pas pire que le bout d'avant, et la règle
  ne s'applique pas. Les transacter serait cohérent mais n'achèterait rien.

## 4. Les preuves

### Ce que les tests engendrés prouvent

Deux tests `#[ignore]` joints à la base, un par famille d'écritures, qui ouvrent une
transaction, y appellent les dépôts qu'un service enchaîne, l'annulent, et constatent que
rien n'a été écrit :

- `tests/password.rs.jinja` :
  `a_rolled_back_transaction_leaves_the_token_and_the_password_untouched` — `consume`
  puis `set_password` dans la transaction, `rollback`, puis `consume` sur la connexion
  rend encore `true` et `password_hash` est celui de l'inscription.
- `tests/session.rs.jinja` :
  `a_rolled_back_rotation_leaves_the_session_open_and_opens_no_other` — `rotate` puis
  `create_refresh_token` dans la transaction, `rollback`, puis `rotate` sur la connexion
  rend encore `Rotation::Done` et `open_sessions_of` ne compte que la session d'origine.

Avant le changement, ces deux tests **ne compilent pas** — `expected
&DatabaseConnection, found &DatabaseTransaction` — et c'est leur rouge. Ils prouvent que
les dépôts acceptent la transaction du service et qu'un abandon défait tout ce qui y a été
écrit : la mécanique sur laquelle les cinq parcours reposent.

### Ce qu'un test rapide d'`integration_auth` prouve

`the_services_chaining_writes_open_one_transaction` : les trois fichiers de service
engendrés portent `db.begin().await?` et `.commit().await?`, et les trois dépôts
`&impl ConnectionTrait`. Textuel, comme `the_one_time_token_repository_is_written` : il
dit que le fragment est écrit ainsi, pas qu'il se comporte ainsi.

### Ce qui n'est pas prouvé — PARTIEL, à dire dans le rapport

L'atomicité de bout en bout d'un *service* sous un échec **au milieu** de sa suite
d'écritures. Aucune contrainte de la table ne permet de faire échouer `set_password`,
`mark_verified` ou `create_refresh_token` à la demande — un `UPDATE` par identifiant
n'échoue pas, et l'empreinte d'un jeton neuf est aléatoire. L'injection de panne
(`sea-orm` `mock`, fermeture du pool en cours de route) demanderait soit une feature
Cargo dans le projet engendré, soit une course. La preuve s'arrête donc au niveau des
dépôts et à la forme du service.

## 5. L'exemple, la documentation, le journal

- `examples/blog-auth/src/auth/` se régénère par diff entre deux générations (recette
  d'`examples/README.md`), jamais par écrasement ; `integration_examples` est l'oracle.
- Le guide `auth` n'a rien à changer : aucune de ses deux langues ne décrit
  l'enchaînement des écritures, et ses régions citées vivent dans les contrôleurs.
- `CHANGELOG.md` et `CHANGELOG.fr.md` : un item `Fixed` sous `[1.5.0]`, même nombre des
  deux côtés. Un projet qui porte déjà `auth` reçoit la règle en recopiant les dépôts et
  les services du fragment — aucune migration, aucune colonne.
