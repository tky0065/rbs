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

{/* rbs:transcript cmd="rbs add auth" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles
auth exige rate-limit : posée avec elle

plan pour …/demo

  + src/auth/mod.rs                                        créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository.rs                                 créé
  + src/auth/service.rs                                    créé
  + src/auth/controller.rs                                 créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests.rs                                      créé
  + migration/src/m20260909_093150_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  ~ .env                                                   modifié
  + src/modules/rate_limit/mod.rs                          créé
  + src/modules/rate_limit/config.rs                       créé
  + src/modules/rate_limit/counter.rs                      créé
  + src/modules/rate_limit/tests.rs                        créé
  + src/modules/mod.rs                                     créé
  ~ src/state.rs                                           modifié
  ~ AGENTS.md                                              modifié

  24 fichiers à écrire
✓ auth installée — 13 fichiers

  rbs migrate up
```

`add` refuse un arbre de travail sale, ce pour quoi la commande ci-dessus ne tourne que
sur un projet fraîchement commité. `auth exige rate-limit : posée avec elle` prouve que
la feature n'arrive pas seule — une route de connexion sans limitation de débit est
exactement le genre de trou qu'un générateur ne devrait pas vous laisser découvrir plus
tard, alors le CLI installe les deux ensemble.

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
sans `--force`. La migration qui vient de s'appliquer est celle qu'`auth` a écrite : deux
nouvelles tables, pour les comptes et les jetons de rafraîchissement.

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
  + src/posts/tests.rs                               créé
  + src/seeds/posts.rs                               créé
  + migration/src/m20260909_093231_create_posts.rs   créé
  ~ src/lib.rs                                       modifié
  ~ src/router.rs                                    modifié
  ~ src/openapi.rs                                   modifié
  ~ migration/src/lib.rs                              modifié
  ~ src/seeds/main.rs                                modifié
  ~ Cargo.toml                                       modifié
  ~ AGENTS.md                                        modifié

  17 fichiers à écrire
✓ posts générée — 10 fichiers

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
curl -s -X POST http://127.0.0.1:8080/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"alice@example.com","password":"un-mot-de-passe-long"}'
```

```text
{"id":"01a08583-7b76-7a01-90a8-c526d25cb1e3","email":"alice@example.com","role":"user","created_at":"2026-09-09T09:33:01.713220Z"}
```

L'inscription produit toujours `"role":"user"` — la preuve qu'aucune route de cette page
ne distribue `admin` sur simple demande ; le compte obtenu ici peut lire `posts`, pas y
écrire.

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
content-length: 100
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Unauthorized","status":401,"request_id":"01M22R73CRW027SYBNBTYJYDBA"}
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
content-length: 97
date: Wed, 09 Sep 2026 09:33:06 GMT

{"type":"about:blank","title":"Forbidden","status":403,"request_id":"01M22R73CYV34S5K7MFAKGHTFY"}
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

```rust file=examples/blog-auth/src/posts/tests.rs region=refus
```

## Pour aller plus loin

- [Authentification](../guides/auth.md) couvre les cinq routes que `add auth` monte, la
  paire de jetons, et l'enum `Role` que cette page n'a utilisée qu'à son défaut.
- [`rbs add`](../cli/add.md) couvre les douze autres features que ce projet pourrait
  encore installer, et le `--force` dont cette page n'a jamais eu besoin.
- [`rbs generate`](../cli/generate.md) a la grammaire complète de `--role`, y compris ce
  qu'il fait sous `--with-upload`, et [ce qu'il faut retirer pour rouvrir une
  route](../guides/auth.md#fermées-par-défaut-à-la-génération).
- [Tests](../guides/testing.md) est le harnais contre lequel `posts/tests.rs` tourne, le
  même que le troisième extrait de cette page étend à la main.
- [Recevoir un fichier](./storage.md) est le tutoriel suivant : un client dépose un
  fichier, et `PUT /uploads/{id}/content` le range.
