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

Usage: rbs new [OPTIONS] <NAME>

Arguments:
  <NAME>  Nom du projet, qui est aussi celui du répertoire créé

Options:
      --database-url <URL>     URL de connexion, à défaut de quoi la question est posée
      --database <MOTEUR>      Moteur de base sur lequel le projet tournera [default: postgres] [possible values: postgres, mysql, sqlite]
      --with <FEATURES>        Features à installer sans passer par les questions, séparées par des virgules
      --core-path <CHEMIN>     Crate `rbs-core` locale à utiliser au lieu de la version publiée
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help (see more with '--help')
  -V, --version                Print version
```

`<NAME>` est à la fois le nom du paquet Cargo et celui du répertoire. Il commence par une
lettre ASCII et ne porte que des lettres, des chiffres, `-` et `_`.

## Les flags

| Flag | Effet |
|---|---|
| `--database-url <URL>` | URL de connexion écrite dans le `.env` du projet sous `RBS_DATABASE__URL`. Absente, la question est posée — ou la valeur par défaut est prise sous `--yes`. |
| `--database <MOTEUR>` | Moteur sur lequel le projet tournera : `postgres`, `mysql` ou `sqlite`. `postgres` par défaut. |
| `--with <FEATURES>` | Features à installer à la création, séparées par des virgules. Réellement installées — voir plus bas. |
| `--core-path <CHEMIN>` | Fait pointer le manifeste généré vers une crate `rbs-core` locale plutôt que vers la version publiée — le mode dans lequel rbs se développe, décrit [plus bas](#construire-contre-un-noyau-local). |
| `--template-dir <CHEMIN>` | Rend le projet depuis un répertoire de templates, au lieu de celles embarquées dans le binaire. |
| `-y`, `--yes` | Ne demande rien : prend les valeurs par défaut et exécute. |

`--template-dir` et `--yes` sont globaux — toutes les commandes les acceptent — mais `--yes`
n'est lu que par `rbs new`, seule commande qui pose des questions, et `--template-dir` par
`rbs new` et [`rbs add`](./add.md).

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

```text
$ rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --yes
✓ blog créé — 17 fichiers

  cd blog
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

Les dix-sept fichiers :

```text
blog/.env
blog/.env.example
blog/.gitignore
blog/Cargo.toml
blog/config/default.toml
blog/config/development.toml
blog/docker-compose.yml
blog/migration/Cargo.toml
blog/migration/src/lib.rs
blog/migration/src/main.rs
blog/src/health/controller.rs
blog/src/health/mod.rs
blog/src/main.rs
blog/src/openapi.rs
blog/src/router.rs
blog/src/seeds/main.rs
blog/src/state.rs
```

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
version = "1.0.0"
features = ["health"]
database = "postgres"
```

`version` est le rbs qui a généré le projet — [`rbs doctor`](./doctor.md) la compare à la
sienne, et [`rbs upgrade`](./upgrade.md) est ce qui la fait bouger. `database` est le
moteur pour lequel le projet a été créé. `features` s'allonge à mesure que
[`rbs generate`](./generate.md) et [`rbs add`](./add.md) installent, et c'est ce qui fait
d'une seconde exécution de la même commande une non-opération plutôt qu'un doublon. Un
fichier d'état à part se serait désynchronisé du dépôt la première fois qu'on aurait
oublié de le committer ; le manifeste, lui, est déjà versionné.

## Les trois questions

Sans `--yes`, et pour chaque réponse qu'aucun flag ne fournit, `rbs new` demande le nom du
projet, l'URL PostgreSQL — par défaut `postgres://postgres:postgres@localhost:5432/<nom>`,
les tirets devenant des soulignés — et les features à installer.

`--yes` court-circuite avant que la première question ne s'affiche, ce qui garde la commande
utilisable en CI. Sans terminal et sans `--yes`, elle nomme les flags qui auraient remplacé
les questions :

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
✓ blog créé — 17 fichiers

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
✓ maison créé — 17 fichiers

  cd maison
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux

