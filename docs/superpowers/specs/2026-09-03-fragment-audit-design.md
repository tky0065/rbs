# `rbs add audit`

**Tâche 76 d'`IMPROVE.md`.** Un projet engendré sait aujourd'hui *qu'*une requête a
modifié une ligne — `rbs-core/src/trace.rs` journalise méthode, chemin, statut et durée —
mais rien ne garde *ce qui* a changé, ni qui l'a changé, au-delà de la rétention des logs.
Aucun des dix fragments ne couvre la question, et un service qui en a besoin réécrit à
chaque fois la même table.

## Ce qui est décidé

**Le fragment fournit un journal métier explicite, pas une trace HTTP.** Le layer qui
journaliserait chaque requête mutante serait plus commode — zéro geste — mais il ne peut
pas dire *quoi* : il ne voit qu'un corps de requête, jamais l'avant et l'après d'une
ligne. Il doublerait par surcroît `trace.rs`, qui journalise déjà toute requête.

Le fragment dépose donc une table, une entrée et une fonction, sur le modèle exact de
`jobs::enqueue` : c'est le service qui décide ce qui mérite une trace, et l'écrit dans la
transaction qui porte le changement.

### L'écriture et sa trace sont indissociables

`record` prend un `&C: ConnectionTrait` et non une `DatabaseConnection`. C'est toute la
raison de mettre le journal en base plutôt que dans un fichier : passez-lui la transaction
du métier, et **la trace naît si et seulement si le changement est committé**. Un journal
qui garde la trace d'un `UPDATE` annulé ment ; un journal qui rate la trace d'un `UPDATE`
committé ment aussi. La transaction règle les deux d'un coup.

C'est le même contrat qu'`enqueue` (`features/jobs/queue.rs.jinja:80-88`), et pour la même
raison.

### L'acteur est optionnel, et le fragment ne dépend pas de `auth`

`actor_id` est `TEXT NULL`, et `Entry::actor` prend une `String` — jamais le type
`Identity`, qui n'existe que sous la feature `auth` de `rbs-core`
(`crates/rbs-core/src/lib.rs:50`).

Deux conséquences, toutes deux voulues. Le fragment s'installe sur un service interne sans
JWT. Et surtout **les écritures hors requête restent traçables** : un job de nettoyage,
un seed, une commande d'administration n'ont aucune identité HTTP, et un journal qui
exigerait un acteur les rendrait invisibles — précisément les écritures qu'on cherche à
expliquer après coup.

Sous `auth`, le controller écrit `identity.user_id`. C'est une ligne, et elle est dans la
documentation.

### `action` est une chaîne, `entity_id` aussi

`action` n'est pas un enum. L'ensemble est ouvert : `login`, `export`, `impersonate` sont
des actions légitimes qu'un enum fermé forcerait à contourner. Trois constantes
(`audit::CREATE`, `UPDATE`, `DELETE`) couvrent le cas courant sans fermer la porte — là
où `jobs::Status` est un enum parce que son ensemble, lui, est fermé.

`entity_id` est `TEXT` et non `UUID` : le générateur pose des clés UUIDv7, mais une entité
écrite à la main peut porter une clé entière ou composite, et le journal doit pouvoir la
citer.

## La table

Migration `create_audit_log`, table `audit_log` :

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | UUIDv7 posé par `ActiveModelBehavior::new`, comme partout ailleurs |
| `actor_id` | `text` null | L'auteur, ou rien pour une écriture système |
| `action` | `text` | `create`, `update`, `delete`, ou ce que le projet décide |
| `entity` | `text` | Le nom de la table visée |
| `entity_id` | `text` | La clé de la ligne visée, telle qu'elle s'écrit |
| `changes` | `json` | Ce qui a changé. Forme libre — le fragment n'impose pas de schéma |
| `created_at` | `timestamptz` | Défaut `current_timestamp` |

Deux index : `(entity, entity_id)`, par lequel se lit l'histoire d'une ligne, et
`created_at`, par lequel se lit celle d'une journée. Sans le premier, le coût d'une
lecture croît avec le journal entier — et un journal est fait pour grossir.

