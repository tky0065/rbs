---
sidebar_position: 4
title: Recevoir un fichier
---

# Recevoir un fichier

C'est le quatrième tutoriel. Il reprend `demo` ; cette page n'a besoin de rien
au-delà de [Préparer le terrain](./setup.md). Le cas : un client dépose un justificatif —
un reçu, un papier à garder — et à la fin de cette page, `PUT /uploads/{id}/content` le
range et répond `204`.

## Ce qu'il vous faut

Rien de plus qu'à [Préparer le terrain](./setup.md) : le même `demo`, toujours en cours
d'exécution avec sa base, et `curl` de nouveau.

## 1. Installer la fonctionnalité

```bash
rbs add storage
```

{/* rbs:transcript cmd="rbs add storage" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add storage
storage : stockage d'objets : un trait à cinq méthodes, deux backends — fichiers et S3

plan pour …/demo

  + src/modules/storage/mod.rs           créé
  + src/modules/storage/files.rs         créé
  + src/modules/storage/s3.rs            créé
  + src/modules/storage/tests/mod.rs     créé
  + src/modules/storage/tests/files.rs   créé
  + src/modules/storage/tests/s3.rs      créé
  + src/modules/mod.rs                   créé
  ~ src/lib.rs                           modifié
  ~ src/state.rs                         modifié
  ~ src/health/controller.rs             modifié
  ~ Cargo.toml                           modifié
  ~ config/default.toml                  modifié
  ~ .env.example                         modifié
  ~ AGENTS.md                            modifié

  7 à créer, 7 à modifier
✓ storage installée — 7 créés, 7 modifiés

  les objets vont sous ./storage : ajoutez-le à .gitignore, ou passez storage.backend à "s3" et recopiez les RBS_STORAGE__* de .env.example
```

Comme chaque brique qu'installe `rbs add`, `storage` ne monte aucune route — elle vous
donne un `Arc<dyn Storage>` sur `AppState` et vous laisse décider quand l'appeler. `add
mail` et `add redis` ne sont pas lancées sur cette page : `demo` ne reçoit que le
stockage. L'exemple que cette page lit,
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop),
porte les trois — ce qui explique que son service tienne des appels que cette page ne
montre jamais.

## 2. Engendrer la ressource

`generate` exige un arbre propre, comme `add` déjà. Commitez ce que `storage` vient
d'écrire, puis transformez le trait ci-dessus en une ressource qui l'appelle :

```bash
git add -A && git commit -q -m "storage installée"
rbs generate crud uploads --fields "title:string,owner_email:string,content_type:string,size:int" --with-upload
```

{/* rbs:transcript cmd="rbs generate crud uploads --fields title:string,owner_email:string,content_type:string,size:int --with-upload" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs add storage" dans="demo" */}
```text
plan pour …/demo

  + src/uploads/mod.rs                                 créé
  + src/uploads/model.rs                               créé
  + src/uploads/dto.rs                                 créé
  + src/uploads/filter.rs                              créé
  + src/uploads/repository.rs                          créé
  + src/uploads/service.rs                             créé
  + src/uploads/controller.rs                          créé
  + src/uploads/tests/mod.rs                           créé
  + src/uploads/tests/lifecycle.rs                     créé
  + src/uploads/tests/errors.rs                        créé
  + src/uploads/tests/filter.rs                        créé
  + src/uploads/tests/content.rs                       créé
  + src/seeds/uploads.rs                               créé
  + migration/src/m20260922_082427_create_uploads.rs   créé
  ~ src/lib.rs                                         modifié
  ~ src/router.rs                                      modifié
  ~ src/openapi.rs                                     modifié
  ~ migration/src/lib.rs                               modifié
  ~ src/seeds/main.rs                                  modifié
  ~ Cargo.toml                                         modifié
  ~ AGENTS.md                                          modifié

  14 à créer, 7 à modifier
✓ uploads générée — 14 créés, 7 modifiés

  la migration m20260922_082427_create_uploads reste à appliquer avant de lancer le projet
```

Deux choses méritent d'être nommées ici. `--with-upload` est ce qui a écrit les trois
routes sur `/uploads/{id}/content` — `PUT`, `GET`, `HEAD` — contre le trait que `storage`
a installé un instant plus tôt ; sans le drapeau, `generate crud` aurait produit les
mêmes six opérations que dans [Votre première ressource](./first-resource.md) et rien
sous `/content`. Et `owner_email` — un champ ordinaire dans `--fields`, sans syntaxe
particulière — a gagné une contrainte `email` dans le DTO engendré pour la seule raison
que son nom finit par `_email` ; rien dans la commande ne le demandait.

## 3. Appliquer la migration et lancer le serveur

```bash
rbs migrate up
```

{/* rbs:transcript cmd="rbs migrate up" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add storage && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs generate crud uploads --fields title:string,owner_email:string,content_type:string,size:int --with-upload" dans="demo" base="oui" extrait="oui" */}
```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.44s
     Running `target/debug/migration up`
✓ migrations appliquées
```

La preuve que la table existe désormais et que le binaire est prêt à la servir : la
migration que `generate` a écrite un instant plus tôt est appliquée, ce qui permet au
serveur ci-dessous de démarrer contre elle.

```bash
cargo run
```

{/* rbs:libre raison="journal d'un serveur qui tourne : le rejeu ne lance aucun serveur, et l'heure change à chaque démarrage" */}
```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

Un démarrage propre est la preuve que les trois gestionnaires de contenu qu'a écrits
`--with-upload` compilent contre le trait de stockage qu'`add storage` a installé — si le
câblage avait été mauvais, cette ligne ne se serait jamais affichée.

## Vérifier

Depuis le second terminal, essayez d'abord une adresse invalide :

```bash
curl -i -X POST http://127.0.0.1:8080/uploads \
  -H 'Content-Type: application/json' \
  -d '{"title":"Justificatif de domicile","owner_email":"pas-un-email","content_type":"application/pdf","size":48213}'
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 422 Unprocessable Entity
content-type: application/problem+json
x-request-id: 01M22SF8V7N4F6RP5DQWPQCBVP
content-length: 143
date: Wed, 09 Sep 2026 09:55:02 GMT

