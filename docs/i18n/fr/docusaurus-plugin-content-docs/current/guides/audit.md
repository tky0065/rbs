---
sidebar_position: 11.5
title: Journal des écritures
---

# Journal des écritures

`rbs add audit` installe un journal des écritures dans un projet existant : quatre fichiers
sous `src/audit/`, et une migration pour la table `audit_log`. Comme les autres briques, il
ne monte aucune route — et, contrairement à elles, il ne se câble même pas sur celles que
vous avez déjà. C'est à votre service de l'appeler, et la raison est
[plus bas](#ce-que-le-fragment-ne-fait-pas).

## Ce qui est installé

```text
$ rbs add audit
audit : journal des écritures : qui a modifié quoi, quand, dans la transaction du changement

plan pour /private/tmp/rbs-demo/demo

  + src/audit/mod.rs                                     créé
  + src/audit/model.rs                                   créé
  + src/audit/repository.rs                              créé
  + src/audit/tests.rs                                   créé
  + migration/src/m20260903_173024_create_audit_log.rs   créé
  ~ migration/src/lib.rs                                 modifié
  ~ src/lib.rs                                           modifié
  ~ Cargo.toml                                           modifié
  ~ AGENTS.md                                            modifié

  9 fichiers à écrire
✓ audit installée — 5 fichiers

  rbs migrate up, puis appelez audit::record dans vos services — l'entrée s'écrit dans la transaction du changement
```

La migration vient avec, et [`rbs migrate up`](../cli/migrate.md) est donc la commande
suivante : tant que la table `audit_log` n'existe pas, le premier `record` échoue sur une
relation absente.

## Ce que c'est, et ce que ce n'est pas

Un projet engendré sait déjà *qu'*une requête a modifié une ligne : `trace.rs` journalise
la méthode, le chemin, le statut et la durée de chaque requête qui le traverse. Ce qu'il ne
sait pas, c'est *ce qui* a changé, ni qui l'a changé, au-delà de la rétention de vos logs.

Ce journal ne remplace pas cette trace, et ce n'est pas un middleware. Un layer qui
journaliserait chaque requête mutante ne vous coûterait aucun geste — et il ne pourrait
jamais dire *quoi* : il ne voit qu'un corps de requête, jamais l'avant et l'après d'une
ligne. Il doublerait par surcroît `trace.rs`, qui journalise déjà toute requête.

Le fragment vous donne donc une table, une entrée et une fonction, et c'est votre service
qui décide ce qui mérite une trace.

## L'écriture et sa trace sont indissociables

`record` prend un `&C: ConnectionTrait` et non une `DatabaseConnection`, et c'est toute la
raison de mettre le journal en base plutôt que dans un fichier. Une transaction *est* un
`ConnectionTrait` : passez-lui celle qui porte votre changement, et la trace naît si et
seulement si le changement est committé.

```rust
use sea_orm::TransactionTrait;
use serde_json::json;

use crate::audit::{self, Entry};

let transaction = state.core().db().begin().await?;

let ancien = post.title.clone();
let post = post.update(&transaction).await?;

audit::record(
    &transaction,
    Entry::new(audit::UPDATE, "posts", post.id.to_string())
        .actor(identity.user_id.clone())
        .changes(json!({ "title": { "from": ancien, "to": post.title } })),
)
.await?;

transaction.commit().await?;
```

Un journal qui garde la trace d'un `UPDATE` annulé ment. Un journal qui rate la trace d'un
`UPDATE` committé ment aussi. La transaction règle les deux d'un coup, et le test qui le
prouve est livré avec le fragment :
`an_entry_written_in_a_rolled_back_transaction_does_not_exist`.

C'est le même contrat que [`jobs::enqueue`](./jobs.md), et pour la même raison.

## L'acteur

`Entry::new` prend les trois champs sans lesquels une ligne de journal ne veut rien dire —
l'action, l'entité, l'identifiant de la ligne. `actor` et `changes` sont des ajouts en
chaîne : ce que l'appelant n'a pas à choisir, il n'a pas à l'écrire.

Sous [`auth`](./auth.md), l'acteur tient en une ligne dans votre handler :

```rust
Entry::new(audit::DELETE, "posts", id.to_string()).actor(identity.user_id.clone())
```

Sans `auth`, ne l'écrivez pas. `actor_id` est nullable, et `Entry::actor` prend une
`String` plutôt que le type `Identity`, qui n'existe que sous la feature `auth` de
`rbs-core`. Deux conséquences, toutes deux voulues : le fragment s'installe sur un service
interne sans le moindre JWT, et **les écritures hors requête restent traçables**. Un job de
nettoyage, un seed, une commande d'administration n'ont aucune identité HTTP, et un journal
qui exigerait un acteur les rendrait invisibles — précisément les écritures qu'on cherche à
expliquer après coup.

Un acteur absent est stocké à `NULL`, jamais à une chaîne vide. La distinction porte : une
chaîne vide dirait « un acteur anonyme », `NULL` dit « aucune identité HTTP ».

## `action` et `changes` sont ouverts

`action` est une `String`, non un enum. Trois constantes couvrent le cas courant :

```rust
pub const CREATE: &str = "create";
pub const UPDATE: &str = "update";
pub const DELETE: &str = "delete";
```

Tout le reste est une action légitime — `login`, `export`, `impersonate` — et un enum fermé
ne ferait que vous forcer à le contourner. `jobs::Status`, lui, *est* un enum, parce que son
ensemble est fermé ; celui-ci ne l'est pas.

`changes` est une `serde_json::Value`, et le fragment ne lui impose aucun schéma. Un
avant/après par champ se relit bien et c'est ce qu'écrit l'exemple ci-dessus, mais une liste
de colonnes touchées, un diff, ou `Value::Null` sont tout aussi valides. `entity_id` est du
`TEXT` et non de l'`UUID` pour la même raison : le générateur pose des clés UUIDv7, mais une
entité écrite à la main peut porter une clé entière ou composite, et le journal doit pouvoir
la citer.

## La table

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | UUIDv7 posé par `ActiveModelBehavior::new`, comme partout ailleurs |
| `actor_id` | `text` null | L'auteur, ou rien pour une écriture système |
| `action` | `text` | `create`, `update`, `delete`, ou ce que votre projet décide |
| `entity` | `text` | Le nom de la table visée |
| `entity_id` | `text` | La clé de la ligne visée, telle qu'elle s'écrit |
| `changes` | `json` | Ce qui a changé. Forme libre |
| `created_at` | `timestamptz` | Défaut `current_timestamp` |

Deux index : `(entity, entity_id)`, par lequel se lit l'histoire d'une ligne, et
`created_at`, par lequel se lit celle d'une journée. Sans le premier, le coût d'une lecture
croît avec le journal entier — et un journal est fait pour grossir.

Pas de colonne `updated_at` : une ligne de journal ne se modifie pas.

Relire l'histoire d'une ligne est une requête ordinaire :

```rust
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::audit::model::{Column, Entity};

let histoire = Entity::find()
    .filter(Column::Entity.eq("posts"))
    .filter(Column::EntityId.eq(id.to_string()))
    .order_by_asc(Column::CreatedAt)
    .order_by_asc(Column::Id)
    .all(state.core().db())
    .await?;
```

Le second tri n'est pas décoratif : MySQL tronque `created_at` à la seconde, et trois
entrées écrites dans la même n'auraient sinon aucun ordre défini. L'UUIDv7 est monotone, il
tranche.

## Ce que le fragment ne fait pas

Il ne câble rien dans le CRUD qu'engendre `rbs generate`. Aucun handler n'appelle `record`
à votre place, et installer la feature ne change le comportement d'aucune route existante.

C'est un choix, pas un manque. Quelles écritures méritent une trace est une question à
laquelle seul votre domaine répond : un `PATCH` sur un brouillon et un `DELETE` sur une
facture n'ont pas le même poids, et un fragment qui journaliserait les deux noierait la
table ou vous forcerait à défaire son câblage. Le service qui porte le changement est le
seul à le savoir, et c'est aussi le seul à tenir la transaction que la trace doit rejoindre.

Il ne monte pas davantage de route pour *lire* le journal. Ce que vous en exposez, et à qui,
est une décision qui a ses propres règles d'accès — la requête ci-dessus suffit à la bâtir.
