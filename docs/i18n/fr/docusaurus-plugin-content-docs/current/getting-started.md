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

Le CLI parle français : `✓ demo créé — 21 fichiers` est une ligne de succès. Les
options, les noms de fichiers et le code généré, eux, sont les mêmes quelle que soit la
langue.

## Ce qu'il vous faut

- **Rust stable**, édition 2024. L'exécution ci-dessous a utilisé `rustc 1.96.0`.
- **Docker, avec Compose.** `rbs new` écrit un `docker-compose.yml` portant la base que
  décrit l'URL du projet ; `docker compose up -d` est ce qui la démarre, quelques
  sections plus bas. Un serveur existant fait tout aussi bien l'affaire à la place, du
  moment que vous pouvez pointer une URL dessus — voir
  [les deux cas où le compose ne s'écrit pas](./cli/new.md).
- **PostgreSQL 14 ou plus**, peu importe comment vous vous le procurez. Ce plancher est la
  plus ancienne version encore corrigée côté sécurité : les modèles générés posent
  eux-mêmes leur identifiant v7, et rien de ce qu'exécute un projet ne réclame de
  `uuidv7()` au serveur.
- **curl**, ou n'importe quel client HTTP, pour la dernière section.

## Installer le CLI

Le paquet s'appelle `rbs-cli`, la commande qu'il installe `rbs`, et le nom `rbs` sur
crates.io appartient à un projet sans rapport :

```bash
cargo install rbs-cli
```

Un exécutable `rbs` atterrit dans `~/.cargo/bin`, accompagné d'une seconde copie nommée
`rbs-cli`, pour le cas que décrit l'encart plus bas. Vérifiez que le binaire répond :

```bash
rbs --version
```

```text
rbs 1.1.0
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

## Créer le projet

```bash
rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
```

{/* rbs:transcript cmd="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" */}
```text
✓ demo créé — 21 fichiers

  cd demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

Le manifeste écrit par la commande dépend de `rbs-core` publié sur crates.io : il n'y a
rien à faire pointer sur quoi que ce soit, ni rien à construire d'abord — `cargo build`
le résout comme n'importe quelle autre dépendance. Le seul cas qui demande davantage est
la contribution au noyau lui-même, et [`rbs new --core-path`](./cli/new.md) s'en charge.

`--yes` répond à chaque question par son défaut — ici, la feature `health` et rien
d'autre. Sans lui, le CLI demande, dans l'ordre, l'URL de la base si `--database-url`
manque, puis les features optionnelles à installer. Il refuse aussi de tourner sans
terminal où poser ses questions : c'est pourquoi un script ou un job de CI a besoin de
`--yes` :

```text
erreur : aucun terminal interactif pour poser les questions : relancez avec `--yes` pour prendre les défauts, ou donnez les réponses en flags — le nom en argument, `--database-url` et `--with`
```

Vingt fichiers, et aucun n'est une boîte noire :

- `src/main.rs`, `src/router.rs`, `src/state.rs`, `src/openapi.rs` — le montage.
- `src/lib.rs` — la bibliothèque sur laquelle s'appuient `src/main.rs` et
  `src/seeds/main.rs`. Ce sont deux binaires distincts, et aucun ne peut atteindre les
  modules de l'autre directement ; la bibliothèque est ce qu'ils partagent.
- `src/health/` — une première feature, pour que la forme soit visible avant d'en
  générer une.
- `src/seeds/` — un second binaire, `seed`, que `rbs seed` lance.
- `migration/` — une seconde crate, qui porte les migrations.
- `config/default.toml`, `config/development.toml` et `config/production.toml` — hôte,
  port, taille du pool.
- `docker-compose.yml` — la base du projet, construite depuis l'URL ci-dessous.
- `.env` — l'URL de la base et les réglages de logs, tenus hors de Git.
- `.env.example` — les mêmes clés sans secret, versionnées.

Le `.env` écrit par la commande porte l'URL que vous avez passée :

```text
RBS_ENV=development
RBS_DATABASE__URL=postgres://rbs:secret@localhost:5432/demo

# Le service `db` de docker-compose.yml interpole ces clés depuis ce fichier : le compose
# est versionné, celui-ci est ignoré par git.
POSTGRES_USER=rbs
POSTGRES_PASSWORD=secret
POSTGRES_DB=demo

RBS_LOG_FORMAT=pretty
RUST_LOG=info,demo=debug
```

## Démarrer la base

`docker-compose.yml` nomme les identifiants et la base plutôt qu'il ne les écrit : le
fichier est versionné, `.env` ne l'est pas, et les valeurs restent du côté de la ligne que
Git ne franchit pas. Le port publié est lu dans la même URL, rien n'est retapé :

```yaml
name: demo

services:
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: "${POSTGRES_USER}"
      POSTGRES_PASSWORD: "${POSTGRES_PASSWORD}"
      POSTGRES_DB: "${POSTGRES_DB}"
    # Le port publié est celui du .env : c'est ce qui rend `docker compose up -d` suivi
    # de `cargo run` vrai sans recopier une valeur d'un fichier à l'autre. Le conflit
    # avec un PostgreSQL déjà installé sur la machine se règle en changeant les deux.
    ports:
      - "5432:5432"
    # PostgreSQL 18 place ses données sous /var/lib/postgresql/18/docker : c'est le
    # répertoire parent qui se monte, et non le /var/lib/postgresql/data des versions
    # précédentes, qui ne persisterait rien.
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", 'pg_isready -U "$$POSTGRES_USER" -d "$$POSTGRES_DB"']
      interval: 2s
      timeout: 3s
      retries: 30

  # <rbs:services>
  # </rbs:services>

volumes:
  pgdata:
```

```bash
docker compose up -d --wait
```

```text
 Network demo_default  Creating
 Network demo_default  Created
 Volume demo_pgdata  Creating
 Volume demo_pgdata  Created
 Container demo-db-1  Creating
 Container demo-db-1  Created
 Container demo-db-1  Starting
 Container demo-db-1  Started
 Container demo-db-1  Waiting
 Container demo-db-1  Healthy
```

`--wait` est ce qui rend la commande suivante de cette page — la première migration —
sûre à lancer tout de suite : sans lui, `docker compose up -d` rend la main dès que le
conteneur démarre, avant que PostgreSQL soit prêt à accepter une connexion.

Rien ne s'écrit ici pour un projet SQLite — il n'y a pas de serveur à démarrer — ni pour
une URL dont l'hôte n'est pas local : le conteneur ne ferait que doubler une base déjà
joignable ailleurs. Les deux cas sont couverts dans [`rbs new`](./cli/new.md).

## La première migration

```bash
cd demo
rbs migrate up
```

La première exécution compile la crate `migration`, ce qui prend une minute ; ce sont
les dernières lignes qui comptent :

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.48s
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

{/* rbs:transcript cmd="rbs generate crud articles --fields title:string,body:text,published:bool" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" */}
```text
plan pour …/demo

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260830_110245_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  17 fichiers à écrire
✓ articles générée — 10 fichiers

  la migration m20260830_110245_create_articles reste à appliquer avant de lancer le projet
```

Votre fichier de migration portera un autre horodatage : le nom est construit à l'instant
où vous lancez la commande. Le reste est identique.

Deux choses à remarquer. L'entité et sa migration viennent toutes deux de `--fields`,
sans base démarrée et sans introspection — le schéma est déclaré une fois, dans la
commande. Et les lignes `~` sont des modifications de fichiers qui vous appartiennent :
le CLI a inséré dans des ancres en commentaires (`// <rbs:features>` dans `src/lib.rs`,
`<rbs:routes>`, `<rbs:openapi>`, `<rbs:seeds>`, `<rbs:migrations>`) plutôt que de réécrire
votre code. Supprimez une ancre et le CLI cesse d'y écrire : il affiche le bloc à coller.

Appliquez la nouvelle migration :

```bash
rbs migrate up
rbs migrate status
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.02s
     Running `target/debug/migration up`
✓ migrations appliquées
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running `target/debug/migration status`
  ✓ m20260829_100554_create_articles   appliquée
```

## Ce que le générateur a écrit

Sept fichiers par feature, plus ses tests et son seed, avec une seule direction de
dépendance : contrôleur → service → dépôt → modèle. Voici le gestionnaire de `POST /articles`, lu
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
10:06:25  INFO   demo                démarrage  adresse=127.0.0.1:8080
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
x-request-id: 01M16FRBHD0ZHWCN4EAEAW3TGK
content-length: 42
date: Sat, 29 Aug 2026 10:06:30 GMT

{"status":"ok","checks":{"database":"ok"}}
```

`/health` est venu avec le projet et vérifie la base, pas seulement le processus. Chaque
fragment qui apporte une dépendance y ajoute sa sonde — `rbs add redis` met une clé
`cache` dans `checks`, `rbs add storage` une clé `storage` — et il suffit qu'une seule se
taise pour que la réponse entière devienne un `503`, ce qui sort le pod de la rotation au
lieu d'y router un service dont le cache a disparu. L'en-tête `x-request-id` figure sur
chaque réponse, et la même valeur se retrouve dans la ligne de log de la requête.

```bash
curl -i -X POST http://127.0.0.1:8080/articles \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier article","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 201 Created
content-type: application/json
x-request-id: 01M16FRBHQDCDV859ADGK8ZMJG
content-length: 191
date: Sat, 29 Aug 2026 10:06:30 GMT

{"id":"01a04cfc-2e37-78a1-bcb1-6599b0c362e2","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-29T10:06:30.457086Z","updated_at":"2026-08-29T10:06:30.457086Z"}
```

L'identifiant et les horodatages viennent du serveur : `id`, `created_at` et `updated_at`
ne font pas partie du corps de la requête.

```bash
curl http://127.0.0.1:8080/articles
```

```text
{"data":[{"id":"01a04cfc-2e37-78a1-bcb1-6599b0c362e2","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-08-29T10:06:30.457086Z","updated_at":"2026-08-29T10:06:30.457086Z"}],"meta":{"page":1,"per_page":20,"total":1,"total_pages":1}}
```

Les collections sont paginées par défaut, sous `data` et `meta`. `?page=` et `?per_page=`
s'y déplacent. Les trois routes restantes — `GET`, `PATCH` et `DELETE` sur
`/articles/{id}` — ont été générées en même temps.

Pendant ce temps, le terminal du serveur affiche une ligne par requête :

```text
10:06:30  INFO   rbs_core::trace     request  status=200 latency_ms=0.80275 request_id=01M16FRBHD0ZHWCN4EAEAW3TGK method=GET path=/health
10:06:30  INFO   rbs_core::trace     request  status=201 latency_ms=4.517125 request_id=01M16FRBHQDCDV859ADGK8ZMJG method=POST path=/articles
10:06:30  INFO   rbs_core::trace     request  status=200 latency_ms=35.174416 request_id=01M16FRBJ3RVKC5T37ACD8SM6F method=GET path=/articles
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

{/* rbs:transcript cmd="rbs doctor" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" base="oui" extrait="oui" */}
```text
  ✓ ancres      les 12 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running `target/debug/migration version`
  ✓ base        postgres 18.6 répond sur localhost:5432
✓ le projet est sain
```

Six vérifications : les ancres sont toujours en place — douze ici, onze du squelette
plus celle du compose, qui sort du compte pour un projet sans `docker-compose.yml` — le
guide et l'inventaire d'[`AGENTS.md`](./guides/agents.md) s'accordent toujours avec ce que
porte le projet, aucun modèle ne porte de relation sans les deux ancres qu'il lui
faudrait pour en recevoir une, `.env` porte chaque clé que déclare `.env.example`, le
projet et `rbs-core` s'accordent avec la version du CLI, et la base répond.

## Pour aller plus loin

- [Logs](./guides/logs.md) — les deux formateurs, et quoi mettre dans `RUST_LOG`.
- Le code généré est le vôtre : ouvrez `src/articles/service.rs` et ajoutez-y une règle.
- `rbs generate crud --dry-run` affiche le plan sans rien écrire : c'est la façon la
  moins coûteuse de voir ce que produit un jeu de `--fields`.
- La [feuille de route](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) liste ce que
  couvre rbs et ce qui en est délibérément exclu.
