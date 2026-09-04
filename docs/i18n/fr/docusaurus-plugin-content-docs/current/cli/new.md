---
sidebar_position: 1
title: rbs new
---

# `rbs new`

Crée un projet qui tourne tel quel : un workspace Cargo, une crate `migration`, une route
`/health`, un `.env` et un dépôt Git. Rien n'est compilé, aucune base n'est contactée — la
commande écrit des fichiers, et s'arrête là.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs new -h
Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health

Usage: rbs new [OPTIONS] [NAME]

Arguments:
  [NAME]  Nom du projet, qui est aussi celui du répertoire créé, à défaut de quoi la question est posée

Options:
      --database-url <URL>     URL de connexion, à défaut de quoi la question est posée
      --database <MOTEUR>      Moteur de base sur lequel le projet tournera [default: postgres] [possible values: postgres, mysql, sqlite]
      --with <FEATURES>        Features à installer sans passer par les questions, séparées par des virgules
      --core-path <CHEMIN>     Crate `rbs-core` locale à utiliser au lieu de la version publiée
      --lang <LANGUE>          Langue de l'`AGENTS.md` engendré. À défaut, celle de l'environnement [possible values: fr, en]
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help (see more with '--help')
  -V, --version                Print version
```

`[NAME]` est à la fois le nom du paquet Cargo et celui du répertoire. Il commence par une
lettre ASCII et ne porte que des lettres, des chiffres, `-` et `_`. Omis, il devient la
première des [trois questions](#les-trois-questions).

## Les flags

| Flag | Effet |
|---|---|
| `--database-url <URL>` | URL de connexion écrite dans le `.env` du projet sous `RBS_DATABASE__URL`. Absente, la question est posée — ou la valeur par défaut est prise sous `--yes`. |
| `--database <MOTEUR>` | Moteur sur lequel le projet tournera : `postgres`, `mysql` ou `sqlite`. `postgres` par défaut. |
| `--with <FEATURES>` | Features à installer à la création, séparées par des virgules. Réellement installées — voir plus bas. |
| `--core-path <CHEMIN>` | Fait pointer le manifeste généré vers une crate `rbs-core` locale plutôt que vers la version publiée — le mode dans lequel rbs se développe, décrit [plus bas](#construire-contre-un-noyau-local). |
| `--lang <LANGUE>` | Langue de l'[`AGENTS.md`](../guides/agents.md) engendré : `fr` ou `en`. Absent, déduite de `LC_ALL`, puis de `LANG`. |
| `--template-dir <CHEMIN>` | Rend le projet depuis un répertoire de templates, au lieu de celles embarquées dans le binaire. |
| `-y`, `--yes` | Ne demande rien : prend les valeurs par défaut et exécute. Le nom du projet doit alors être donné en argument — c'est la seule réponse sans valeur par défaut. |

`--yes` n'est déclaré que sur `rbs new`, seule commande qui pose des questions, et
`--template-dir` sur `rbs new` et [`rbs add`](./add.md). Aucune autre ne les accepte : un
`--template-dir` égaré est une erreur de clap plutôt qu'un flag sans effet.

## Choisir le moteur

```text
$ rbs new blog --database sqlite --yes
```

Manifestes, `.env.example`, compose et configuration suivent tous la valeur choisie.
`sea-orm` reçoit la feature `sqlx-*` correspondante, et la migration engendrée évite ce qui
n'a pas d'équivalent sur les deux autres.

Une valeur inconnue est refusée avant que rien ne soit écrit :

```text
$ rbs new blog --database oracle
error: invalid value 'oracle' for '--database <MOTEUR>'
  [possible values: postgres, mysql, sqlite]

For more information, try '--help'.
```

Sans le drapeau, `postgres` reste le défaut, et un manifeste sans clé `database` se relit
comme un projet PostgreSQL — aucun projet créé avant l'existence de ce drapeau ne change de
comportement.

:::warning
`--database` et `--database-url` doivent s'accorder. Demander `--database mysql` avec une
URL `postgres://` est un refus, levé dans la phase de vérification et donc avant le premier
fichier écrit.

