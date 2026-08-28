---
sidebar_position: 2
title: Démarrage rapide
---

# Démarrage rapide

Cette page mène d'un répertoire vide à une API CRUD qui répond sur `localhost:8080`, en
huit commandes. Chaque bloc de sortie ci-dessous a été recopié d'une exécution réelle —
si votre terminal affiche la même chose, vous n'avez pas dévié — aux durées, aux
identifiants et aux dates près, qui sont les vôtres. Une seule chose a été retirée des
blocs : le chemin absolu du répertoire où l'exécution a eu lieu, noté `…/demo`.

En 0.1, le CLI parle français : `✓ demo créé — 16 fichiers` est une ligne de succès. Les
options, les noms de fichiers et le code généré, eux, sont les mêmes quelle que soit la
langue.

## Ce qu'il vous faut

- **Rust stable**, édition 2024. L'exécution ci-dessous a utilisé `rustc 1.96.0`.
- **PostgreSQL 18 ou plus.** La ligne Docker ci-dessous suffit ; un serveur existant fait
  tout aussi bien l'affaire, du moment que vous pouvez pointer une URL dessus — et que
  c'est bien une 18. Les migrations générées posent `uuidv7()` en défaut de clé primaire,
  fonction absente des versions antérieures.
- **curl**, ou n'importe quel client HTTP, pour la dernière section.
- **Un clone du dépôt rbs.** La 0.1 n'est pas encore sur crates.io, ce qui a deux
  conséquences que vous rencontrerez plus bas.

## Installer le CLI

Une fois la 0.1 publiée, ce sera `cargo install rbs-cli` : le paquet s'appelle `rbs-cli`,
la commande qu'il installe `rbs`, et le nom `rbs` sur crates.io appartient à un projet
sans rapport. En attendant, elle se construit depuis le dépôt :

```bash
git clone https://github.com/tky0065/rbs
cd rbs
cargo install --path crates/rbs-cli
cd ..
```

Un exécutable `rbs` atterrit dans `~/.cargo/bin`, accompagné d'une seconde copie nommée
`rbs-cli`, pour le cas que décrit l'encart plus bas. Le dernier `cd` vous fait ressortir du
clone : la suite de cette page travaille depuis le répertoire qui le *contient*, si bien
que le projet que vous allez créer atterrit à côté du clone et non dedans. Vérifiez que
le binaire répond :

```bash
rbs --version
```

```text
rbs 0.1.0
```

:::note

L'écosystème Ruby distribue un outil sans rapport, lui aussi nommé `rbs`. Si
`rbs --version` affiche quelque chose comme `rbs 3.10.0`, c'est celui-là qui gagne sur
votre `PATH`. Rien à réorganiser : l'installation a déposé le même programme une seconde
fois, sous un nom que personne d'autre ne revendique.

```bash
rbs-cli --version
```

Utilisez `rbs-cli` partout où cette page écrit `rbs`. C'est le même binaire, et il
s'annonce sous le nom par lequel vous l'appelez — son `--help` reste donc fidèle à ce que
vous tapez.

:::

## Démarrer une base

rbs ne gère pas votre base : il attend une URL qui répond. Le plus court chemin pour en
avoir une :

```bash
docker run --rm -d --name rbs-demo \
  -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=demo \
  -p 5432:5432 postgres:18
```

Laissez-la tourner jusqu'à la fin de cette page. `docker stop rbs-demo` la supprime une
fois que vous avez fini — le conteneur a été lancé avec `--rm`, rien ne subsiste.

## Créer le projet

```bash
rbs new demo --yes \
  --database-url postgres://rbs:rbs@localhost:5432/demo \
  --core-path rbs/crates/rbs-core
```

```text
✓ demo créé — 16 fichiers

  cd demo
  cargo run          # la base visée est dans .env
```

`--core-path` est la seconde conséquence de la 0.1 non publiée : sans lui, le manifeste
généré réclame `rbs-core = "0.1.0"` sur crates.io, où il n'existe pas encore, et
`cargo build` échoue à la résolution. Pointez l'option sur le répertoire
`crates/rbs-core` du clone que vous venez de faire — un chemin relatif convient, le CLI
inscrit l'absolu dans `Cargo.toml`.

