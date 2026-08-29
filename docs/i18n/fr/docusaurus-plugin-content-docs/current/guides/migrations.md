---
sidebar_position: 5
title: Migrations
---

# Migrations

Les changements de schéma vivent dans la crate `migration` de votre projet, un migrateur
SeaORM ordinaire. Ce que rbs ajoute, c'est le sens dans lequel les choses circulent : vous
décrivez les champs en ligne de commande, et l'entité comme sa migration en sortent.
Aucune base n'a besoin de tourner pour cela.

## La ligne de commande écrit le schéma, et non l'inverse

`sea-orm-cli generate entity` lit une base existante et en produit du Rust. Cela suppose
que le schéma existe déjà — écrit à la main, ou par une migration que vous avez aussi
écrite à la main.

`rbs generate crud` va dans l'autre sens :

```bash
rbs generate crud articles --fields 'title:string,body:text,published:bool'
```

Une commande, et les six fichiers de la feature, l'entité SeaORM **et** la migration qui
crée sa table sont tous écrits depuis la même description. Rien n'est contacté.

Un champ s'écrit `nom:type`, suivi le cas échéant de modificateurs :

| Type | Colonne | Rust |
|---|---|---|
| `string` | `string()` | `String` |
| `text` | `text()` | `String` |
| `int` | `integer()` | `i32` |
| `float` | `double()` | `f64` |
| `bool` | `boolean()` | `bool` |
| `uuid` | `uuid()` | `Uuid` |
| `datetime` | `timestamp_with_time_zone()` | `DateTimeWithTimeZone` |

| Modificateur | Effet |
|---|---|
| `unique` | index unique sur la colonne |
| `optional` | colonne nullable, `Option<T>` dans l'entité |
| `index` | index simple — refusé avec `unique`, qui indexe déjà |

## Ce qui en sort

La migration des `articles` ci-dessus, telle qu'elle est générée :

```rust file=examples/hello-crud/migration/src/m20260826_205243_create_articles.rs region=up
```

Trois colonnes s'ajoutent à celles que vous avez nommées. `id` est un UUID sans défaut de
colonne : **c'est le modèle engendré qui le pose**, avec `Uuid::now_v7()`, juste avant
l'insertion. Les identifiants se trient toujours par ordre de création, et aucun moteur ne
se voit réclamer un `uuidv7()` à lui — c'est ce qui permet à la même migration de tourner
sur PostgreSQL, MySQL et SQLite indifféremment. `created_at` et `updated_at` prennent tous
deux l'horodatage de la transaction pour défaut.

Les noms de colonnes sont déclarés dans l'énumération `DeriveIden` en bas du fichier, à
laquelle le constructeur de requêtes de SeaORM se réfère :

```rust file=examples/hello-crud/migration/src/m20260826_205243_create_articles.rs region=colonnes
```

Le nom du fichier porte la date et l'heure de sa création —
`m20260826_205243_create_articles.rs` — et c'est ce qui ordonne les migrations. C'est
aussi de là que `DeriveMigrationName` tire le nom inscrit en base : renommer le fichier
d'une migration appliquée fait croire au migrateur qu'elle n'a jamais tourné.

Le migrateur lui-même se remplit par deux ancres, une pour le module et une pour la
liste :

```rust file=examples/hello-crud/migration/src/lib.rs
```

Comme partout dans rbs, une ancre absente signifie que le CLI n'écrit rien et affiche le
bloc à coller.

## Les lancer

```bash
rbs migrate up       # applique tout ce qui est en attente
rbs migrate down     # annule la dernière migration appliquée
rbs migrate status   # ce qui est appliqué, ce qui attend
rbs migrate new add_slug_to_articles
```

`up`, `down` et `status` enveloppent `cargo run -p migration -- <commande>` dans votre
projet : le moteur de SeaORM n'est pas réimplémenté, seulement rendu lisible. Il leur faut
savoir quelle base viser, et elles le lisent dans le `.env` du projet, sous
`RBS_DATABASE__URL` — la variable même de la configuration du runtime, et non un
`DATABASE_URL` que rbs serait seul à connaître. L'environnement de l'appelant l'emporte,
si bien que

```bash
RBS_DATABASE__URL=postgres://… rbs migrate up
```

vise une autre base sans toucher au fichier.

`new` fait exception : elle crée une migration vide et la monte dans la crate, sans cargo
et sans base démarrée. C'est ce qu'on emploie pour tout ce que `generate crud` ne produit
pas — une colonne ajoutée, un index, une reprise de données.

## Jugez par vous-même

Depuis un projet généré, avec une base joignable :

```bash
rbs migrate status
```

Il inventorie toutes les migrations que la crate connaît, appliquées ou en attente, avant
que vous ne changiez quoi que ce soit.
