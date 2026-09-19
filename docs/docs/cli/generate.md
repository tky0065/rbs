---
sidebar_position: 2
title: rbs generate
---

# `rbs generate`

Adds a feature to an existing project: the six files of the feature, plus — for `crud` —
a test file, a SeaORM entity and its migration, written from `--fields` with no database
running. It is the reverse of `sea-orm-cli generate entity`, which needs a schema first.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

{/* rbs:transcript cmd="rbs generate --help" */}
```text
$ rbs generate --help
Génère une feature dans un projet existant

Utilisation : rbs generate <COMMANDE>

Commandes :
  crud       Génère une feature CRUD complète, entité et migration comprises
  feature    Génère une feature vide : six fichiers, aucun champ
  client     Engendre un client typé depuis le document OpenAPI du projet
  job        Génère un job de la file, et son échéance sous --every ; exige la feature jobs
  migration  Écrit une migration d'évolution : des colonnes de plus sur une table existante
  help       Affiche cette aide, ou celle des commandes données

Options :
  -h, --help     Affiche l'aide
  -V, --version  Affiche la version
```

`g` is an alias for `generate`: `rbs g crud users` and `rbs generate crud users` parse to
the same thing.

`rbs generate` takes neither `--template-dir` nor `--yes`: it asks no questions, and its
templates are compiled into the binary rather than read from a directory. Passing either is
a clap error rather than a flag that is taken and ignored.

## `rbs generate crud`

{/* rbs:transcript cmd="rbs generate crud --help" */}
```text
$ rbs generate crud --help
Génère une feature CRUD complète, entité et migration comprises

Utilisation : rbs generate crud [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom de la feature, au pluriel

Options :
      --fields <CHAMPS>    Champs de l'entité, ex. "name:string,status:enum(draft,published)"
      --singular <NOM>     Forme singulière du nom, quand l'heuristique se trompe (ex. news)
      --force              Écrit même si le working tree Git est sale
      --dry-run            Affiche le plan sans rien écrire
      --json               Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
      --has-many <ENTITE>  Entité enfant dont ce modèle doit porter la variante inverse, répétable
      --role <ROLE>        Relève à ce rôle le seuil des écritures ; exige la feature auth
      --soft-delete        Rend le DELETE logique : la ligne reste, marquée d'une date de suppression
      --with-upload        Ajoute trois routes de contenu binaire ; exige la feature storage
      --cursor             Pagine GET /<ressource> par curseur ; la route de filtre garde ses pages
      --no-admin           N'émet pas les écrans d'administration de la table, même si le shell est posé
  -h, --help               Affiche l'aide
  -V, --version            Affiche la version
```