La même contradiction atteinte après coup — en éditant le `.env` d'un projet existant — est
ce que [`rbs doctor`](./doctor.md) constate, en nommant les deux valeurs.
:::

SQLite est celui qui change la forme du projet plutôt qu'une de ses lignes : ni compose, ni
attente de la base dans [`rbs dev`](./dev.md), et une URL sans hôte ni port.

## Créer un projet

{/* rbs:transcript cmd="rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --yes" */}
```text
$ rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --yes
✓ blog créé — 21 fichiers

  cd blog
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

Les vingt fichiers :

```text
blog/.env
blog/.env.example
blog/.gitignore
blog/AGENTS.md
blog/Cargo.toml
blog/config/default.toml
blog/config/development.toml
blog/config/production.toml
blog/docker-compose.yml
blog/migration/Cargo.toml
blog/migration/src/lib.rs
blog/migration/src/main.rs
blog/src/bin/openapi.rs
blog/src/health/controller.rs
blog/src/health/mod.rs
blog/src/lib.rs
blog/src/main.rs
blog/src/openapi.rs
blog/src/router.rs
blog/src/seeds/main.rs
blog/src/state.rs
```

`src/lib.rs` et `src/main.rs` séparent ce qui n'était qu'un seul fichier, et la séparation
a une raison d'être : `src/main.rs` et `src/seeds/main.rs` sont deux racines de crate
distinctes, si bien qu'aucune ne peut atteindre directement les modules de l'autre.
`src/lib.rs` est ce que les deux partagent — `AppState`, le routeur, et, une fois
[`rbs generate`](./generate.md) passé, le modèle de chaque feature — si bien que le binaire
des seeds atteint une entité par la bibliothèque plutôt que par un attribut `#[path]` qui
lui serait propre. `src/main.rs` ne garde que le démarrage : construire l'état, monter le
routeur, lier l'écouteur.

`docker-compose.yml` est le compose engendré, couvert plus bas — son port ici est
`55432`, pris dans l'URL plutôt que le `5432` propre au moteur.

`git init` passe en dernier. S'il échoue, le projet reste complet : la commande le signale
sur stderr au lieu d'échouer.

Le manifeste dépend de `rbs-core` publié sur crates.io, dans la version du CLI qui l'a
écrit. Il n'y a rien à construire ni à cloner d'abord.

## Ce qui porte l'idempotence

Le `Cargo.toml` généré porte une section rbs, et cette section est le seul endroit où rbs
garde un état sur le projet :

```text
[package.metadata.rbs]
version = "1.1.0"
features = ["health"]
database = "postgres"
lang = "fr"
```

`version` est le rbs qui a généré le projet — [`rbs doctor`](./doctor.md) la compare à la
sienne, et [`rbs upgrade`](./upgrade.md) est ce qui la fait bouger. `database` est le
moteur pour lequel le projet a été créé. `features` s'allonge à mesure que
[`rbs generate`](./generate.md) et [`rbs add`](./add.md) installent, et c'est ce qui fait
d'une seconde exécution de la même commande une non-opération plutôt qu'un doublon. `lang`
est la langue dans laquelle [`AGENTS.md`](../guides/agents.md) est écrit, couverte plus
bas. Un fichier d'état à part se serait désynchronisé du dépôt la première fois qu'on
aurait oublié de le committer ; le manifeste, lui, est déjà versionné.

## La langue d'AGENTS.md

`rbs new` écrit aussi [`AGENTS.md`](../guides/agents.md) à la racine du projet — le mode
d'emploi de rbs, écrit pour un agent plutôt que pour un humain. `--lang` choisit la langue
dans laquelle il est écrit :

```text
$ rbs new demo-api --database-url postgres://rbs:rbs@localhost:5432/demo_api --lang en --yes
✓ demo-api créé — 21 fichiers

$ grep lang demo-api/Cargo.toml
lang = "en"
```

Sans le drapeau, la langue est déduite de l'environnement — `LC_ALL` d'abord, puis `LANG`
— une valeur qui commence par `fr` donnant le français, toute autre valeur non vide
donnant l'anglais, et l'absence de valeur donnant le français. Le choix s'inscrit dans
tous les cas dans `lang`, dans `[package.metadata.rbs]` ci-dessus, ce qui permet à
[`rbs add`](./add.md) et [`rbs upgrade`](./upgrade.md) de continuer à écrire le fichier
dans la langue du projet plutôt que dans celle de qui lance la commande ensuite.

