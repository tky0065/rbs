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
$ rbs new --help
Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health

Usage: rbs new [OPTIONS] <NOM>

Arguments:
  <NOM>  Nom du projet, qui est aussi celui du répertoire créé

Options:
      --database-url <URL>     URL de la base PostgreSQL, à défaut de quoi la question est posée
      --with <FEATURES>        Features à installer sans passer par les questions, séparées par des virgules
      --core-path <CHEMIN>     Crate `rbs-core` locale à utiliser au lieu de la version publiée
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

`<NOM>` est à la fois le nom du paquet Cargo et celui du répertoire. Il commence par une
lettre ASCII et ne porte que des lettres, des chiffres, `-` et `_`.

## Les flags

| Flag | Effet |
|---|---|
| `--database-url <URL>` | URL de connexion écrite dans le `.env` du projet sous `RBS_DATABASE__URL`. Absente, la question est posée — ou la valeur par défaut est prise sous `--yes`. |
| `--with <FEATURES>` | Features à installer à la création, séparées par des virgules. Ce que 0.1.0 en fait est décrit plus bas. |
| `--core-path <CHEMIN>` | Fait pointer le manifeste généré vers une crate `rbs-core` locale plutôt que vers la version publiée. Le chemin est canonisé : Cargo le résout depuis le projet créé, et non depuis le répertoire d'où la commande est lancée. |
| `--template-dir <CHEMIN>` | Rend le projet depuis un répertoire de templates, au lieu de celles embarquées dans le binaire. |
| `-y`, `--yes` | Ne demande rien : prend les valeurs par défaut et exécute. |

`--template-dir` et `--yes` sont globaux — toutes les commandes les acceptent — mais `--yes`
n'est lu que par `rbs new`, seule commande qui pose des questions, et `--template-dir` par
`rbs new` et [`rbs add`](./add.md).

## Créer un projet

```text
$ rbs new blog --database-url postgres://rbs:rbs@localhost:55432/blog --core-path /private/tmp/rbs-core --yes
✓ blog créé — 16 fichiers

  cd blog
  cargo run          # la base visée est dans .env
```

Les quinze fichiers :

```text
blog/.env
blog/.env.example
blog/.gitignore
blog/Cargo.toml
blog/config/default.toml
blog/config/development.toml
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

`git init` passe en dernier. S'il échoue, le projet reste complet : la commande le signale
sur stderr au lieu d'échouer.

Le parcours de ces pages utilise `--core-path` parce que `rbs-core` 0.1.0 n'est pas encore
sur crates.io. Sans ce flag, le manifeste reçoit `rbs-core = "0.1.0"` depuis le registre.

## Ce qui porte l'idempotence

Le `Cargo.toml` généré porte une section rbs, et cette section est le seul endroit où rbs
garde un état sur le projet :

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health"]
```

`version` est le rbs qui a généré le projet — [`rbs doctor`](./doctor.md) la compare à la
sienne. `features` s'allonge à mesure que [`rbs generate`](./generate.md) et
[`rbs add`](./add.md) installent, et c'est ce qui fait d'une seconde exécution de la même
commande une non-opération plutôt qu'un doublon. Un fichier d'état à part se serait
désynchronisé du dépôt la première fois qu'on aurait oublié de le committer ; le manifeste,
lui, est déjà versionné.

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

## Des templates prises du disque

`--template-dir` remplace le squelette embarqué par un répertoire de même forme : une
template `.jinja` par fichier à écrire, le suffixe retiré en sortie. Ci-dessous, une copie
du squelette dont le `.env.jinja` porte une ligne de plus :

```text
$ rbs new maison --template-dir /private/tmp/rbs-demo/mes-templates --core-path /private/tmp/rbs-core --yes
✓ maison créé — 16 fichiers

  cd maison
  cargo run          # la base visée est dans .env

$ tail -2 maison/.env
RUST_LOG=info,maison=debug
MAISON=1
```

## `--with` dans cette version

`--with` nomme les features à installer à la création. rbs en connaît trois — `auth`,
`ci` et `docker` — et les refuse toutes ici : elle les installe par
[`rbs add`](./add.md), et le dit plutôt que d'inscrire dans `[package.metadata.rbs]` une
feature qu'elle n'aurait pas posée.

```text
$ rbs new site --with auth --yes
erreur : `auth` ne s'installe pas à la création : créez le projet sans `--with`, puis `rbs add auth`
```

Un nom qui n'est pas une feature du tout est refusé avec la liste de celles qui en sont :

```text
$ rbs new site --with graphql --yes
erreur : `graphql` n'est pas une feature rbs — disponibles : docker, ci, auth
```

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
