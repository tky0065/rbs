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
| `ancres` | Les neuf points d'insertion : `// <rbs:features>` dans `src/main.rs`, `// <rbs:routes>` dans `src/router.rs`, `// <rbs:openapi>` dans `src/openapi.rs`, `// <rbs:migration_modules>` et `// <rbs:migrations>` dans `migration/src/lib.rs`, `// <rbs:state_champs>` et `// <rbs:state_init>` dans `src/state.rs`, `// <rbs:startup>` dans `src/main.rs`, `// <rbs:seeds>` dans `src/seeds/main.rs`. |
| `.env` | Toute variable déclarée par `.env.example` est renseignée dans `.env`. `.env.example` sert de référence parce qu'il est versionné et généré avec le squelette — une liste tenue dans le CLI aurait fait deux vérités à synchroniser. |
| `versions` | Le rbs inscrit dans `[package.metadata.rbs]`, la dépendance `rbs-core`, et le CLI qui diagnostique. |
| `base` | Le pilote compilé au manifeste face au schéma de l'URL, puis une connexion TCP en moins de trois secondes, puis la version du serveur — demandée au binaire de la crate `migration`, rbs n'embarquant aucun client SQL. Chaque moteur a son plancher, et chaque plancher sa raison : PostgreSQL 14, le plus ancien encore maintenu ; MySQL 8.0, pour `FOR UPDATE SKIP LOCKED` ; SQLite 3.35, pour `UPDATE … RETURNING`. |

Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est précisément
pourquoi `doctor` la cherche avant que [`rbs generate`](./generate.md) ne bute dessus.

Le pilote passe avant la connexion, et c'est délibéré. Un serveur qui répond ne prouve rien
quand le pilote compilé dans votre binaire ne sait pas parler son protocole, et sonder le
port d'abord ferait payer trois secondes à un diagnostic qui tient dans deux lectures de
fichier :

```text
  ✗ base       le manifeste compile `sqlx-postgres` et RBS_DATABASE__URL est une URL `mysql://`
      alignez les deux : la feature `sqlx-mysql` de sea-orm au manifeste, ou une URL `postgres://` dans le .env
```

C'est la contradiction que [`rbs new`](./new.md) refuse d'emblée, rencontrée ici après coup
— sur un projet dont le `.env` a été édité plus tard.

## Les features installées

Chaque feature qui porte de la configuration ajoute une ligne à elle, et cette ligne
n'existe que sur un projet qui a déclaré la feature. `jobs` est celle que ce jalon a
ajoutée :

```text
  ✗ jobs       config/default.toml ne porte pas de section `[jobs]`
      ajoutez à config/default.toml :
      [jobs]
      max_attempts = 5
      retry_delay_secs = 30
      poll_interval_secs = 1
```

Une feature déclarée dans `[package.metadata.rbs]` dont la section a disparu de la
configuration est un projet qui compile et échoue au démarrage — ce que `doctor` sait dire
à froid, avant que vous ne le lanciez. Une section mise en commentaire ne compte pas pour
une section.

## Un projet sain

```text
$ rbs doctor
  ✓ ancres     les 9 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✓ base       postgres 17.10 répond sur localhost:55446
  ✓ jobs       la configuration de la file est en place
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
  ✗ base       rien ne répond sur localhost:55446
      démarrez postgres, ou corrigez l'URL du .env
  ✓ jobs       la configuration de la file est en place
attention : le projet demande votre attention
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
  ✓ ancres     les 9 points d'insertion sont en place
  ✓ .env       les 4 variables de .env.example sont renseignées
  ✓ versions   projet et rbs-core pris d'un chemin local alignés sur le CLI 0.1.0
  ✗ base       localhost:55446 répond, mais sa version reste inconnue : la crate migration a échoué (code 1)
      vérifiez que `cargo run -p migration -- version` aboutit
  ✓ jobs       la configuration de la file est en place
attention : le projet demande votre attention
```

## Hors d'un projet

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Code de sortie 1.