`--yes` répond à chaque question par son défaut — ici, la feature `health` et rien
d'autre. Sans lui, le CLI demande, dans l'ordre, l'URL de la base si `--database-url`
manque, puis les features optionnelles à installer. Il refuse aussi de tourner sans
terminal où poser ses questions : c'est pourquoi un script ou un job de CI a besoin de
`--yes` :

```text
erreur : aucun terminal interactif pour poser les questions : relancez avec `--yes` pour prendre les défauts, ou donnez les réponses en flags — le nom en argument, `--database-url` et `--with`
```

Seize fichiers, et aucun n'est une boîte noire :

- `src/main.rs`, `src/router.rs`, `src/state.rs`, `src/openapi.rs` — le montage.
- `src/health/` — une première feature, pour que la forme soit visible avant d'en
  générer une.
- `src/seeds/` — un second binaire, `seed`, que `rbs seed` lance.
- `migration/` — une seconde crate, qui porte les migrations.
- `config/default.toml` et `config/development.toml` — hôte, port, taille du pool.
- `.env` — l'URL de la base et les réglages de logs, tenus hors de Git.
- `.env.example` — les mêmes clés sans secret, versionnées.

Le `.env` écrit par la commande porte l'URL que vous avez passée :

```text
RBS_ENV=development
RBS_DATABASE__URL=postgres://rbs:rbs@localhost:5432/demo

RBS_LOG_FORMAT=pretty
RUST_LOG=info,demo=debug
```

## La première migration

```bash
cd demo
rbs migrate up
```

La première exécution compile la crate `migration`, ce qui prend une minute ; ce sont
les dernières lignes qui comptent :

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.91s
     Running `target/debug/migration up`
✓ migrations appliquées
```

Il n'y a encore rien à migrer — la commande crée la table dont SeaORM se sert pour
suivre les migrations appliquées. La lancer maintenant, c'est apprendre que l'URL de
`.env` est la bonne avant qu'un seul fichier généré n'en dépende.

## Générer une feature CRUD

```bash
rbs generate crud articles --fields "title:string,body:text,published:bool"
```

La commande affiche ce qu'elle compte faire, puis le fait :

```text
plan pour …/demo

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + migration/src/m20260826_214305_create_articles.rs   créé
  ~ src/main.rs                                         modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ Cargo.toml                                          modifié

  13 fichiers à écrire
✓ articles générée — 8 fichiers

  la migration m20260826_214305_create_articles reste à appliquer avant de lancer le projet
```

Votre fichier de migration portera un autre horodatage : le nom est construit à l'instant
où vous lancez la commande. Le reste est identique.

Deux choses à remarquer. L'entité et sa migration viennent toutes deux de `--fields`,
sans base démarrée et sans introspection — le schéma est déclaré une fois, dans la
commande. Et les quatre lignes `~` sont des modifications de fichiers qui vous
appartiennent : le CLI a inséré dans des ancres en commentaires (`// <rbs:features>`,
`<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`) plutôt que de réécrire votre code.
Supprimez une ancre et le CLI cesse d'y écrire : il affiche le bloc à coller.

Appliquez la nouvelle migration :

```bash
rbs migrate up
rbs migrate status
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.07s
     Running `target/debug/migration up`
✓ migrations appliquées
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running `target/debug/migration status`
  ✓ m20260826_214305_create_articles   appliquée
```

## Ce que le générateur a écrit

