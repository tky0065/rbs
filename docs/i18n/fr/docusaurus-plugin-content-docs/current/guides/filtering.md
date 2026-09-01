---
sidebar_position: 5.7
title: Filtrage
---

# Filtrage et tri

Une liste engendrée répond à `GET /articles?page=2`, et rien de plus. La restreindre — les
publiés, ceux qui dépassent dix vues, ceux dont le titre contient « rust » — supposait
d'écrire la requête à la main, dans le seul fichier que le générateur venait d'écrire pour
vous.

`rbs generate crud` produit désormais un septième fichier, `filter.rs`, et monte la route
qui le lit :

```text
POST /articles/filter
```

## Le corps porte les conditions

```json
{
  "published": true,
  "views": { "gte": 10, "lt": 100 },
  "title": { "contains": "rust" },
  "sort": ["-views", "title"]
}
```

Toutes les conditions se composent en **ET**. Il n'y a ni `or`, ni groupe imbriqué, ni
`in` : ce serait un moteur de requêtes à engendrer, à tester et à borner contre les
requêtes pathologiques, et l'échappatoire existe déjà — `repository.rs` vous appartient.

Une valeur nue vaut une égalité : `"published": true` et `"published": { "eq": true }`
disent la même chose, et la forme courte est celle que l'on écrit le plus souvent.

| Opérateur | S'applique à | Sens |
|---|---|---|
| `eq` | toute colonne | égalité stricte |
| `gt`, `gte`, `lt`, `lte` | `int`, `float`, `datetime`, `uuid` | comparaison |
| `contains` | `string`, `text` | sous-chaîne, `LIKE '%…%'` |
| `is_null` | toute colonne | `true` exige une colonne nulle, `false` une colonne renseignée |

`contains` suit la collation du moteur : PostgreSQL distingue la casse, MySQL l'ignore avec
sa collation par défaut. `ILIKE` trancherait, mais sea-orm ne l'expose que par `PgExpr`, et
rbs engendre aussi pour MySQL et SQLite.

## Le tri

`sort` est une liste de noms de colonnes, `-` préfixant les décroissantes. Sans elle,
l'ordre reste l'`id` décroissant — un UUIDv7, ce qui rend la pagination stable.

## La pagination reste dans la chaîne de requête

```text
POST /articles/filter?page=2&per_page=50
```

`Pagination` est un extracteur qui s'applique à n'importe quelle requête, filtrée ou non.
La porter aussi dans le corps donnerait deux sources à une même valeur.

## Aucun nom de colonne n'atteint la base

Le filtre est typé par les colonnes de `--fields`, à la génération :

```rust file=examples/hello-crud/src/articles/filter.rs region=champs
```

Un nom venu du client n'est jamais qu'un bras de `match`, écrit avant que le projet ne soit
compilé :

```rust file=examples/hello-crud/src/articles/filter.rs region=colonnes
```

Une colonne de tri inconnue est refusée par un 400 qui nomme celles qui sont acceptées ;
une clé inconnue du corps est ignorée par serde. Rien n'interpole une chaîne du client dans
du SQL : il n'y a rien à échapper, et rien à injecter.

## La requête se construit en un seul endroit

`apply` est la seule fonction qui touche une requête, et `repository.rs` en est le seul
appelant — la règle selon laquelle seul le repository construit une requête ne bouge pas :

```rust file=examples/hello-crud/src/articles/filter.rs region=conditions
```

`list` est le filtre vide : la liste et la liste filtrée partagent un seul chemin. Deux
`order_by` divergeraient dès que l'un des deux changerait.

## Toute colonne est filtrable, et cela a un coût

Y compris celles qui ne portent aucun index. `published:bool` est précisément le champ sur
lequel on veut filtrer, et l'indexer en vaut rarement la peine — mais un filtre sur une
colonne sans index parcourt la table. Quand l'une d'elles grandit, ajoutez `index` au champ
et régénérez la migration :

```text
rbs g crud articles --fields "title:string:index, published:bool"
```

## Ce qu'il ne fait pas

- ni `or`, ni groupes imbriqués, ni `in` ;
- aucune recherche plein texte — `contains` est un `LIKE`, et une vraie recherche demande un
  index que rbs ne crée pas ;
- aucun filtre à travers une relation : `author_id` est filtrable, le nom de l'auteur ne
  l'est pas.

Chacun de ces manques s'arrête au même endroit : le fichier engendré est le vôtre, et
`filter.rs` tient en quelques dizaines de lignes lisibles.
