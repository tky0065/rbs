# Filtres et tri dans le CRUD engendré

**Tâche 57 d'`IMPROVE.md`.** `templates/feature/repository.rs.jinja:19` trie par
`order_by_desc(Column::Id)` en dur, et rien ne permet de restreindre une liste. Or
`generate/fields.rs` connaît le type et le nom de chaque colonne à la génération : un
filtre typé peut s'engendrer sans qu'aucun nom de colonne ne vienne jamais de la requête.

## Ce qui est décidé

**Une route, `POST /<module>/filter`, dont le corps porte le filtre.** Le corps autorise
des valeurs longues et des opérateurs nommés que l'URL rendrait illisibles.

```json
{
  "published": true,
  "views": { "gte": 10, "lt": 100 },
  "title": { "contains": "rust" },
  "author_id": { "is_null": false },
  "sort": ["-views", "title"]
}
```

Toutes les conditions se composent en **ET**. Il n'y a ni `or`, ni groupe imbriqué, ni
`in` : c'est un moteur de requêtes qu'il faudrait alors engendrer, tester et borner contre
les requêtes pathologiques.

**La pagination reste en chaîne de requête** — `POST /articles/filter?page=2&per_page=50`.
`Pagination` est un `FromRequestParts` (`crates/rbs-core/src/pagination.rs:73`) qui
s'applique à n'importe quelle requête : la dupliquer dans le corps donnerait deux sources
pour une même donnée.

## La frontière noyau / engendré

Elle suit la règle du projet : au noyau ce qui n'a aucune raison de varier, au projet ce
que l'utilisateur voudra lire et modifier.

### `crates/rbs-core/src/filter.rs` — les opérateurs

Trois types, dont aucun ne connaît de colonne ni d'entité :

| Type | Rôle |
|---|---|
| `Comparison<T>` | `eq`, `gt`, `gte`, `lt`, `lte`, `is_null` sur un scalaire ordonné |
| `TextMatch` | `eq`, `contains`, `is_null` — `contains` rend un `LIKE '%…%'`, dont la casse suit la collation du moteur |
| `Sort` | une liste de colonnes préfixées, `-` pour décroissant |

Tous trois sont `Deserialize` et `ToSchema`. `Sort` ne fait qu'analyser la syntaxe du
préfixe ; il ne connaît aucun nom de colonne valide et n'en valide aucun.

`Comparison<T>` et `TextMatch` se désérialisent sous **deux formes**, par `#[serde(untagged)]` :
une valeur nue vaut `eq`, un objet nomme ses opérateurs. `"published": true` et
`"published": { "eq": true }` disent donc la même chose, et la forme courte est ce qu'un
client écrit dans le cas courant. `"is_null": false` signifie « la colonne n'est pas nulle » ;
sur une colonne non `optional`, l'opérateur reste accepté et ne retire rien.

`rbs-core` est en `1.2.0`, **non publiée** — crates.io s'arrête à `1.1.0`. L'ajout, qui est
purement additif, entre dans la version en cours : ni montée de version ni note de
migration.

### `src/<module>/filter.rs` — les colonnes

Le CLI engendre un septième fichier par feature. Il porte le type nommé d'après l'entité,
dont chaque champ vient de `--fields` :

```rust
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct ArticleFilter {
    pub title: Option<TextMatch>,
    pub views: Option<Comparison<i32>>,
    pub published: Option<Comparison<bool>>,
    pub sort: Option<Sort>,
}

pub(super) fn apply(select: Select<Entity>, filtre: &ArticleFilter) -> Select<Entity>
```

`apply` est un `match` sur des `Column::` **écrites à la génération**. Aucun nom de colonne
ne traverse la requête : il n'y a rien à valider à l'exécution, et rien à injecter. Un champ
inconnu du corps est ignoré par serde ; une colonne inconnue dans `sort` est refusée en 400
par la fonction `column_of` engendrée, qui nomme les colonnes acceptées.

**La couche reste celle du repository** : `filter.rs` construit des conditions SeaORM, et
`repository.rs` est son seul client. Le controller et le service ne font que transporter le
type — la règle « seul le repository construit une requête » ne bouge pas.

## Ce qui est filtrable

Tout scalaire et toute référence, plus `id`, `created_at` et `updated_at`. `contains` n'est
offert que sur `string` et `text`.

Aucune restriction aux colonnes indexées : `published:bool` est l'exemple canonique du
champ que l'on veut filtrer, et il n'est pas indexé. Le fichier engendré porte donc le
commentaire qui énonce le coût — un filtre sur une colonne sans index parcourt la table,
`index` s'ajoute dans `--fields`.

## Le montage

`POST /<module>/filter` se monte **avant** `/<module>/{id}`, sans quoi `filter` se lit comme
un identifiant. Le précédent est dans le dépôt : `examples/newsletter-queue` monte
`broadcast` avant `/subscribers/{id}` pour cette raison exacte.

La route reste **ouverte** quand `--role` protège la feature : c'est une lecture, comme
`list` et `find`, et `examples/blog-auth` ne garde que les trois écritures.

`repository::list` cesse de trier en dur. En l'absence de `sort`, **`-id` reste le tri par
défaut** : l'`id` est un UUIDv7, c'est lui qui rend la pagination stable, et les tests
engendrés existants en dépendent.

## Ce que la tâche traîne derrière elle

- le `#[utoipa::path]` de la route, et le corps déclaré à l'ancre `<rbs:openapi>` ;
- un scénario dans `templates/feature/tests.rs.jinja` : créer, filtrer, retrouver — lui
  aussi `#[ignore]`, comme ses voisins depuis la tâche 87 ;
- les quatre projets d'`examples/` régénérés, `integration_examples` étant l'oracle ;
- la page de documentation bilingue, dont les extraits sont tirés d'`examples/` et non
  écrits à la main.

## Vérification

TDD sur le noyau (désérialisation des trois types, analyse du préfixe de tri, refus d'une
colonne inconnue) et sur chaque générateur touché. Puis `integration_crud`, étendu au
filtre : il compile le projet engendré et l'exécute contre un PostgreSQL réel, ce qui est
la seule preuve que la requête produite est valide.