Six fichiers par feature, plus ses tests, avec une seule direction de dépendance :
contrôleur → service → dépôt → modèle. Voici le gestionnaire de `POST /articles`, lu
dans [`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
la même feature, produite par la même commande :

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

Le contrôleur passe la requête au service et traduit le résultat en code de statut ;
c'est tout ce qu'il fait. Le service ne voit jamais de `DatabaseConnection`, et le
contrôleur ne construit jamais de requête SeaORM. Rien ici ne porte la mention « ne pas
modifier » : règles de validation, points d'entrée supplémentaires et logique métier
vivent dans ces fichiers.

## Le lancer

```bash
cargo run
```

La première compilation est longue — c'est tout l'arbre Axum, SeaORM et utoipa. Une fois
terminée :

```text
21:43:30  INFO   demo                démarrage  adresse=127.0.0.1:8080
```

C'est le formateur de logs `pretty` : horodatage, niveau, cible, message, champs. Passez
`RBS_LOG_FORMAT=json` dans `.env` quand c'est un collecteur, et non un humain, qui lit la
sortie.

## Premières requêtes

Laissez le serveur tourner et ouvrez un second terminal.

```bash
curl -i http://127.0.0.1:8080/health
```

```text
HTTP/1.1 200 OK
content-type: application/json
x-request-id: 01M100EQEJX68AKBH79CHX5R6B
content-length: 42
date: Wed, 26 Aug 2026 21:43:39 GMT

{"status":"ok","checks":{"database":"ok"}}
```

`/health` est venu avec le projet et vérifie la base, pas seulement le processus.
L'en-tête `x-request-id` figure sur chaque réponse, et la même valeur se retrouve dans la
ligne de log de la requête.

```bash
curl -i -X POST http://127.0.0.1:8080/articles \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier article","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 201 Created
content-type: application/json
x-request-id: 01M100EQEWHSSAMH7N54N4CEG3
content-length: 191
date: Wed, 26 Aug 2026 21:43:39 GMT

{"id":"01a04007-5dde-7103-97cd-6531d6f67704","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-26T21:43:39.741644Z","updated_at":"2026-08-26T21:43:39.741644Z"}
```

L'identifiant et les horodatages viennent du serveur : `id`, `created_at` et `updated_at`
ne font pas partie du corps de la requête.

```bash
curl http://127.0.0.1:8080/articles
```

```text
{"data":[{"id":"01a04007-5dde-7103-97cd-6531d6f67704","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-26T21:43:39.741644Z","updated_at":"2026-08-26T21:43:39.741644Z"}],"meta":{"page":1,"per_page":20,"total":1,"total_pages":1}}
```

Les collections sont paginées par défaut, sous `data` et `meta`. `?page=` et `?per_page=`
s'y déplacent. Les trois routes restantes — `GET`, `PUT` et `DELETE` sur
`/articles/{id}` — ont été générées en même temps.

Pendant ce temps, le terminal du serveur affiche une ligne par requête :

```text
21:43:39  INFO   rbs_core::trace     request  status=200 latency_ms=0.711291 request_id=01M100EQEJX68AKBH79CHX5R6B method=GET path=/health
21:43:39  INFO   rbs_core::trace     request  status=201 latency_ms=3.819458 request_id=01M100EQEWHSSAMH7N54N4CEG3 method=POST path=/articles
21:43:39  INFO   rbs_core::trace     request  status=200 latency_ms=36.957833 request_id=01M100EQF8K15VW6E8PNTV9JGY method=GET path=/articles
```

## Le document OpenAPI

Le document est construit depuis les annotations posées sur les gestionnaires : il décrit
donc les routes qui existent, et non celles dont quelqu'un s'est souvenu. Ouvrez
`http://127.0.0.1:8080/docs` pour Swagger UI, ou lisez le document lui-même :

```bash
curl http://127.0.0.1:8080/api-docs/openapi.json
```

Son `paths` contient désormais `/health`, `/articles` et `/articles/{id}`. Les deux
routes se coupent depuis `[docs]` dans `config/default.toml` ; désactivez-les en
production.

## Diagnostiquer un projet

Quand quelque chose cloche, demandez avant de deviner :

```bash
rbs doctor
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running `target/debug/migration version`
  ✓ ancres     les 7 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✓ base       PostgreSQL 18.6 répond sur localhost:5432
✓ le projet est sain
```

Quatre vérifications : les ancres sont toujours en place, `.env` porte chaque clé que
déclare `.env.example`, le projet et `rbs-core` s'accordent avec la version du CLI, et la
base répond.

## Pour aller plus loin

- [Logs](./guides/logs.md) — les deux formateurs, et quoi mettre dans `RUST_LOG`.
- Le code généré est le vôtre : ouvrez `src/articles/service.rs` et ajoutez-y une règle.
- `rbs generate crud --dry-run` affiche le plan sans rien écrire : c'est la façon la
  moins coûteuse de voir ce que produit un jeu de `--fields`.
- La [feuille de route](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) liste ce que
  couvre la 0.1 et ce qui en est délibérément exclu.
