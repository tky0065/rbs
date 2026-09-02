---
sidebar_position: 5.7
title: Filtering
---

# Filtering and sorting

A generated list answers `GET /articles?page=2`, and nothing more. Narrowing it — the
published ones, those with more than ten views, those whose title contains "rust" — meant
writing the query by hand, in the one file the generator had just written for you.

`rbs generate crud` now emits a seventh file, `filter.rs`, and mounts a route that reads
it:

```text
POST /articles/filter
```

## The body carries the conditions

```json
{
  "published": true,
  "views": { "gte": 10, "lt": 100 },
  "title": { "contains": "rust" },
  "sort": ["-views", "title"]
}
```

Every condition is an **AND**. There is no `or`, no nested group, no `in`: that would be a
query engine to generate, test and bound against pathological requests, and the escape
hatch is one you already have — `repository.rs` is yours to edit.

A bare value means equality, so `"published": true` and `"published": { "eq": true }` say
the same thing, and the short form is the one you write most often.

| Operator | Applies to | Meaning |
|---|---|---|
| `eq` | every column | strict equality |
| `gt`, `gte`, `lt`, `lte` | `int`, `float`, `datetime`, `uuid` | comparison |
| `contains` | `string`, `text` | substring, `LIKE '%…%'` |
| `is_null` | every column | `true` requires null, `false` requires a value |

`contains` follows the engine's collation: PostgreSQL distinguishes case, MySQL ignores it
with its default collation. `ILIKE` would settle it, but sea-orm only exposes it through
`PgExpr`, and rbs generates for MySQL and SQLite too.

## Sorting

`sort` is a list of column names, `-` prefixing the descending ones. Without it the order
stays the descending `id` — a UUIDv7, which is what makes pagination stable.

## Pagination stays in the query string

```text
POST /articles/filter?page=2&per_page=50
```

`Pagination` is an extractor that applies to any request, filtered or not. Putting it in
the body as well would give one value two sources.

## Cursor pagination, for lists that outgrow an offset

`Pagination` asks the engine to walk past the rows it is about to discard, and an insert
between two requests shifts the window — page 2 repeats a row page 1 already showed. Past
a few thousand rows, `Cursor` replaces it:

```text
GET /articles?after=0199e0b1-9c4a-7c3e-9d21-6f2a1b0c4d5e&per_page=50
```

`after` is the `id` of the last row you were served, and it is exclusive. Leave it out for
the first page. A malformed `after` answers 400; a `per_page` beyond 100 is quietly capped,
exactly as it is for `Pagination`.

The response drops the counts:

```json
{
  "data": [ … ],
  "meta": { "per_page": 50, "next": "0199e0b1-9c4a-7c3e-9d21-6f2a1b0c4d5e" }
}
```

`next` is null once the walk is over. There is no `total`: the `COUNT(*)` it needs is the
cost the cursor exists to avoid.

The generated CRUD keeps `Pagination` — switching it would drop `total` from every
response your clients already read. `Cursor` is there for the routes you write yourself:

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

The cursor only walks `id` descending — the order `list` already applies, and the one a
UUIDv7 makes total. It does not follow a `sort` you chose: on a column where two rows share
a value, the boundary would be ambiguous and the next page would skip or repeat rows.

## No column name ever reaches the database

The filter is typed by the columns of `--fields`, at generation time:

```rust file=examples/hello-crud/src/articles/filter.rs region=champs
```

A name coming from the client is only ever a `match` arm, written before the project was
compiled:

```rust file=examples/hello-crud/src/articles/filter.rs region=colonnes
```

An unknown sort column is refused with a 400 that names the ones that are accepted; an
unknown key in the body is ignored by serde. Nothing interpolates a client string into SQL,
so there is nothing to escape and nothing to inject.

## The query is built in one place

`apply` is the only function that touches a query, and `repository.rs` is its only caller —
the rule that only the repository builds a query does not move:

```rust file=examples/hello-crud/src/articles/filter.rs region=conditions
```

`list` is the empty filter, so the list and the filtered list share a single path. Two
`order_by` calls would diverge the first time one of them changed.

## Every column is filterable, and that has a cost

Including the ones that carry no index. `published:bool` is the very field you want to
filter on, and indexing it is rarely worth it — but a filter on an unindexed column scans
the table. When one of them grows, add `index` to the field and regenerate the migration:

```text
rbs g crud articles --fields "title:string:index, published:bool"
```

## What it does not do

- no `or`, no nested groups, no `in`;
- no full-text search — `contains` is a `LIKE`, and a real search wants an index rbs does
  not create;
- no filtering across a relation: `author_id` is filterable, the author's name is not.

Each of these stops at the same place: the generated file is yours, and `filter.rs` is a
few dozen readable lines to extend.
