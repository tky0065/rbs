---
sidebar_position: 3
title: Fermer l'API aux inconnus
---

# Fermer l'API aux inconnus

C'est le troisième des neuf tutoriels. Il reprend `demo` juste après [Votre première
ressource](./first-resource.md) — en cours d'exécution, avec le CRUD `articles` de cette
page — et le ferme à qui il ne connaît pas. Le cas : un blog où chaque visiteur peut lire
un billet, et seul un administrateur peut en écrire un. Plutôt que de rouvrir
`articles`, cette page engendre une seconde ressource, `posts` : ce qui change ici, c'est
la protection, pas la ressource, et un CRUD engendré avant que la garde n'existe ne dit
rien de la garde elle-même.

## Ce qu'il vous faut

Rien de plus qu'à [Préparer le terrain](./setup.md) : le même `demo` en cours
d'exécution, et `curl` de nouveau, pour trois requêtes au lieu de deux.

## 1. Installer la fonctionnalité

```bash
rbs add auth
```

{/* rbs:transcript cmd="rbs add auth" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles
auth exige mail, rate-limit : posée avec elle

plan pour …/demo

  + src/modules/mail/mod.rs                                créé
  + src/modules/mail/config.rs                             créé
  + src/modules/mail/template.rs                           créé
  + src/modules/mail/service.rs                            créé
  + src/modules/mail/tests.rs                              créé
  + templates/mail/bienvenue.html                          créé
  + src/modules/mod.rs                                     créé
  ~ src/lib.rs                                             modifié
  ~ src/state.rs                                           modifié
  ~ docker-compose.yml                                     modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  + src/modules/rate_limit/mod.rs                          créé
  + src/modules/rate_limit/config.rs                       créé
  + src/modules/rate_limit/counter.rs                      créé
  + src/modules/rate_limit/tests.rs                        créé
  ~ src/router.rs                                          modifié
  + src/auth/mod.rs                                        créé
  + src/auth/config.rs                                     créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository/mod.rs                             créé
  + src/auth/repository/user.rs                            créé
  + src/auth/repository/refresh_token.rs                   créé
  + src/auth/repository/one_time_token.rs                  créé
  + src/auth/service/mod.rs                                créé
  + src/auth/service/session.rs                            créé
  + src/auth/service/account.rs                            créé
  + src/auth/service/password.rs                           créé
  + src/auth/service/verification.rs                       créé
  + src/auth/controller/mod.rs                             créé
  + src/auth/controller/session.rs                         créé
  + src/auth/controller/account.rs                         créé
  + src/auth/controller/password.rs                        créé
  + src/auth/controller/verification.rs                    créé
  + templates/mail/reinitialisation.html                   créé
  + templates/mail/verification.html                       créé
  + templates/mail/inscription.html                        créé
  + src/auth/guard.rs                                      créé
  + src/seeds/admin.rs                                     créé
  + src/auth/tests/mod.rs                                  créé
  + src/auth/tests/account.rs                              créé
  + src/auth/tests/change.rs                               créé
  + src/auth/tests/guard.rs                                créé
  + src/auth/tests/http.rs                                 créé
  + src/auth/tests/login.rs                                créé
  + src/auth/tests/logout.rs                               créé
  + src/auth/tests/openapi.rs                              créé
  + src/auth/tests/refresh.rs                              créé
  + src/auth/tests/registration.rs                         créé
  + src/auth/tests/replay.rs                               créé
  + src/auth/tests/reset.rs                                créé
  + src/auth/tests/roles.rs                                créé
  + src/auth/tests/sessions.rs                             créé
  + src/auth/tests/tokens.rs                               créé
  + src/auth/tests/verification.rs                         créé
  + migration/src/m20260920_112413_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/openapi.rs                                         modifié
  ~ src/seeds/main.rs                                      modifié
  ~ config/development.toml                                modifié
  ~ .env                                                   modifié
  ~ AGENTS.md                                              modifié

  51 à créer, 13 à modifier
✓ auth installée — 51 créés, 13 modifiés

  rbs migrate up

  rbs seed pose le compte d'administration dans la table des comptes : ADMIN_EMAIL (admin@demo.test) et ADMIN_PASSWORD, tiré dans votre .env, sont les identifiants que l'écran de connexion demande
```

`add` refuse un arbre de travail sale, ce pour quoi la commande ci-dessus ne tourne que
sur un projet fraîchement commité. `auth exige mail, rate-limit : posée avec elle` prouve
que la feature n'arrive pas seule — une route de connexion sans limitation de débit, et
une réinitialisation de mot de passe sans moyen d'envoyer l'e-mail, sont exactement le
genre de trou qu'un générateur ne devrait pas vous laisser découvrir plus tard, alors le
CLI installe les trois ensemble.

## 2. Appliquer la migration

```bash
git add -A && git commit -q -m "auth installée"
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.74s
     Running `target/debug/migration up`
✓ migrations appliquées
```

`generate` et `add` exigent tous deux un arbre propre, d'où le commit en premier — une
preuve de rien en soi, mais ce qui rend les deux commandes suivantes capables de tourner
sans `--force`. La migration qui vient de s'appliquer est celle qu'`auth` a écrite : trois
nouvelles tables — les comptes, les jetons de rafraîchissement, et les jetons à usage
unique derrière les liens de mot de passe et de vérification plus bas.

## 3. Engendrer une ressource protégée

```bash
rbs generate crud posts --fields "title:string,body:text,published:bool" --role admin
```

```text
plan pour …/demo

  + src/posts/mod.rs                                 créé
  + src/posts/model.rs                               créé
  + src/posts/dto.rs                                 créé
  + src/posts/filter.rs                              créé
  + src/posts/repository.rs                          créé
  + src/posts/service.rs                             créé
  + src/posts/controller.rs                          créé
  + src/posts/tests/mod.rs                           créé
  + src/posts/tests/lifecycle.rs                     créé
  + src/posts/tests/errors.rs                        créé
  + src/posts/tests/filter.rs                        créé
  + src/posts/tests/access.rs                        créé
  + src/seeds/posts.rs                               créé
  + migration/src/m20260909_093231_create_posts.rs   créé
  ~ src/lib.rs                                       modifié
  ~ src/router.rs                                    modifié
  ~ src/openapi.rs                                   modifié
  ~ migration/src/lib.rs                             modifié
  ~ src/seeds/main.rs                                modifié
  ~ Cargo.toml                                       modifié
  ~ AGENTS.md                                        modifié

  14 à créer, 7 à modifier
✓ posts générée — 14 créés, 7 modifiés

  la migration m20260909_093231_create_posts reste à appliquer avant de lancer le projet
```

Voici le cœur de cette page : sur un projet portant `auth`, `generate crud` ferme
**toutes** les routes qu'il écrit au seuil le plus bas — `list`, `find`, `filter`
compris — avant même que `--role` ne soit lu. Ce que fait le drapeau est étroit, pas
ouvrant : il relève les trois écritures (`create`, `update`, `delete`) au rôle nommé, et
ne touche à rien d'autre. Retirez `--role admin` de la commande ci-dessus, et le plan
aurait la même allure, mais lire `/posts` demanderait aussi un jeton — le seul rôle du
drapeau est de distinguer `create`, `update` et `delete` du reste, pas de décider si la
fonctionnalité est protégée du tout.

## 4. Appliquer la dernière migration et lancer le serveur

```bash
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.12s
     Running `target/debug/migration up`
✓ migrations appliquées
```

Seule la table `posts` était en attente cette fois — le premier `migrate up` avait déjà
appliqué la paire d'`auth`. Relancez le serveur pour que les nouvelles routes soient
compilées :

```bash
cargo run
```

```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

Un démarrage propre ici prouve que le binaire compile désormais `auth` et la garde
`--role admin` sur `posts` dans un seul routeur — si l'un des deux avait été mal câblé,
cette ligne ne se serait jamais affichée.

## Vérifier

Depuis le second terminal, créez un compte et échangez-le contre un jeton :

```bash
curl -i -X POST http://127.0.0.1:8080/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com","password":"un-mot-de-passe-long"}'
```

```text
HTTP/1.1 202 Accepted
content-length: 0
```

202 sans corps, que l'adresse soit déjà prise ou non : la réponse ne dit pas laquelle, et
une adresse prise vaut à son titulaire un courriel d'avertissement plutôt qu'un second
compte. Le compte existe dès que le 202 arrive, et c'est toujours un `user` — aucune route
de cette page ne distribue `admin` sur simple demande ; le compte obtenu ici peut lire
`posts`, pas y écrire.

Sur ce poste de travail, il se connecte tout de suite : `config/development.toml`, que
`RBS_ENV=development` pose par-dessus les défauts, met `login_requires_verification` à
`false`. Le défaut versionné est `true`, et c'est lui qu'un déploiement porte — là, un
compte reste à l'écart tant que son adresse n'est pas prouvée, parce qu'une connexion avec
le mot de passe tout juste envoyé distinguerait sinon une adresse neuve d'une prise, comme
l'explique le guide auth. La preuve arrive par courriel, et elle vaut d'être déroulée une
fois ici. `auth` arrive avec `mail`, et le
SMTP par défaut de `mail` est Mailpit — le service `mailpit` que `docker-compose.yml`
porte déjà, qui attrape chaque message que le projet envoie sans qu'aucune vraie boîte
n'existe de l'autre côté. Ouvrez [`http://localhost:8025`](http://localhost:8025) dans un
navigateur et laissez-le ouvert : un message intitulé *Confirmez votre adresse* y attend,
avec un lien de la forme `http://localhost:5173/verify-email#token=…`. Recopiez le jeton
qu'il porte :

```bash
curl -i -X POST http://127.0.0.1:8080/auth/verify-email \
  -H 'Content-Type: application/json' \
  -d '{"token":"<le token du lien>"}'
```

```text
HTTP/1.1 204 No Content
```

Un lien périme après `verification_ttl_secs` ; un client qui le trouve expiré en demande un
neuf par `POST /auth/resend-verification`. L'adresse est prouvée — ce qu'un déploiement
aurait exigé — et le mot de passe ouvre le compte :

```bash
TOKEN=$(curl -s -X POST http://127.0.0.1:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com","password":"un-mot-de-passe-long"}' \
  | jq -r .access_token)
```

Trois requêtes séparent les deux régimes que le reste de cette page a décrits. Aucun
jeton du tout, sur la route que le drapeau a relevée :

```bash
curl -i -X POST http://127.0.0.1:8080/posts \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier post","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 401 Unauthorized
content-type: application/problem+json
x-request-id: 01M22R73CRW027SYBNBTYJYDBA
content-length: 112
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Authentification requise","status":401,"request_id":"01M22R73CRW027SYBNBTYJYDBA"}
```

L'extracteur refuse celle-ci avant qu'aucun gestionnaire ne tourne — la preuve qu'un
jeton absent n'atteint jamais `require_role`. Voici maintenant la même écriture, avec un
jeton réel mais insuffisant :

```bash
curl -i -X POST http://127.0.0.1:8080/posts \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title":"Premier post","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 403 Forbidden
content-type: application/problem+json
x-request-id: 01M22R73CYV34S5K7MFAKGHTFY
content-length: 103
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Accès interdit","status":403,"request_id":"01M22R73CYV34S5K7MFAKGHTFY"}
```

`401` est devenu `403` — la preuve que l'appelant est désormais identifié, et refusé par
`require_role` lui-même, dans le gestionnaire, plutôt que par l'extracteur. Le même
jeton lit :

```bash
curl -i http://127.0.0.1:8080/posts \
  -H "Authorization: Bearer $TOKEN"
```

```text
HTTP/1.1 200 OK
content-type: application/json
x-request-id: 01M22R73D3PS6VQHTRP0GSD8MD
content-length: 69
date: Wed, 09 Sep 2026 09:33:06 GMT

{"data":[],"meta":{"page":1,"per_page":20,"total":0,"total_pages":0}}
```

La preuve que `--role admin` n'a jamais touché cette route : le même jeton `user`,
refusé sur l'écriture, lit la liste vide sans se plaindre.

Pour passer l'écriture, il faut un compte portant `admin`, et aucune route n'en distribue.
`rbs seed` l'écrit : `auth` a déposé `src/seeds/admin.rs`, qui prend `ADMIN_EMAIL` et
`ADMIN_PASSWORD` dans votre `.env` — `rbs add auth` les y a posées — et crée le compte,
son adresse déjà vérifiée. Connectez-vous avec ces deux-là plutôt qu'avec celles d'Alice,
et l'écriture répond 201.

## Changer, réinitialiser

Gardez l'onglet Mailpit ouvert : les requêtes suivantes y déposent encore quelque chose.

Alice, qui tient toujours `$TOKEN` ci-dessus, change son propre mot de passe :

```bash
curl -s -X POST http://127.0.0.1:8080/auth/change-password \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"current_password":"un-mot-de-passe-long","new_password":"un-second-mot-de-passe-long"}'
```

```text
{"access_token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...","refresh_token":"MKySJ39zGPdyiC-eIjOT01s0xJTMI5Zvhn8JByDqwWI","token_type":"Bearer","expires_in":900}
```

200, et non 204 : la route rend une paire neuve, parce qu'elle vient de révoquer toutes
les sessions du compte, `$TOKEN` comprise — rien dans la requête ne dit laquelle l'a
émise, aucune n'est donc épargnée. L'ancien `$TOKEN` est mort dès que cette réponse
arrive.

Supposons maintenant qu'Alice oublie ce mot de passe neuf :

```bash
curl -i -X POST http://127.0.0.1:8080/auth/forgot-password \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com"}'
```

```text
HTTP/1.1 202 Accepted
content-length: 0
```

202 que l'adresse existe ou non — regardez l'onglet Mailpit, un message y attend, intitulé
*Réinitialisation de votre mot de passe*, avec un lien de la forme
`http://localhost:5173/reset-password#token=…`. Recopiez le jeton qu'il porte :

```bash
curl -i -X POST http://127.0.0.1:8080/auth/reset-password \
  -H 'Content-Type: application/json' \
  -d '{"token":"<le token du lien>","new_password":"un-troisieme-mot-de-passe-long"}'
```

```text
HTTP/1.1 204 No Content
```

204, et toutes les sessions du compte sont révoquées à nouveau — se connecter à partir
d'ici exige le mot de passe qui vient d'être posé.

## Ce qui a été installé

Trois fichiers, lus depuis
[`examples/blog-auth`](https://github.com/tky0065/rbs/tree/main/examples/blog-auth) —
les deux mêmes commandes, lancées dans le même ordre, sur un projet compilé en CI.

### L'écriture engendrée

`create` est ce que `--role admin` a façonné : une `Identity` extraite avant que le
gestionnaire ne tourne, et `require_role` appelée en premier dedans.

```rust file=examples/blog-auth/src/posts/controller.rs region=create
```

### La garde

`require_role` vit dans votre projet, pas dans `rbs-core` — le noyau sait qu'un appelant
porte un rôle, en clair, mais pas quels sont les rôles ni ce qu'une route exige. Elle
compare un seuil, pas une égalité, ce pour quoi un jeton `Admin` satisfait une route qui
ne demande que `User`.

```rust file=examples/blog-auth/src/auth/guard.rs region=require_role
```

### Le test qui distingue les refus

Deux 401 et un 403 se ressemblent, à ne juger que le code de statut. Ce test est celui
qui fixe lequel est lequel — un jeton `user` refusé sur l'écriture, et qui lit quand
même.

```rust file=examples/blog-auth/src/posts/tests/access.rs region=refus
```

## Pour aller plus loin

- [Authentification](../guides/auth.md) couvre les quinze routes que `add auth` monte, la
  paire de jetons, et l'enum `Role` que cette page n'a utilisée qu'à son défaut.
- [`rbs add`](../cli/add.md) couvre les dix autres features que ce projet pourrait
  encore installer, et le `--force` dont cette page n'a jamais eu besoin.
- [`rbs generate`](../cli/generate.md) a la grammaire complète de `--role`, y compris ce
  qu'il fait sous `--with-upload`, et [ce qu'il faut retirer pour rouvrir une
  route](../guides/auth.md#fermées-par-défaut-à-la-génération).
- [Tests](../guides/testing.md) est le harnais contre lequel `posts/tests/` tourne, le
  même que le troisième extrait de cette page étend à la main.
- [Recevoir un fichier](./storage.md) est le tutoriel suivant : un client dépose un
  fichier, et `PUT /uploads/{id}/content` le range.