## Les trois questions

Sans `--yes`, et pour chaque réponse qu'aucun flag ne fournit, `rbs new` demande le nom du
projet, l'URL PostgreSQL — par défaut `postgres://postgres:postgres@localhost:5432/<nom>`,
les tirets devenant des soulignés — et les features à installer.

`--yes` court-circuite avant que la première question ne s'affiche, ce qui garde la commande
utilisable en CI. Le nom est la seule réponse qu'il ne peut pas fournir : aucun défaut ne
vaudrait un répertoire qu'il faudrait ensuite renommer.

```text
$ rbs new --yes
erreur : le nom du projet manque : `--yes` ne pose aucune question, et aucun nom par défaut ne vaudrait celui que vous n'avez pas donné — nommez le projet en argument, `rbs new mon-api --yes`
```

Sans terminal et sans `--yes`, elle nomme les flags qui auraient remplacé les questions :

```text
$ rbs new sans-tty < /dev/null
erreur : aucun terminal interactif pour poser les questions : relancez avec `--yes` pour prendre les défauts, ou donnez les réponses en flags — le nom en argument, `--database-url` et `--with`
```

## Construire contre un noyau local

Par défaut, le manifeste généré dépend de `rbs-core` pris au registre, ce que veut un
projet. `--core-path` remplace cette dépendance par un chemin vers une copie locale de la
crate :

```text
$ rbs new blog --core-path /private/tmp/rbs-core --yes
✓ blog créé — 21 fichiers

  cd blog
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux

$ grep rbs-core blog/Cargo.toml
rbs-core = { path = "/private/tmp/rbs-core", default-features = false, features = ["postgres"] }
```

C'est le mode dans lequel rbs se développe, et la seule raison de saisir ce flag : une
modification du noyau s'éprouve en engendrant un projet contre lui, avant que la version
qui la porte soit publiée. Le chemin est canonisé dans le manifeste : Cargo le résout
depuis le projet, et non depuis le répertoire d'où la commande a été lancée.

Deux commandes lisent alors le manifeste autrement. [`rbs doctor`](./doctor.md) annonce un
noyau pris d'un chemin local au lieu d'en nommer la version, et
[`rbs upgrade`](./upgrade.md) laisse la dépendance en place — un chemin n'a pas de version
à monter.

## Des templates prises du disque

`--template-dir` remplace le squelette embarqué par un répertoire de même forme : une
template `.jinja` par fichier à écrire, le suffixe retiré en sortie. Ci-dessous, une copie
du squelette dont le `.env.jinja` porte une ligne de plus :

```text
$ rbs new maison --template-dir /private/tmp/rbs-demo/mes-templates --yes
✓ maison créé — 21 fichiers

  cd maison
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux

$ tail -2 maison/.env
RUST_LOG=info,maison=debug
MAISON=1
```

## `--with` installe

`--with` nomme les features à installer à la création, séparées par des virgules. rbs en
connaît treize — `audit`, `auth`, `ci`, `cors`, `docker`, `jobs`, `mail`,
`observability`, `rate-limit`, `redis`, `scheduler`, `storage` et `webhooks` — et
installe chacune des nommées, dans la même passe qui écrit le projet :

{/* rbs:transcript cmd="rbs new site --with auth --yes" */}
```text
$ rbs new site --with auth --yes
✓ site créé — 21 fichiers
  + auth       13 fichiers, 1 migration

  rbs migrate up

  cd site
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

L'ordre d'installation est dérivé des noms, non de l'ordre où ils ont été tapés —
alphabétique, le même ordre dans lequel [`rbs add`](./add.md) énumère les douze :

```text
$ rbs new with-demo --database-url postgres://rbs:secret@localhost:5432/with_demo --with storage,auth,docker --yes
✓ with-demo créé — 21 fichiers
  + auth       13 fichiers, 1 migration
  + docker     2 fichiers
  + storage    4 fichiers

  rbs migrate up

  docker compose --profile app up --build

  les objets vont sous ./storage : ajoutez-le à .gitignore, ou passez storage.backend à "s3" et recopiez les RBS_STORAGE__* de .env.example

  cd with-demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

