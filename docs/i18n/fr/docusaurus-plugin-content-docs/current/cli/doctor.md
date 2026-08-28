---
sidebar_position: 5
title: rbs doctor
---

# `rbs doctor`

Diagnostique un projet généré par quatre contrôles : les ancres, le `.env`, les versions et
la base. Chacun est indépendant et rend son verdict sans interrompre les autres — un
diagnostic qui s'arrête au premier problème oblige à le relancer autant de fois qu'il y a de
problèmes.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs doctor --help
Diagnostique le projet : ancres, .env, base joignable, versions

Usage: rbs doctor [OPTIONS]

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

Aucun flag propre. Les deux options globales sont acceptées parce que clap les propage, et
aucune n'a d'effet ici.

## Les quatre contrôles

| Contrôle | Ce qu'il regarde |
|---|---|
| `ancres` | Les huit points d'insertion : `// <rbs:features>` dans `src/main.rs`, `// <rbs:routes>` dans `src/router.rs`, `// <rbs:openapi>` dans `src/openapi.rs`, `// <rbs:migration_modules>` et `// <rbs:migrations>` dans `migration/src/lib.rs`, `// <rbs:state_champs>` et `// <rbs:state_init>` dans `src/state.rs`, `// <rbs:seeds>` dans `src/seeds/main.rs`. |
| `.env` | Toute variable déclarée par `.env.example` est renseignée dans `.env`. `.env.example` sert de référence parce qu'il est versionné et généré avec le squelette — une liste tenue dans le CLI aurait fait deux vérités à synchroniser. |
| `versions` | Le rbs inscrit dans `[package.metadata.rbs]`, la dépendance `rbs-core`, et le CLI qui diagnostique. |
| `base` | Une connexion TCP en moins de trois secondes, puis la version du serveur — demandée au binaire de la crate `migration`, rbs n'embarquant aucun client SQL. PostgreSQL 18 est le minimum : `uuidv7()`, que les migrations générées posent en défaut de clé primaire, n'existe pas avant. |

Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est précisément
pourquoi `doctor` la cherche avant que [`rbs generate`](./generate.md) ne bute dessus.

## Un projet sain

```text
$ rbs doctor
  ✓ ancres     les 7 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✓ base       PostgreSQL 18.6 répond sur localhost:55432
✓ le projet est sain
```

Code de sortie 0.

## Un projet à problèmes

Ci-dessous, le même projet privé de `// <rbs:openapi>` dans `src/openapi.rs`, de
`RBS_LOG_FORMAT` dans `.env`, et avec PostgreSQL arrêté :

```text
$ rbs doctor
  ✗ ancres     openapi manque dans src/openapi.rs
      dans src/openapi.rs :
      // <rbs:openapi>
      // </rbs:openapi>
  ✗ .env       RBS_LOG_FORMAT absente du .env
      ajoutez au .env :
      RBS_LOG_FORMAT=pretty
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✗ base       rien ne répond sur localhost:55432
      démarrez PostgreSQL, ou corrigez l'URL du .env
```

Trois échecs, un contrôle encore au vert, et chaque ligne en défaut porte le geste qui la
corrige : le bloc d'ancre à recoller, la ligne de `.env` à ajouter, le serveur à démarrer.

Code de sortie 1. Un diagnostic qui trouve quelque chose n'est pas un échec de la commande,
mais un script doit pouvoir le distinguer d'un projet sain : le code diffère.

## Joignable mais illisible

Les deux moitiés du contrôle `base` échouent séparément. Ici l'hôte répond sur le port, mais
la version n'a pas pu être lue, la crate `migration` n'ayant pas abouti — le remède nomme la
commande à lancer à la main :

```text
$ rbs doctor
  ✓ ancres     les 7 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✗ base       localhost:5432 répond, mais sa version reste inconnue : la crate migration a échoué (code 1)
      vérifiez que `cargo run -p migration -- version` aboutit
```

## Hors d'un projet

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Code de sortie 1.