$ tail -2 maison/.env
RUST_LOG=info,maison=debug
MAISON=1
```

## `--with` installe

`--with` nomme les features à installer à la création, séparées par des virgules. rbs en
connaît sept — `auth`, `ci`, `docker`, `jobs`, `mail`, `redis` et `storage` — et installe
chacune des nommées, dans la même passe qui écrit le projet :

```text
$ rbs new site --with auth --yes
✓ site créé — 17 fichiers
  + auth     9 fichiers, 1 migration

  cd site
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

L'ordre d'installation est dérivé des noms, non de l'ordre où ils ont été tapés —
alphabétique, le même ordre dans lequel [`rbs add`](./add.md) énumère les sept :

```text
$ rbs new with-demo --database-url postgres://rbs:secret@localhost:5432/with_demo --with storage,auth,docker --yes
✓ with-demo créé — 17 fichiers
  + auth     9 fichiers, 1 migration
  + docker   2 fichiers
  + storage  4 fichiers

  cd with-demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

`storage,auth,docker` a été tapé ; `auth`, puis `docker`, puis `storage` ont été
installées, et c'est l'ordre dans lequel `[package.metadata.rbs]` les consigne — le même
qu'un second `rbs add` de l'une d'elles laisserait intact.

Un nom qui n'est pas une feature du tout est refusé avant que le premier fichier ne soit
écrit :

```text
$ rbs new site --with graphql --yes
erreur : `graphql` n'est pas une feature rbs — disponibles : auth, ci, docker, jobs, mail, redis, storage
```

## Le compose engendré

Sauf dans les deux cas ci-dessous, `rbs new` écrit un `docker-compose.yml` à côté du
projet, portant la base que décrit son URL — identifiants, nom de base et port publié, le
tout lu depuis elle, rien de retapé :

```yaml
name: blog

services:
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: rbs
      POSTGRES_PASSWORD: rbs
      POSTGRES_DB: blog
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
      test: ["CMD-SHELL", "pg_isready -U rbs -d blog"]
      interval: 2s
      timeout: 3s
      retries: 30

  # <rbs:services>
  # </rbs:services>

volumes:
  pgdata:
```

`docker compose up -d` la démarre. L'ancre `# <rbs:services>` est là où [`rbs
add`](./add.md) insère les services qu'apporte `docker`, et c'est l'une des dix ancres que
vérifie [`rbs doctor`](./doctor.md) — neuf sur un projet sans compose pour en porter une
dixième.

Quatre cas n'écrivent rien :

- **un projet SQLite** — il n'y a pas de serveur à démarrer, et son URL n'a ni hôte ni
  port à porter dans un compose ;
- **une URL dont l'hôte n'est pas local** — le conteneur ne ferait que doubler une base
  déjà joignable ailleurs ;
- **une URL sans identifiants** — valide, acceptée par `--database-url`, mais l'image
  PostgreSQL officielle refuse de s'initialiser sans mot de passe : un compose qui ne peut
  pas démarrer est pire que pas de compose ;
- **une URL que l'analyseur refuse d'emblée** — un séparateur non encodé dans le mot de
  passe, par exemple, l'arrête plutôt que de deviner un hôte ou un nom de base : rien n'en
  est tiré, donc rien n'en écrit de compose.

```text
$ rbs new sqlite-demo --database sqlite --yes
✓ sqlite-demo créé — 16 fichiers

  cd sqlite-demo
  cargo run          # la base visée est dans .env
```

Seize fichiers, et non dix-sept : c'est au compte que ça se voit, rien dans la sortie ne
nommant le compose par son absence.

Un projet créé avant rbs 1.1.0 n'a pas non plus de compose, et lancer [`rbs
upgrade`](./upgrade.md) ne lui en ajoute pas — cette commande ne fait que réécrire
`[package.metadata.rbs]`. [`rbs add docker`](./add.md) en écrit un entier dans ce cas,
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