`storage,auth,docker` a été tapé ; `auth`, puis `docker`, puis `storage` ont été
installées, et c'est l'ordre dans lequel `[package.metadata.rbs]` les consigne — le même
qu'un second `rbs add` de l'une d'elles laisserait intact.

Un nom qui n'est pas une feature du tout est refusé avant que le premier fichier ne soit
écrit :

{/* rbs:transcript cmd="rbs new site --with graphql --yes" */}
```text
$ rbs new site --with graphql --yes
erreur : `graphql` n'est pas une feature rbs — disponibles : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage, webhooks
```

## Le compose engendré

Sauf dans les quatre cas ci-dessous, `rbs new` écrit un `docker-compose.yml` à côté du
projet, portant la base que décrit son URL — identifiants, nom de base et port publié, le
tout lu depuis elle, rien de retapé. Le compose est versionné et `.env` ne l'est pas : il
nomme donc les trois valeurs plutôt qu'il ne les écrit, et Compose les interpole depuis le
`.env` posé à côté :

```yaml
name: blog

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
      - "55432:5432"
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

`docker compose up -d` la démarre. L'ancre `# <rbs:services>` est là où [`rbs
add`](./add.md) insère les services qu'apporte `docker`, et c'est l'une des douze ancres
que vérifie [`rbs doctor`](./doctor.md) — onze sur un projet sans compose pour en porter
une douzième.

Quatre cas n'écrivent rien :

- **un projet SQLite** — il n'y a pas de serveur à démarrer, et son URL n'a ni hôte ni
  port à porter dans un compose ;
- **une URL dont l'hôte n'est pas local** — le conteneur ne ferait que doubler une base
  déjà joignable ailleurs ;
- **une URL sans identifiants** — valide, acceptée par `--database-url`, mais l'image
  PostgreSQL officielle refuse de s'initialiser sans mot de passe : un compose qui ne peut
  pas démarrer est pire que pas de compose ;
- **une URL que l'analyseur refuse d'emblée** — un séparateur non encodé dans le mot de
  passe, ou une socket Unix comme `postgres:///demo`, l'arrête plutôt que de deviner un
  hôte ou un nom de base : rien n'en est tiré, donc rien n'en écrit de compose. C'est le
  seul cas dont `rbs new` avertit, et le seul qui en ait besoin : les trois précédents sont
  des choix lisibles dans l'URL, quand celui-ci ne se voit qu'à un fichier qui n'a jamais
  été écrit.

```text
$ rbs new sqlite-demo --database sqlite --yes
✓ sqlite-demo créé — 20 fichiers

  cd sqlite-demo
  cargo run          # la base visée est dans .env
```

Dix-neuf fichiers, et non vingt : c'est au compte que ça se voit, rien dans la sortie ne
nommant le compose par son absence.

Un projet créé avant rbs 1.1.0 n'a pas non plus de compose, et lancer [`rbs
upgrade`](./upgrade.md) ne lui en ajoute pas — le compose ne fait pas plus partie de ce
qu'elle touche que le `.env`. [`rbs add docker`](./add.md) en écrit un entier dans ce cas,
services de déploiement compris.

## Les échecs

Tout ce qui peut être vérifié l'est avant que le rendu commence, et le rendu aboutit avant
que le premier fichier soit écrit. Un nom refusé, une feature indisponible ou une template
qui ne se rend pas laissent le disque exactement dans l'état où ils l'ont trouvé.

Un répertoire déjà pris :

```text
$ rbs new blog --yes
erreur : /private/tmp/rbs-demo/blog existe déjà : choisissez un autre nom, ou retirez ce répertoire
```

Un nom qui ne peut pas être un paquet Cargo :

```text
$ rbs new 4chan --yes
erreur : `4chan` n'est pas un nom de projet utilisable : lettres, chiffres, `-` et `_`, en commençant par une lettre
```

Chacun de ces cas sort en code 1 sans rien écrire.
