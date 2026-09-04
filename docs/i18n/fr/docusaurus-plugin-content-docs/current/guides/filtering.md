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

## Pagination par curseur, pour les listes qui débordent un offset

`Pagination` fait parcourir au moteur les lignes qu'il va jeter, et une insertion survenue
entre deux requêtes décale la fenêtre — la page 2 réaffiche une ligne que la page 1 avait
déjà rendue. Au-delà de quelques milliers de lignes, `Cursor` la remplace :

```text
GET /articles?after=0199e0b1-9c4a-7c3e-9d21-6f2a1b0c4d5e&per_page=50
```

`after` est l'`id` de la dernière ligne qui vous a été servie, et la borne est exclusive.
Omettez-le pour la première page. Un `after` illisible répond 400 ; un `per_page` au-delà
de 100 est ramené en silence, exactement comme pour `Pagination`.

La réponse abandonne les décomptes :

```json
{
  "data": [ … ],
  "meta": { "per_page": 50, "next": "0199e0b1-9c4a-7c3e-9d21-6f2a1b0c4d5e" }
}
```

`next` est nul une fois la marche terminée. Il n'y a pas de `total` : le `COUNT(*)` qu'il
demanderait est précisément le coût que le curseur existe pour éviter.

Le CRUD engendré garde `Pagination` — basculer retirerait `total` de toutes les réponses
que vos clients lisent déjà. `Cursor` est là pour les routes que vous écrivez vous-même :

```rust
let mut query = Entity::find().order_by_desc(Column::Id);
if let Some(after) = cursor.after() {
    query = query.filter(Column::Id.lt(after));
}
let rows = query.limit(cursor.per_page()).all(db).await?;
let dernier = rows.last().map(|row| row.id);

Ok(Json(CursorPage::new(
    rows.into_iter().map(Into::into).collect(),
    &cursor,
    dernier,
)))
```

Le curseur n'avance que sur l'`id` décroissant — l'ordre que `list` applique déjà, et celui
que l'UUIDv7 rend total. Il ne suit pas un `sort` que vous auriez choisi : sur une colonne
où deux lignes partagent une valeur, la frontière serait ambiguë et la page suivante
sauterait des lignes ou en répéterait.

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
