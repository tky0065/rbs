---
sidebar_position: 5.5
title: Relations
---

# Relations

A generator that produces a CRUD feature but cannot produce a foreign key leaves its user
writing, by hand, the exact part SeaORM makes the most tedious: the `Relation` variant, the
`impl Related`, the migration's constraint, and the index that constraint needs. The eighth
type of [`--fields`](../cli/generate.md#the-eight-types), `references`, closes that gap —
entirely from the command line, with no database running.

```text
rbs g crud posts --fields "title:string, author:references:users"
```

## A reference is a field, because it is a column

`belongs_to` lays down a column — `author_id` — so it lives in `--fields` like any other
field, not behind a separate flag. The name declared is the relation's, `author`; the column
is derived from it. This derivation is what lets the SeaORM variant, the foreign key and the
migration agree on a name without the command line repeating it three times:

```rust title="src/posts/model.rs"
#[sea_orm(indexed)]
pub author_id: Uuid,
```

```rust title="src/posts/model.rs"
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::auth::model::user::Entity",
        from = "Column::AuthorId",
        to = "crate::auth::model::user::Column::Id",
        on_delete = "Restrict"
    )]
    Author,
    // <rbs:relations:posts>
    // </rbs:relations:posts>
}
```

```rust title="migration/.../m…_create_posts.rs"
.foreign_key(
    ForeignKey::create()
        .name("fk_posts_author_id")
        .from(Posts::Table, Posts::AuthorId)
        .to(Users::Table, Users::Id)
        .on_delete(ForeignKeyAction::Restrict),
)
```

The third segment of the field is the target: the name of a table as it exists in the project,
not a type the CLI invents on the spot. `rbs generate` inventories every entity under `src/`
before looking at what you typed, including one nested in a module rather than living under
its own directory — in a project with `auth`, `users` sits inside `src/auth/model.rs`, next to
`refresh_tokens`, which is exactly why the two snippets above name
`crate::auth::model::user::Entity` for a field that only ever wrote `users`. A target absent
from that inventory is refused, by name, next to the ones the CLI does know:

```text
$ rbs g crud comments --fields "body:text, author:references:writers" --dry-run
erreur : relation « author » — « writers » est introuvable dans ce projet
        → entités connues : comments, posts, refresh_tokens, users
```

A target the inventory does know, but whose table no migration creates yet, is refused on the
same principle — a foreign key pointed at it would fail the moment migrations run, far from
the command that wrote it:

```text
$ rbs g crud comments --fields "body:text, draft:references:drafts" --dry-run
erreur : relation « draft » — « drafts » n'a pas de migration dans ce projet
        → une clé étrangère la viserait avant qu'aucune migration ne crée sa table : écrivez sa migration avec `rbs migrate new`
```

## Two shapes, and an index that isn't optional

A bare reference is many-to-one: any number of posts can name the same author. `unique` costs
no extra grammar to turn it one-to-one — it is a `belongs_to` whose column happens to be
unique, which is exactly what a one-to-one relation is in a relational database:

```text
owner:references:users:unique:cascade
```

```rust title="src/profiles/model.rs"
#[sea_orm(unique)]
pub owner_id: Uuid,
```

```rust title="migration/.../m…_create_profiles.rs"
.col(
    ColumnDef::new(Profiles::OwnerId)
        .uuid()
        .not_null()
        .unique_key(),
)
.foreign_key(
    ForeignKey::create()
        .name("fk_profiles_owner_id")
        .from(Profiles::Table, Profiles::OwnerId)
        .to(Users::Table, Users::Id)
        .on_delete(ForeignKeyAction::Cascade),
)
```

Every other reference gets an index it never asked for — `idx_posts_author_id` above — because
it isn't optional: without one on the carrying column, deleting a row in `users` would make
PostgreSQL scan every row of `posts` to check the constraint it is about to violate. `unique`
already indexes the column by being a constraint, which is why `profiles` gets no separate
`create_index` call. Asking for `index` on top of either is refused as redundant — the same
refusal a plain `unique:index` gets on an ordinary column, and the reason a reference never
takes `index` explicitly at all:

```text
$ rbs g crud comments --fields "body:text, author:references:users:index" --dry-run
erreur : champ 2 « author » — « index » redondant : une clé étrangère est déjà indexée
        → retirez « index »
```

`optional` and the two delete policies read the same way they would on any foreign key:
`cascade` for `ON DELETE CASCADE`, above, and `nullify` for `ON DELETE SET NULL`. `nullify` on
a `NOT NULL` column is refused rather than silently requiring `optional` for you — a column's
nullability is not something this grammar infers from a policy chosen three words later — and
asking for both policies at once is refused as the contradiction it is:

```text
$ rbs g crud comments --fields "body:text, author:references:users:nullify" --dry-run
erreur : champ 2 « author » — « nullify » sur une colonne non nullable
        → ajoutez « optional », ou choisissez « cascade »

$ rbs g crud comments --fields "body:text, author:references:users:optional:cascade:nullify" --dry-run
erreur : champ 2 « author » — « cascade » et « nullify » se contredisent
        → gardez l'un des deux
```

Left unstated, a reference is `ON DELETE RESTRICT`, as `posts.author_id` is above: deleting a
referenced row fails rather than taking its dependents down with it, silently, in an order
nobody chose.

## The side that wasn't asked for

Declaring `author:references:users` on `posts` implies `users` has posts. Rather than make
that a second flag to keep in sync with the first, the CLI writes the reverse `has_many` itself,
in the same run, into the target's own model:

```rust title="src/auth/model.rs (pub mod user)"
pub enum Relation {
    // <rbs:relations:users>
    #[sea_orm(has_many = "crate::posts::model::Entity")]
    Posts,
    // </rbs:relations:users>
}

// <rbs:related:users>
impl Related<crate::posts::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}
// </rbs:related:users>
```

Two anchors, not one: a `Relation` variant lives inside the enum's braces, an `impl Related`
cannot. Their file is found the same way the target itself was — from the entity inventory,
not from a directory guessed off the table's name — which is why the module-nesting case above
matters here too. On a project generated before this anchor existed, the CLI writes nothing
into that file rather than guess where the insertion belongs; it finishes everything else and
prints the block to paste in by hand, the same rule [every anchor in this
generator](../cli/generate.md#anchors) follows.

`--has-many` exists for exactly that repair, and for it alone: a child that already carries its
key, whose parent never received the variant back. It refuses a parent it cannot find and a
child that does not actually carry the expected key — the second check exists so that the
variant it writes is one SeaORM would accept, rather than one that fails to compile forty
seconds later:

```text
$ rbs g crud users --has-many categories --dry-run
erreur : categories ne porte aucune colonne référençant `users` : ajoutez-la avant de relancer `--has-many categories`
```

## A target claimed twice loses the shortcut

Two relations naming the same target — `author` and `reviewer`, both `references:users` — are
a real shape, not a mistake. But `impl Related<T>` takes only the target type as its key: two
implementations for the same pair of types is `rustc` refusing a duplicate trait impl, not a
choice the CLI gets to arbitrate. So neither is written, on either side, and a comment explains
why in the two places the missing code would otherwise have been:

```rust title="src/reviews/model.rs"
// `users` est visée par 2 relations (`Author`, `Reviewer`) : `Related` serait ambigu, et son modèle ne reçoit donc pas non plus le `has_many` en retour, qui l'exige. Joindre explicitement, par exemple
// `Entity::find().join(JoinType::LeftJoin, Relation::Author.def())`.
```

```rust title="src/auth/model.rs (pub mod user)"
// `reviews` vise cette table par 2 relations (`Author`, `Reviewer`) : pas de `has_many`
// ici, `EntityTrait::has_many` exigeant le `Related` que `reviews` ne peut pas poser
// sans arbitrer entre elles. Joindre explicitement depuis le côté portant.
```

The columns, the foreign keys and the plain `Relation` variants are unaffected — it is only the
shortcut that a single ambiguous trait cannot provide.

## A required reference leaves its entity unseeded

`rbs generate crud` writes a seed file alongside the feature by default, one row it can insert
without asking anything of the database. A required reference breaks that: the seed would need
a real `author_id` from `users`, and has no row to point at that it can defend as correct. Owed
its own explanation rather than a silent omission, so the command says exactly why it skipped
it:

```text
aucun seed pour posts : la référence « author » est requise, et un seed ne peut pas deviner vers quelle ligne pointer
```

Making the reference `optional` sidesteps the problem outright — an unseeded `posts` seeds
cleanly with `author_id` absent — and remains the only fix available from `--fields` alone;
everything else this page covers still applies to an optional reference exactly as it does to
a required one.
