---
sidebar_position: 4
title: rbs migrate
---

# `rbs migrate`

Pilote les migrations du projet. `up`, `down` et `status` enveloppent le binaire de la crate
`migration` du projet : le moteur de SeaORM n'est pas réimplémenté, seulement rendu lisible.
`new` n'a besoin de personne — ni de cargo, ni d'une base démarrée.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs migrate --help
Pilote les migrations du projet

Usage: rbs migrate [OPTIONS] <COMMAND>

Commands:
  up      Applique les migrations en attente
  down    Annule la dernière migration appliquée
  status  Affiche les migrations appliquées et celles en attente
  new     Crée un fichier de migration vide
  help    Print this message or the help of the given subcommand(s)

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

Aucune sous-commande n'a de flag propre. Les deux options globales sont acceptées parce que
clap les propage, et aucune n'a d'effet ici.

```text
$ rbs migrate up --help
Applique les migrations en attente

Usage: rbs migrate up [OPTIONS]

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

`down` et `status` sont déclarées de la même manière. Seule `new` prend un argument :

```text
$ rbs migrate new --help
Crée un fichier de migration vide

Usage: rbs migrate new [OPTIONS] <NOM>

Arguments:
  <NOM>  Nom de la migration

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

## Quelle base

`up`, `down` et `status` lisent le `.env` du projet et y prennent la cible dans
`RBS_DATABASE__URL` — la variable que la configuration du noyau utilise déjà pour alimenter
`database.url`, et non un `DATABASE_URL` que rbs serait seul à connaître. Elles délèguent
ensuite à `cargo`, ce qui compile la crate `migration` à la première exécution.

## `status`

Les migrations appliquées portent `✓`, celles en attente `·`. Sur un projet dont la
migration n'a jamais tourné :

```text
$ rbs migrate status
  · m20260826_213608_create_articles   en attente
```

## `up`

```text
$ rbs migrate up
✓ migrations appliquées
```

Et le même projet, une fois à jour :

```text
$ rbs migrate status
  ✓ m20260826_213608_create_articles   appliquée
```

## `new`

Crée un fichier de migration vide, horodaté, et l'inscrit dans le `Migrator`. Elle ne touche
ni à cargo ni à la base : elle fonctionne donc sans que rien ne tourne.

```text
$ rbs migrate new add_tags_index
✓ migration/src/m20260826_213622_add_tags_index.rs créée

  décrivez le changement de schéma, puis `rbs migrate up`
```

L'inscription passe par deux ancres de `migration/src/lib.rs`, tenues distinctes parce que
Rust interdit un `mod` non-inline dans un bloc : la déclaration ne peut donc pas tenir dans
le `vec!` du `Migrator`.

```text
$ cat migration/src/lib.rs
pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260826_213608_create_articles;
mod m20260826_213622_add_tags_index;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260826_213608_create_articles::Migration),
            Box::new(m20260826_213622_add_tags_index::Migration),
            // </rbs:migrations>
        ]
    }
}
```

`status` en a maintenant une de chaque :

```text
$ rbs migrate status
  ✓ m20260826_213608_create_articles   appliquée
  · m20260826_213622_add_tags_index    en attente
```

Le corps du nouveau fichier est un `todo!()` qui porte la consigne : lancer `up` avant
d'avoir décrit le changement de schéma le dit exactement, au lieu d'appliquer une migration
vide.

```text
$ rbs migrate up

thread 'main' (7889417) panicked at migration/src/m20260826_213622_add_tags_index.rs:11:9:
not yet implemented: décrivez le changement de schéma, puis relancez `rbs migrate up`
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
erreur : la crate migration a échoué (code 101)
```

## `down`

Annule la dernière migration appliquée — une, pas toutes :

```text
$ rbs migrate down
✓ dernière migration annulée

$ rbs migrate status
  · m20260826_213608_create_articles   en attente
  · m20260826_213622_add_tags_index    en attente
```

## Les échecs

Hors d'un projet — la recherche remonte depuis le répertoire courant jusqu'à un `Cargo.toml`
portant `[package.metadata.rbs]`, ce qui empêche aussi une commande lancée depuis
`migration/src` de viser la mauvaise racine :

```text
$ rbs migrate status
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Avec un `.env` qui ne dit pas quelle base viser :

```text
$ rbs migrate status
erreur : RBS_DATABASE__URL est absente du .env : rbs ne sait pas quelle base migrer
```

Avec rien qui réponde à l'autre bout, le message vient du binaire de migration, dont rbs
rapporte le code de sortie :

```text
$ rbs migrate status
Connection Error: pool timed out while waiting for an open connection
erreur : la crate migration a échoué (code 1)
```

`rbs migrate new` est insensible aux deux derniers cas : elle ne lit jamais le `.env` et
n'ouvre jamais de connexion. Elle a toujours besoin d'un projet.

Chacun de ces cas sort en code 1.