Pas de colonne `updated_at` : une ligne de journal ne se modifie pas.

## L'API

```rust
use crate::audit::{self, Entry};

let post = post.update(&txn).await?;

audit::record(
    &txn,
    Entry::new(audit::UPDATE, "posts", post.id.to_string())
        .actor(identity.user_id.clone())
        .changes(json!({ "title": { "from": ancien, "to": post.title } })),
)
.await?;

txn.commit().await?;
```

`Entry::new` prend les trois champs sans lesquels une ligne de journal ne veut rien dire —
action, entité, identifiant. `actor` et `changes` sont des ajouts en chaîne : ce que
l'appelant n'a pas à choisir, il n'a pas à l'écrire. `record` rend l'`Uuid` de la ligne
posée, comme `enqueue`.

## Les fichiers

```
templates/features/audit/
  feature.toml         description, ancre `features`, migration, feature `with-json` de sea-orm
  mod.rs.jinja         `Entry`, les trois constantes, réexport de `record`
  model.rs.jinja       l'entité `audit_log`
  repository.rs.jinja  `record` — la seule couche qui construit une requête
  migration.rs.jinja   la table et ses deux index
  tests.rs.jinja       les tests livrés avec le fragment
```

Le découpage suit la frontière du projet : `repository.rs` construit la requête, `mod.rs`
porte le type d'entrée et l'API publique. Aucun `controller` ni `service` — le fragment
n'expose pas de route, il outille celles du projet.

`feature.toml` déclare `[cargo.sea-orm] features = ["with-json"]` : la colonne `changes`
est une valeur JSON, que sea-orm ne sait lire que sous cette feature. `jobs` la déclare
déjà pour `payload` ; les deux fragments installés ensemble n'entrent pas en conflit,
`add_feature_to_dependency` (`metadata.rs:390`) étant idempotent.

Aucune section de configuration : le fragment n'a rien à régler. Aucune ancre hors
`features` : il ne monte ni route, ni layer, ni tâche de fond.

## Ce que le CLI doit apprendre

Trois lignes, et rien de plus — le mécanisme d'installation est générique :

- `cli.rs:56` : `audit` rejoint la liste des features de l'aide du drapeau.
- `lib.rs:451` : le conseil post-installation, qui doit dire la migration et le geste —
  « rbs migrate up, puis appelez `audit::record` dans vos services ».
- `docs/docs/cli/add.md:283` fige le message d'erreur qui énumère les features
  installables. Il change, et son transcript avec.

## Tests

**Du fragment, dans le projet engendré** (`tests.rs.jinja`, joués par la suite du projet) :

1. `record` pose une ligne lisible — acteur, action, entité, identifiant et `changes` s'y
   retrouvent tels qu'ils ont été passés.
2. Une entrée sans `actor` pose `actor_id` à `NULL` et non à une chaîne vide.
3. **`record` dans une transaction annulée n'écrit rien.** C'est la garantie centrale du
   fragment ; sans ce test elle n'est qu'une intention.
4. Deux entrées sur la même ligne se relisent dans l'ordre par `(entity, entity_id)`.

**Du CLI** (`crates/rbs-cli/src/`, sans Docker) :

5. Le manifeste du fragment est lisible et déclare la migration et l'ancre `features`.
6. Le rendu des templates est un point fixe de `rustfmt` — le balayage existant des
   fragments couvre les nouveaux fichiers dès qu'ils sont là.

**D'intégration** (`crates/rbs-cli/tests/`, sous Docker) :

7. `rbs new` puis `rbs add audit` produit un projet qui compile et passe
   `clippy -D warnings`.
8. La migration s'applique et la suite du projet engendré passe, PostgreSQL monté.

## Documentation

- `docs/docs/guides/audit.md` et sa version française : à quoi sert le journal, l'appel
  dans un service, l'acteur sous `auth` et sans, ce que le fragment ne fait pas — il ne
  câble aucune route existante, et c'est un choix.
- La ligne du tableau de `docs/docs/cli/add.md` et sa version française.
- Les deux transcripts qui énumèrent les features installables.