{"type":"about:blank","title":"Validation échouée","status":422,"errors":{"owner_email":["email"]},"request_id":"01M22SF8V7N4F6RP5DQWPQCBVP"}
```

La preuve que `_email` gagne sa contrainte sans que personne ne la déclare : rien dans
`--fields` ne disait « validez cette colonne », et le DTO l'a refusée quand même. Le même
corps, avec une adresse réelle :

```bash
ID=$(curl -s -X POST http://127.0.0.1:8080/uploads \
  -H 'Content-Type: application/json' \
  -d '{"title":"Justificatif de domicile","owner_email":"alice@example.com","content_type":"application/pdf","size":48213}' \
  | jq -r .id)
```

La ligne existe ; son contenu, pas encore :

```bash
curl -i -I http://127.0.0.1:8080/uploads/$ID/content
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 404 Not Found
content-type: application/problem+json
x-request-id: 01M22SFC21MJ86CC06CZC804JP
content-length: 130
date: Wed, 09 Sep 2026 09:55:05 GMT
```

La preuve qu'une ressource et son contenu sont deux choses séparées : `create` ci-dessus
a écrit la ligne, et rien sous `/content` n'existe tant que rien n'y a été déposé. Voici
maintenant le dépôt lui-même, le cas par lequel cette page a commencé :

```bash
curl -i -X PUT http://127.0.0.1:8080/uploads/$ID/content \
  -H 'Content-Type: application/octet-stream' \
  --data-binary 'Justificatif de domicile, PDF simulé pour la démo du tutoriel.'
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 204 No Content
x-request-id: 01M22SFC29MWP1REHVJBA2B7F3
date: Wed, 09 Sep 2026 09:55:05 GMT
```

La preuve que le dépôt a bien eu lieu : le même `HEAD` qui répondait `404` un instant
plus tôt répond maintenant sans corps :

```bash
curl -i -I http://127.0.0.1:8080/uploads/$ID/content
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 204 No Content
x-request-id: 01M22SFC2PJTX8YFDNPH9QYJY8
content-length: 0
date: Wed, 09 Sep 2026 09:55:05 GMT
```

La preuve que la présence et l'absence se lisent différemment, même à travers un simple
`HEAD` : `content-length` tombe de `130` — la taille du problème JSON qu'un `GET` aurait
rendu un instant plus tôt — à un `0` sec, si bien qu'un appelant distingue les deux cas
depuis l'en-tête seul, sans jamais récupérer de corps. Et en le relisant :

```bash
curl -i http://127.0.0.1:8080/uploads/$ID/content
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 200 OK
content-type: application/octet-stream
x-request-id: 01M22SFC2X607FJFHR3X8A4FB4
content-length: 64
date: Wed, 09 Sep 2026 09:55:05 GMT

Justificatif de domicile, PDF simulé pour la démo du tutoriel.
```

La preuve que l'aller-retour tient : les octets que `PUT` a envoyés sont ceux que `GET`
rend, octet pour octet, à travers le magasin que `storage` a choisi — `fs` ici, `s3` sur
un autre `storage.backend`, sans qu'aucune autre ligne de cette page ne change.

## Ce qui a été installé

Trois fichiers, lus depuis
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — les
deux mêmes commandes, lancées dans le même ordre, sur un projet compilé en CI.

:::note
`file-drop` porte les trois briques v0.3 à la fois — `storage`, `mail` et `redis` — sur un
même projet, si bien que son `src/uploads/service.rs` envoie aussi un courriel à la
création et met la liste en cache, ce que cette page n'a installé ni l'un ni l'autre.
L'extrait `contenu` ci-dessous est la part de ce fichier que le stockage seul explique.
:::

### Le trait

Cinq méthodes forment tout le contrat qu'`add storage` promet, quel que soit le backend
qui y répond.

```rust file=examples/file-drop/src/modules/storage/mod.rs region=trait
```

### Le gestionnaire d'écriture

`put_content` est ce qu'`--with-upload` a écrit : le corps arrive en octets bruts, jamais
en JSON, et passe directement au service.

```rust file=examples/file-drop/src/uploads/controller.rs region=put_content
```

### Le service

La ligne est lue avant le dépôt, si bien que le magasin n'accumule jamais un objet
qu'aucune ressource ne réclame — et `exists` répond à la requête `HEAD` plutôt qu'un
`get` dont on jetterait le corps.

```rust file=examples/file-drop/src/uploads/service.rs region=contenu
```

## Pour aller plus loin

- [Storage](../guides/storage.md) couvre les deux backends, la règle d'échappement des
  clés que cette page n'a jamais déclenchée, et tout ce qu'`--with-upload` vous laisse
  faire — les limites de taille, le filtrage MIME, le listage.
- [`rbs add`](../cli/add.md) couvre les autres features que ce projet pourrait
  encore installer.
- [`rbs generate`](../cli/generate.md) a la grammaire complète d'`--with-upload`, y
  compris comment il se combine avec `--role` et `--soft-delete`.
- [Tests](../guides/testing.md) est le harnais contre lequel `uploads/tests/` tourne,
  et le test `round` propre au fragment `storage` que les extraits de cette page
  n'ouvrent jamais.
- [Envoyer un mail](./mail.md) est le tutoriel suivant : dès que le dépôt de cette page
  réussit, son propriétaire reçoit un mail qui le confirme.
