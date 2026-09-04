# Pagination par curseur dans `rbs-core`

**Tâche 75 d'`IMPROVE.md`.** `crates/rbs-core/src/pagination.rs:62` calcule un `offset()`,
et le repository engendré le passe à SeaORM (`templates/feature/repository.rs.jinja:67`).
Un `OFFSET n` fait parcourir au moteur les `n` lignes qu'il va jeter : au-delà de quelques
milliers de lignes la dernière page coûte le balayage de toute la table. Et entre deux
requêtes, une insertion décale la fenêtre — la page 2 réaffiche une ligne déjà vue en
page 1, ou en saute une.

## Ce qui est décidé

**L'ajout est opt-in et cantonné à `rbs-core`.** Le CRUD engendré ne change pas : il garde
`Pagination`, `Page<T>` et son `meta.total`. Un projet existant régénéré produit le même
code, et aucun contrat JSON déjà servi ne bouge.

C'est un arbitrage assumé. Basculer le CRUD sur le curseur retirerait `total` et
`total_pages` des réponses de tout projet engendré — une rupture observable par chaque
client déjà écrit, pour un gain qui ne se manifeste qu'au-delà de quelques milliers de
lignes. Le noyau offre l'outil ; le projet qui en a besoin l'appelle.

## Ce que le noyau gagne

Deux types, dans le module `pagination` où vit déjà leur parenté.

### `Cursor` — l'extracteur

```rust
pub struct Cursor {
    after: Option<Uuid>,
    per_page: u64,
}
```

Un `FromRequestParts` frère de `Pagination`, lisant `?after=<uuid>&per_page=<n>`.

| Cas | Réponse |
|---|---|
| `after` absent | première page, `after == None` |
| `after` illisible | **400**, comme `per_page=abc` aujourd'hui |
| `per_page` hors bornes | ramené dans `1..=PAR_PAGE_MAX`, **en silence** |
| `per_page` absent | `PAR_PAGE_PAR_DEFAUT` |

L'asymétrie entre le bornage muet et le refus bruyant n'est pas une inconséquence : elle
est déjà celle de `Pagination` (`pagination.rs:80-82`), et pour la même raison — ignorer
une valeur illisible ferait débugger au client une pagination qui « ne marche pas ».

Les trois constantes du module (`PAGE_PAR_DEFAUT`, `PAR_PAGE_PAR_DEFAUT`, `PAR_PAGE_MAX`)
sont partagées, non redéclarées. `PAGE_PAR_DEFAUT` ne concerne pas le curseur.

**Le curseur est l'`id` en clair, non encodé.** Il est déjà public dans chaque réponse, et
un base64 ne cacherait rien qu'un client ne lise à la ligne du dessus. Ce qu'un encodage
opaque achète — la liberté de changer la forme du curseur sans casser les clients — ne
s'achète pas ici : la forme est un UUIDv7, et c'est la clé primaire de toute entité que rbs
engendre (`templates/feature/model.rs.jinja`).

### `CursorPage<T>` — la page rendue

```json
{
  "data": [ … ],
  "meta": { "per_page": 20, "next": "0199…" }
}
```

Pas de `total`, pas de `total_pages` : le `COUNT(*)` est précisément ce que le curseur
existe pour ne pas payer. Les porter obligerait à relancer le balayage que l'on vient
d'éviter.

`next` est l'`id` du dernier élément rendu, ou `null` quand la page est la dernière. La
distinction se lit sans compter : un appelant qui a demandé `per_page` lignes et en a reçu
moins est au bout. `CursorPage::new(data, &cursor)` pose `next` d'après cette règle, pour
que chaque repository ne la réécrive pas avec une chance sur deux de se tromper.

`Serialize` et `ToSchema` seulement, comme `Page<T>` : la page se rend, elle ne se relit
pas.

## Ce que ça ne fait pas

**Aucun curseur sur un tri arbitraire.** `Sort` (`filter.rs:111`) reste offset-only. Un
curseur ne sait avancer que sur une clé totalement ordonnée et stable ; sur `-views`, deux
lignes à la même valeur rendraient une frontière ambiguë et la page suivante sauterait des
lignes ou en répéterait. Le faire correctement demande un curseur composite
`(colonne, id)`, ce qui est un autre sujet — et le prétendre ici engendrerait des pages
fausses sans que rien ne le signale.

L'ordre du curseur est donc figé : `id` décroissant. C'est déjà celui que `list` applique
en dur (`repository.rs.jinja:53`), pour la même raison — l'UUIDv7 est monotone, donc
l'`id` décroissant est l'ordre d'insertion inversé.

## Ce que le repository appelant écrit

Non engendré, mais documenté :

```rust
let mut query = Entity::find().order_by_desc(Column::Id);
if let Some(after) = cursor.after() {
    query = query.filter(Column::Id.lt(after));
}
let items = query.limit(cursor.per_page()).all(db).await?;
```

`lt` et non `lte` : la borne est exclusive, faute de quoi chaque page réafficherait la
dernière ligne de la précédente.

## Tests

Le harnais `query()` des tests existants (`pagination.rs:127`) monte un routeur, extrait,
et rend `(statut, JSON)`. Le curseur s'y branche sans rien réécrire.

| Test | Ce qu'il prouve |
|---|---|
| `the_first_page_needs_no_cursor` | `after` absent → 200, `after == None` |
| `an_unreadable_cursor_answers_400` | `after=pas-un-uuid` → 400 |
| `the_cursor_shares_the_page_size_bounds` | `per_page=5000` → `PAR_PAGE_MAX`, muet |
| `a_full_page_names_its_successor` | `per_page` lignes rendues → `next` = `id` de la dernière |
| `a_short_page_ends_the_walk` | moins de `per_page` lignes → `next` nul |

Les deux derniers portent sur `CursorPage::new` et non sur l'extracteur.

## Version et documentation

`rbs-core` est en `1.2.0`, **non publiée** — crates.io s'arrête à `1.1.0`. L'ajout est
purement additif : il entre dans la version en cours, sans montée ni note de migration.
`cargo semver-checks -p rbs-core --all-features` en est la preuve.

Trois écritures obligatoires :

- `CHANGELOG.md` et `CHANGELOG.fr.md`, section `[Unreleased] / Added` ;
- `docs/docs/guides/filtering.md`, sous la section « Pagination stays in the query
  string » (`:53`), et sa paire française — la règle de parité du projet ne souffre pas
  d'exception, et `docs/scripts/parite.mjs` la contrôle ;
- le `///` de chaque item public, `#![warn(missing_docs)]` étant posé sur la crate.
