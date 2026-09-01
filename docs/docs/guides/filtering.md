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