| Flag | Effect |
|---|---|
| `--fields <CHAMPS>` | The entity's columns, in the grammar below. Omitted, the feature is generated with no column of its own. |
| `--singular <NOM>` | The singular form of the name, when the built-in heuristic gets it wrong. It names the entity, the DTOs and the local variables — `CreateNewsItem` and `let news_item` for `rbs generate crud news --singular news_item` — while the module, the table and the routes keep the plural. The heuristic already leaves `news`, `series` and `species` untouched; for any other invariable or irregular plural, this flag is the fix. Must be snake_case, and neither a Rust keyword nor a skeleton module, checked before anything is written. |
| `--force` | Writes even though the Git working tree is dirty, and overwrites files reported as conflicting. |
| `--dry-run` | Prints the plan and stops. Nothing is written. |
| `--json` | Prints the plan — or the error — as one JSON document on standard output instead of the coloured text: every action with its effect, the full content of created files, and `applique` to say whether anything was written. Independent of `--dry-run`, and accepted by `generate feature` too. [The agents guide](../guides/agents.md#reading-a-plan-as-json) has the document and the error codes. |
| `--has-many <ENTITE>` | Repairs the far side of a relation: writes into the model of an already generated feature the `has_many` variant pointing at the named child, and nothing else. Repeatable. [The relations guide](../guides/relations.md) covers when it is needed. |
| `--role <ROLE>` | Raises the threshold on the writes — `create`, `update`, `delete`, and the `PUT` of the content route when `--with-upload` comes along — to that role instead of the default `Role::User`. It opens nothing and closes nothing: on a project carrying `auth`, *every* generated route already takes an `Identity` and calls `require_role`, and the reads (`list`, `find`, `filter`, and the content route's `GET` and `HEAD`) simply keep the default threshold. Requires the [`auth`](../guides/auth.md) feature, and a role its `Role` enum declares — both are checked before anything is written. [The authentication guide](../guides/auth.md#closed-by-default-at-generation-time) explains what to remove to reopen a route. |
| `--soft-delete` | Makes `DELETE` logical instead of removing the row. The HTTP contract does not change, and a `unique` field's constraint narrows to live rows — on MySQL it stays global, so a deleted value stays reserved there. [The migrations guide](../guides/migrations.md#soft-delete) has the rest. |
| `--with-upload` | Mounts three routes on `/<resource>/{id}/content` — `PUT`, `GET`, `HEAD` — against the `storage` fragment's trait. Requires the [`storage`](../guides/storage.md) feature, and the fragment under `src/modules/storage/` where `rbs add` has laid it out since 1.3.0 — both are checked before anything is written, and a project that still carries `src/storage/` is refused until the directory is moved and its `use` statements fixed. With `--role`, the `PUT` joins the writes whose threshold the flag raises; with `--soft-delete`, the content outlives the row that `DELETE` only stamps. It also writes their tests into `tests/content.rs` — the round trip, the 404s, the 413, and the 401 under `auth`. [The storage guide](../guides/storage.md#generated-content-routes) has both. |
| `--cursor` | Pages `GET /<resource>` by cursor instead of by page number: the route takes `after` and `per_page`, and returns `data` with `meta.next` — the `id` to pass as the next `after`, `null` once the walk is over — and no `total`. `POST /<resource>/filter` keeps its pages, whatever its sort: a cursor on `id` is wrong as soon as the order follows another column. Combines with `--role`, with `--soft-delete` — deleted rows stay out of the walk — and with `--with-upload`. For an entity its tests can create — one without a required reference — the generated tests walk every page until `next` goes out, and check that no row comes back twice. [The filtering guide](../guides/filtering.md#cursor-pagination-for-lists-that-outgrow-an-offset) has the rest. |

## `rbs generate feature`

{/* rbs:transcript cmd="rbs generate feature --help" */}
```text
$ rbs generate feature --help
Génère une feature vide : six fichiers, aucun champ

Utilisation : rbs generate feature [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom de la feature

Options :
      --singular <NOM>  Forme singulière du nom, quand l'heuristique se trompe (ex. news)
      --force           Écrit même si le working tree Git est sale
      --dry-run         Affiche le plan sans rien écrire
      --json            Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
  -h, --help            Affiche l'aide
  -V, --version         Affiche la version
```

Same flags minus `--fields`, `--has-many` and `--role`: an empty feature has no columns, so
it gets neither an entity worth the name, nor a migration, nor a relation to repair; and it
carries no handler for a guard to protect. `--singular` stays: the skeleton still names its
service and its DTOs after the singular.

## `rbs generate job`

{/* rbs:transcript cmd="rbs generate job --help" */}
```text
$ rbs generate job --help
Génère un job de la file, et son échéance sous --every ; exige la feature jobs

Utilisation : rbs generate job [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom du job, en snake_case : celui de son module et de son KIND

Options :
      --every <CRON>  Expression cron de l'échéance, à cinq ou six champs, évaluée en UTC ; exige la feature scheduler
      --force         Écrit même si le working tree Git est sale
      --dry-run       Affiche le plan sans rien écrire
      --json          Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
  -h, --help          Affiche l'aide
  -V, --version       Affiche la version
```

Neither `--fields` nor an entity: a job is not a CRUD feature, and the manifest never
records its name. `<NAME>` is both the module under `src/modules/jobs/` and the `KIND` the
registry looks it up by, which is why it has to be a valid Rust identifier — refused
otherwise, along with a Rust keyword and a name that collides with one of the six files the
`jobs` fragment itself owns (`config`, `demo`, `model`, `queue`, `worker`, `tests`), or with
`jobs` — `pub mod jobs;` inside `src/modules/jobs/mod.rs` would name the module after its
own directory, which `clippy::module_inception` refuses. So is the name of a crate that file
or the job template reaches for — `std`, `core`, `alloc`, `serde`, `serde_json`, `anyhow`,
`async_trait`, `tracing`: declared there, the module would hide the crate from every
`serde::…` path of the file.

The command requires the `jobs` feature, and `--every` requires `scheduler` on top of it —
each refusal names the command that installs what is missing. A project that received
either before 1.3.0 still carries it under `src/jobs/` or `src/scheduler/`, which
`rbs upgrade` does not move: the command refuses it as well, naming the move to make by
hand. On a project carrying `jobs`:

{/* rbs:transcript cmd="rbs generate job purge_sessions --dry-run" setup="rbs new demo --yes --with jobs --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs generate job purge_sessions --dry-run
plan pour …/demo

  + src/modules/jobs/purge_sessions.rs   créé
  ~ src/modules/jobs/mod.rs              modifié

  1 à créer, 1 à modifier

  rien n'a été écrit (--dry-run)
```

Two anchors here, both deposited by the `jobs` fragment and not by the skeleton:
`// <rbs:job_modules>` declares the module, `// <rbs:jobs>` registers it with the worker.
On a project that also carries `scheduler`, `--every` adds a third file and a third anchor
— `// <rbs:schedules>` pushes the job's due date onto the calendar:

```text
$ rbs generate job purge_sessions --every "0 3 * * *" --dry-run
plan pour /private/tmp/rbs-demo/demo

  + src/modules/jobs/purge_sessions.rs   créé
  ~ src/modules/jobs/mod.rs              modifié
  ~ src/modules/scheduler/mod.rs         modifié

  1 à créer, 2 à modifier

  rien n'a été écrit (--dry-run)
```

Rerunning either command changes nothing: a job file that already exists is never
rewritten, `--force` included, and the plan reports it unchanged.

On a project generated before 1.5.0, `src/modules/jobs/mod.rs` has no
`// <rbs:job_modules>`. The job's file is still written, but its declaration is not — nor
are the registration and the due date, which name the module and would stop the project
from compiling without it: the plan prints the three blocks instead. `rbs doctor --fix`
puts that anchor back, after which rerunning the command writes the rest. A calendar still
written as a `vec![]` has no line to hang `// <rbs:schedules>` from; the plan says so
rather than promising `--fix`, and the
[scheduler guide](../guides/scheduler.md#a-calendar-predating-the-anchor) shows the
rewrite.

## `rbs generate migration`

{/* rbs:transcript cmd="rbs generate migration --help" */}
```text
$ rbs generate migration --help
Écrit une migration d'évolution : des colonnes de plus sur une table existante

Utilisation : rbs generate migration [OPTIONS] --add-column <TABLE> --fields <CHAMPS> <NAME>

Arguments :
  <NAME>  Nom de la migration, en snake_case : celui de son module

Options :
      --add-column <TABLE>  Table à modifier, telle que le projet la déclare
      --fields <CHAMPS>     Colonnes à ajouter, toutes optionnelles, ex. "statut:enum(draft,published):optional"
      --force               Écrit même si le working tree Git est sale
      --dry-run             Affiche le plan sans rien écrire
      --json                Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
  -h, --help                Affiche l'aide
  -V, --version             Affiche la version
```

The other four subcommands write a feature; this one changes a table that already exists.
It writes a single file — `migration/src/m<timestamp>_<name>.rs` — declared between
`// <rbs:migration_modules>` and registered in `// <rbs:migrations>`, like every migration
rbs generates. `up` stacks one `alter_table().add_column()` per field, one statement per
column because SQLite accepts a single alteration per `ALTER TABLE`; `down` undoes them in
reverse, dropping an index before the column it names, which SQLite also insists on. The
file declares its own minimal `Iden` — the table, and the columns it adds, and nothing
else: the others belong to the migration that created them.

```bash
rbs generate migration ajoute_statut --add-column articles \
  --fields "statut:enum(draft,published):optional,prix:decimal:optional"
```

`--fields` is the grammar described below, parsed by the same parser, with the same faults
and the same refusals. What differs is what an *added* column may be:

| Refused | Why |
|---|---|
| a column that is not `optional` | The table already holds rows, and they have no value for the new column. SQLite demands a default for a `NOT NULL` column added after the fact; the other two refuse the addition outright. The refusal names the field and the `:optional` that lifts it. A default is not offered: it would apply to the old rows and the new ones alike, and that choice belongs to the schema, not to the CLI. |
| `unique` | SQLite cannot add a column under a uniqueness constraint. The refusal holds on all three engines, PostgreSQL included — a generated migration is meant to run anywhere, and one rule is one rule — and points at `rbs migrate new` for the unique index written by hand. |
| `references` | SQLite cannot add a foreign key to a table that already exists. Add the column as `uuid:optional`, then write the constraint by hand. |
| `decimal` under SQLite | The refusal `rbs generate crud` prints, word for word: sqlx-sqlite declines to bind an exact decimal, whichever path the column arrives by. |

A table no module declares is refused too, and the message names what was searched —
`src/*/model.rs` — and the tables the project does declare. The lookup is that inventory
rather than a directory: the `users` table of an authenticated project lives under
`src/auth/model.rs`, not in a `src/users/`.

A `decimal` column still patches the manifest, through the same plan actions as
`generate crud`: `rust_decimal` with `serde-str`, and sea-orm's `with-rust_decimal`.

### What it prints rather than writes

The migration teaches the database about the column; the entity and its DTOs still have to
learn about it. `model.rs` and `dto.rs` carry no anchor, and the CLI never rewrites an AST
— so the command prints the lines to paste, and writes none of them:

{/* rbs:transcript cmd="rbs generate migration ajoute_statut --add-column articles --fields statut:enum(draft,published):optional,prix:decimal:optional --dry-run" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs generate crud articles --fields titre:string && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m crud" dans="demo" extrait="oui" */}
```text
$ rbs generate migration ajoute_statut --add-column articles --fields statut:enum(draft,published):optional,prix:decimal:optional --dry-run
plan pour …/demo

  + migration/src/m20260916_133333_ajoute_statut.rs   créé
  ~ migration/src/lib.rs                              modifié
  ~ Cargo.toml                                        modifié

  1 à créer, 2 à modifier

  à coller dans src/articles/model.rs :

    // aux imports, en tête du fichier
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    /// Valeurs acceptées par la colonne « statut ».
    ///
    /// Une valeur de plus s'ajoute ici et dans le `CHECK` que porte une migration nouvelle :
    /// la base refuse d'elle-même celles qu'elle ne connaît pas. Plus longue que toutes les
    /// actuelles, elle demande en troisième lieu d'élargir le `StringLen::N` ci-dessous, et
    /// avec lui le `string_len` de cette migration.
    #[derive(
        Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
    )]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::N(9))")]
    pub enum ArticleStatut {
        #[sea_orm(string_value = "draft")]
        #[serde(rename = "draft")]
        Draft,
        #[sea_orm(string_value = "published")]
        #[serde(rename = "published")]
        Published,
    }

    // dans `struct Model`
        pub statut: Option<ArticleStatut>,
        pub prix: Option<Decimal>,

  à coller dans src/articles/dto.rs :

    // aux imports, en tête du fichier
    use sea_orm::prelude::Decimal;

    // remplacez `use super::model::Model;` par :
    use super::model::{ArticleStatut, Model};

    // dans `CreateArticle`, `UpdateArticle` et `ArticleResponse`
        pub statut: Option<ArticleStatut>,
        #[schema(value_type = Option<String>, format = "decimal")]
        pub prix: Option<Decimal>,

  rien n'a été écrit (--dry-run)
```

Every added column being optional, the three DTOs take the same `Option<T>` line — a
required column would have told them apart. For an `enum(a,b,c)` field the model block also
carries the `DeriveActiveEnum` type to paste, exactly as `generate crud` renders it: the
migration's `CHECK` and the model's variants describe one and the same column, and a model
that disagreed would either refuse a value the database holds or offer one it rejects.

The blocks are computed against that module's own files rather than printed blindly, so
that following them to the letter compiles. Only the `use` lines it actually lacks appear:
the `serde` and `utoipa` imports a `DeriveActiveEnum` needs — the generated model carries
them only when it already had an enum — and the `sea_orm::prelude` name a `date` or
`decimal` column needs in the DTOs. The model import is given as an **edit** rather than an
addition, because that line already exists and the enum type joins it: adding it whole
would declare it twice. A module that already carries an enum is offered neither import.

Like the other subcommands, this one honours `--dry-run`, `--json` and `--force`, and goes
through the same plan: nothing is written until the whole plan is computed, and a partial
failure restores what it touched.

## The `--fields` grammar

One field per comma; within a field, colons separate a name, a type, and any number of
modifiers:

```text
nom:type[:modificateur…][,nom:type[:modificateur…]…]
```

Whitespace around every separator is ignored, so `" titre : string , email : string :
unique "` and `"titre:string,email:string:unique"` describe the same two fields. An empty
`--fields` declares no field at all. Fields keep their declaration order in the entity and
in the migration.

### The eleven types

There is no twelfth, and no `email` type: a string format is not a column type.

| Type | Rust | Migration |
|---|---|---|
| `string` | `String` | `string()` |
| `text` | `String` | `text()` |
| `int` | `i32` | `integer()` |
| `float` | `f64` | `double()` |
| `decimal` | `Decimal` | `decimal_len(19, 4)` |
| `bool` | `bool` | `boolean()` |
| `uuid` | `Uuid` | `uuid()` |
| `datetime` | `DateTimeWithTimeZone` | `timestamp_with_time_zone()` |
| `date` | `Date` | `date()` |
| `enum(a,b,c)` | an enum named after the entity and the field | `string_len(n)` under a `CHECK` |

`string` and `text` share a Rust type, so `text` is the only one that also carries an
explicit column type on the entity — without it SeaORM would infer `varchar`.

`float` and `decimal` both carry a fractional number, and only the second one carries it
exactly: `rust_decimal::Decimal`, a `DECIMAL(19, 4)` column — written out, because MySQL
would otherwise narrow a bare `DECIMAL` to `DECIMAL(10, 0)` — and a JSON string
(`"12.5000"`), so that a JavaScript client loses no cent to a float. Declaring one adds
`rust_decimal` to the project's manifest and sea-orm's `with-rust_decimal` feature; the
string form is pinned there by `serde-str` rather than left to the crate's default, and a
JSON number is refused rather than quietly rounded — which is the whole point of the type.
**SQLite refuses it**, before anything is written: sqlx-sqlite deliberately declines to
bind an exact decimal — its `NUMERIC` affinity keeps only fifteen significant digits — and
sea-query binds a `Decimal` for PostgreSQL and MySQL only. The refusal names the field and
offers the two fallbacks: `float`, or an integer of cents. A project that has already
pinned `rust_decimal` itself, at another version, gets the whole generation refused before
anything is written: a version someone chose is never rewritten, and the refusal names
both. Align the pin on the version asked for, or drop it and let the generation declare
it.

`enum(a,b,c)` is the only type carrying its own values. They are snake_case, distinct, and
at least one. The model declares an enum named after the entity and the field in PascalCase
— `status` on `articles` gives `ArticleStatus` — one variant per value, and the migration
bounds the column to the longest value under a `CHECK (status IN ('draft', 'published'))`
that holds on PostgreSQL, MySQL and SQLite alike. The entity is part of the name because
utoipa derives the OpenAPI component name from the Rust type name: two features each
declaring `status:enum(…)` would otherwise write both enums to the document under a single
name, and the document — like the typed client reading it — would keep only one. An
`optional` column stays nullable: a `NULL` passes a `CHECK`. A field named `response` or
`filter` is refused before anything is written: its type would then be named like the DTO
or the filter that the CRUD already declares, in the very file that imports it from the
model. Such a column is filtered by `eq`, `in` and `is_null` rather than by the comparisons
of an ordered one; [Filtering](../guides/filtering.md) has that table.

The eleventh, `references`, is not a scalar at all: it points the column at another entity
instead of giving it a type of its own.

```text
author:references:users
```

The name declared is the *relation*'s, `author`; the column is derived from it, `author_id`
— which is what lets the SeaORM variant, the foreign key and the DTO field agree on a name
without anyone repeating it. The third segment is the target table, as it exists in the
project; a table the CLI cannot find is refused, by name, alongside the ones it does know.
What a reference writes on both ends of the relation, its own two modifiers, and the shape
of its refusals belong to [Relations](../guides/relations.md), not to this page.

### The six modifiers

| Modifier | Effect |
|---|---|
| `unique` | Unique constraint on the column — on a reference, this is what makes the relation one-to-one. |
| `optional` | The column is nullable and the Rust type becomes `Option<T>`. |
| `index` | Plain index on the column. |
| `max=<n>` | Textual field only. Length bound in the generated DTOs, overriding the default. |
| `cascade` | Reference only. `ON DELETE CASCADE`. |
| `nullify` | Reference only. `ON DELETE SET NULL` — requires `optional`. |

Their order is free and each may appear at most once. `unique` and `index` together are
refused as redundant — a unique constraint already lays down an index — and so is `index`
alone on a reference, whose foreign key is indexed without being asked. `cascade` and
`nullify` contradict each other and are refused together; [Relations](../guides/relations.md)
has the rest of a reference's grammar, including why the index is never optional.

Neither `unique` nor `index` applies to a `text` field: MySQL refuses an index on a `TEXT`
column without a prefix length (error 1170). The refusal holds on every engine, PostgreSQL
included — a generated migration is meant to run anywhere, and one rule is one rule. An
indexed column of text is a `string`, that is a `varchar(255)`.

### What a name may be

A field name is `snake_case`: it starts with an ASCII lowercase letter and holds only
lowercase letters, digits and underscores, with no trailing underscore. Four families of
name are refused outright, because each would produce a project that does not compile or a
schema that is wrong:

- Rust's 51 strict and reserved keywords, from the 2015 to the 2024 editions — rustc would
  have said so forty seconds later.
- `id`, `created_at` and `updated_at`, which rbs lays down on every entity.
- `table`, which collides with the `Table` variant `DeriveIden` reserves for the table name
  in the migration.
- A name already declared earlier in the same `--fields`.

A field named `email`, or ending in `_email`, and typed `string` or `text` gets an email
constraint in the generated DTOs. That is deduced from the name because the name is the
only thing available.

### The length bound

A `string` field is bounded at 255 characters in the generated DTOs, without anyone asking:
`#[validate(length(max = 255))]`. Nothing else bounds it — `ColumnDef::string()` renders a
`varchar` with no length on PostgreSQL — so without that line every public route of a
generated project accepts a string of arbitrary length. The value is the one of the
traditional `varchar(255)`, wide enough for a name, a title or an address.

`text` is the type one picks *to* exceed that bound, so it gets none by default. Write
`max=<n>` to set a bound on either type, or to widen or narrow the default one:

```bash
rbs generate crud articles --fields "title:string:max=200,summary:text:max=5000"
```

`max=` bounds a length of text, and is refused on any other type; `<n>` is a strictly
positive integer. A constraint written by hand in the generated DTO survives, like every
other edit: this code is meant to be modified.

### Errors

Every fault on the line is collected in one pass, so the line gets fixed in one go rather
than one fault per run. A field carrying two faults reports only the first.

```text
$ rbs generate crud tags --fields "Title:string,type:text,prix:money,slug:string:unique:index,email:string,email:int" --dry-run
erreur : champ 1 « Title » — le nom doit être en snake_case : minuscules ASCII, chiffres et souligné
        → essayez « title »
erreur : champ 2 « type » — « type » est un mot-clé Rust
        → essayez « kind » ou « type_ »
erreur : champ 3 « prix » — type inconnu « money »
        → string, int, float, decimal, bool, uuid, datetime, date, text, references:<table>, enum(a,b,c)
erreur : champ 4 « slug » — « index » redondant : « unique » pose déjà un index
        → retirez « index »
erreur : champ 6 « email » — « email » est déjà déclaré au champ 5
        → un nom de champ ne peut apparaître qu'une fois
```

Note the rank of the duplicate: field 6 is reported against field 5, and field 5 itself is
accepted.

```text
$ rbs generate crud tags --fields "id:string,table:string,bio:text:optional:optional" --dry-run
erreur : champ 1 « id » — « id » ne se déclare pas
        → id, created_at et updated_at sont posés sur toute entité
erreur : champ 2 « table » — « table » entrerait en collision avec l'identifiant de la table dans la migration
        → essayez « table_ »
erreur : champ 3 « bio » — modificateur « optional » en double
```

A field with no type — or a stray separator, such as a trailing comma or `email:string:` —
is a shape error rather than an unknown type:

```text
$ rbs generate crud tags --fields "titre" --dry-run
erreur : champ 1 « titre » — forme attendue : « nom:type[:modificateur…] »
        → exemple : « email:string:unique »
```

## The plan

Every run prints its plan before writing anything — what the command is about to do should
not be discovered afterwards. `--dry-run` stops there.

```text
$ rbs generate crud articles --fields "title:string,body:text,slug:string:unique,published:bool,views:int:optional" --dry-run
plan pour /private/tmp/rbs-demo/blog

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests/mod.rs                           créé
  + src/articles/tests/lifecycle.rs                     créé
  + src/articles/tests/errors.rs                        créé
  + src/articles/tests/filter.rs                        créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260830_110925_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  13 à créer, 7 à modifier

  rien n'a été écrit (--dry-run)
```

The same command without `--dry-run` prints the same plan, then applies it:

```text
$ rbs generate crud articles --fields "title:string,body:text,slug:string:unique,published:bool,views:int:optional"
plan pour /private/tmp/rbs-demo/blog

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests/mod.rs                           créé
  + src/articles/tests/lifecycle.rs                     créé
  + src/articles/tests/errors.rs                        créé
  + src/articles/tests/filter.rs                        créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260830_110925_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  13 à créer, 7 à modifier
✓ articles générée — 13 créés, 7 modifiés

  la migration m20260830_110925_create_articles reste à appliquer avant de lancer le projet
```

Thirteen files created, seven modified through their anchors. The feature is then recorded in
the manifest, which is what makes the command idempotent:

```text
[package.metadata.rbs]
version = "1.2.0"
features = ["health", "articles"]
database = "postgres"
```

Markers in the plan read: `+` created, `~` modified, `·` unchanged, `!` conflicting.

`rbs generate feature` writes six files and no migration:

```text
$ rbs generate feature comments --force
plan pour /private/tmp/rbs-demo/blog

  + src/comments/mod.rs          créé
  + src/comments/model.rs        créé
  + src/comments/dto.rs          créé
  + src/comments/repository.rs   créé
  + src/comments/service.rs      créé
  + src/comments/controller.rs   créé
  ~ src/lib.rs                   modifié
  ~ src/router.rs                modifié
  ~ src/openapi.rs               modifié
  ~ Cargo.toml                   modifié
  ~ AGENTS.md                    modifié

  6 à créer, 5 à modifier
✓ comments générée — 6 créés, 5 modifiés
```

## A dirty working tree

The generated files are new, but the insertions are edits to files you already have. So
`rbs generate` refuses to run over uncommitted changes — including under `--dry-run`, since
the check happens while planning:

```text
$ rbs generate feature comments
erreur : le working tree n'est pas propre : Cargo.toml, src/lib.rs, src/openapi.rs, src/router.rs — commitez, ou relancez avec --force
```

Untracked files are not counted: they are exactly what the command is about to create. Past
five names the list is abbreviated. `--force` runs anyway, which is what the message
suggests and what the run above used.

## Anchors

`rbs generate` never rewrites an AST. It inserts between comment markers the skeleton
carries. `rbs generate crud` and `rbs generate feature` use six of the eighteen — the two in
`src/state.rs`, `// <rbs:layers>` and `// <rbs:startup>` belong to the fragments
[`rbs add`](./add.md) installs:

| Anchor | File |
|---|---|
| `// <rbs:features>` | `src/lib.rs` |
| `// <rbs:routes>` | `src/router.rs` |
| `// <rbs:openapi>` | `src/openapi.rs` |
| `// <rbs:migration_modules>` | `migration/src/lib.rs` |
| `// <rbs:migrations>` | `migration/src/lib.rs` |
| `// <rbs:seeds>` | `src/seeds/main.rs` |

`rbs generate job` uses three, none shared with the two commands above and none carried by
the skeleton either — each lives in a file a fragment deposits, and `// <rbs:jobs>` also
receives the `webhooks` fragment's delivery:

| Anchor | File |
|---|---|
| `// <rbs:job_modules>` | `src/modules/jobs/mod.rs`, deposited by `jobs` |
| `// <rbs:jobs>` | `src/modules/jobs/mod.rs`, deposited by `jobs` |
| `// <rbs:schedules>` | `src/modules/scheduler/mod.rs`, deposited by `scheduler`, under `--every` |

`src/lib.rs` is the library every generated project carries: `src/main.rs` and
`src/seeds/main.rs` are two separate crate roots, and the library is what lets both reach a
feature's modules — models included, now that a relation can name one from another. A
project generated before this library existed has none, and on it `// <rbs:features>`
stays where it always lived, in `src/main.rs` — `rbs generate` and `rbs doctor` resolve the
anchor to whichever file is actually present, so an older project keeps working unchanged.

Remove one and the command writes nothing at all — not the feature files either — and
prints the block to paste back:

```text
$ rbs generate feature notes --force
erreur : ancre // <rbs:routes> introuvable dans src/router.rs

dans src/router.rs :
// <rbs:routes>
// </rbs:routes>
```

[`rbs doctor`](./doctor.md) checks all eighteen anchors — twelve on a project carrying no
compose, no queue and no fragment moved under `src/modules/`, six of the seven optional ones —
so a missing one can be found before a generation trips over it.

## Failures

A feature that is already there is refused rather than merged:

```text
$ rbs generate crud articles --fields "title:string"
erreur : src/articles existe déjà : la feature `articles` est déjà là
```

Outside a project:

```text
$ rbs generate crud users --dry-run
erreur : aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`
```

Both exit with status 2: the call is what needs correcting — another name, or the right
directory. See [exit codes](./doctor.md#exit-codes).
