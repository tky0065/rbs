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

```text
$ rbs generate --help
Génère une feature dans un projet existant

Usage: rbs generate [OPTIONS] <COMMAND>

Commands:
  crud     Génère une feature CRUD complète, entité et migration comprises
  feature  Génère une feature vide : six fichiers, aucun champ
  help     Print this message or the help of the given subcommand(s)

Options:
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

`g` is an alias for `generate`: `rbs g crud users` and `rbs generate crud users` parse to
the same thing.

The two global options are accepted here because clap propagates them, but neither changes
anything: `rbs generate` asks no questions, and its templates are compiled into the binary
rather than read from `--template-dir`.

## `rbs generate crud`

```text
$ rbs generate crud --help
Génère une feature CRUD complète, entité et migration comprises

Usage: rbs generate crud [OPTIONS] <NOM>

Arguments:
  <NOM>  Nom de la feature, au pluriel

Options:
      --fields <CHAMPS>        Champs de l'entité, ex. "name:string,email:string:unique"
      --force                  Écrit même si le working tree Git est sale
      --dry-run                Affiche le plan sans rien écrire
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

| Flag | Effect |
|---|---|
| `--fields <CHAMPS>` | The entity's columns, in the grammar below. Omitted, the feature is generated with no column of its own. |
| `--force` | Writes even though the Git working tree is dirty, and overwrites files reported as conflicting. |
| `--dry-run` | Prints the plan and stops. Nothing is written. |

## `rbs generate feature`

```text
$ rbs generate feature --help
Génère une feature vide : six fichiers, aucun champ

Usage: rbs generate feature [OPTIONS] <NOM>

Arguments:
  <NOM>  Nom de la feature

Options:
      --force                  Écrit même si le working tree Git est sale
      --dry-run                Affiche le plan sans rien écrire
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

Same flags minus `--fields`: an empty feature has no columns, so it gets neither an entity
worth the name nor a migration.

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

### The seven types

There is no eighth, and no `email` type: a string format is not a column type.

| Type | Rust | Migration |
|---|---|---|
| `string` | `String` | `string()` |
| `text` | `String` | `text()` |
| `int` | `i32` | `integer()` |
| `float` | `f64` | `double()` |
| `bool` | `bool` | `boolean()` |
| `uuid` | `Uuid` | `uuid()` |
| `datetime` | `DateTimeWithTimeZone` | `timestamp_with_time_zone()` |

`string` and `text` share a Rust type, so `text` is the only one that also carries an
explicit column type on the entity — without it SeaORM would infer `varchar`.

### The three modifiers

| Modifier | Effect |
|---|---|
| `unique` | Unique constraint on the column. |
| `optional` | The column is nullable and the Rust type becomes `Option<T>`. |
| `index` | Plain index on the column. |

Their order is free and each may appear at most once. `unique` and `index` together are
refused as redundant — a unique constraint already lays down an index.

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

### Errors

Every fault on the line is collected in one pass, so the line gets fixed in one go rather
than one fault per run. A field carrying two faults reports only the first.

```text
$ rbs generate crud tags --fields "Title:string,type:text,prix:decimal,slug:string:unique:index,email:string,email:int" --dry-run
erreur : erreur : champ 1 « Title » — le nom doit être en snake_case : minuscules ASCII, chiffres et souligné
        → essayez « title »
erreur : champ 2 « type » — « type » est un mot-clé Rust
        → essayez « kind » ou « type_ »
erreur : champ 3 « prix » — type inconnu « decimal »
        → string, int, float, bool, uuid, datetime, text
erreur : champ 4 « slug » — « index » redondant : « unique » pose déjà un index
        → retirez « index »
erreur : champ 6 « email » — « email » est déjà déclaré au champ 5
        → un nom de champ ne peut apparaître qu'une fois
```

Note the rank of the duplicate: field 6 is reported against field 5, and field 5 itself is
accepted.

```text
$ rbs generate crud tags --fields "id:string,table:string,bio:text:optional:optional" --dry-run
erreur : erreur : champ 1 « id » — « id » ne se déclare pas
        → id, created_at et updated_at sont posés sur toute entité
erreur : champ 2 « table » — « table » entrerait en collision avec l'identifiant de la table dans la migration
        → essayez « table_ »
erreur : champ 3 « bio » — modificateur « optional » en double
```

A field with no type — or a stray separator, such as a trailing comma or `email:string:` —
is a shape error rather than an unknown type:

```text
$ rbs generate crud tags --fields "titre" --dry-run
erreur : erreur : champ 1 « titre » — forme attendue : « nom:type[:modificateur…] »
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
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + migration/src/m20260826_213608_create_articles.rs   créé
  ~ src/main.rs                                         modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ Cargo.toml                                          modifié

  13 fichiers à écrire

  rien n'a été écrit (--dry-run)
```

The same command without `--dry-run` prints the same plan, then applies it:

```text
$ rbs generate crud articles --fields "title:string,body:text,slug:string:unique,published:bool,views:int:optional"
plan pour /private/tmp/rbs-demo/blog

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + migration/src/m20260826_213608_create_articles.rs   créé
  ~ src/main.rs                                         modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ Cargo.toml                                          modifié

  13 fichiers à écrire
✓ articles générée — 8 fichiers

  la migration m20260826_213608_create_articles reste à appliquer avant de lancer le projet
```

Eight files created, five modified through their anchors. The feature is then recorded in
the manifest, which is what makes the command idempotent:

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health", "articles"]
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
  ~ src/main.rs                  modifié
  ~ src/router.rs                modifié
  ~ src/openapi.rs               modifié
  ~ Cargo.toml                   modifié

  10 fichiers à écrire
✓ comments générée — 6 fichiers
```

## A dirty working tree

The generated files are new, but the insertions are edits to files you already have. So
`rbs generate` refuses to run over uncommitted changes — including under `--dry-run`, since
the check happens while planning:

```text
$ rbs generate feature comments
erreur : le working tree n'est pas propre : Cargo.toml, migration/src/lib.rs, src/main.rs, src/openapi.rs, src/router.rs — commitez, ou relancez avec --force
```

Untracked files are not counted: they are exactly what the command is about to create. Past
five names the list is abbreviated. `--force` runs anyway, which is what the message
suggests and what the run above used.

## Anchors

`rbs generate` never rewrites an AST. It inserts between comment markers the skeleton
carries, and there are five of them:

| Anchor | File |
|---|---|
| `// <rbs:features>` | `src/main.rs` |
| `// <rbs:routes>` | `src/router.rs` |
| `// <rbs:openapi>` | `src/openapi.rs` |
| `// <rbs:migration_modules>` | `migration/src/lib.rs` |
| `// <rbs:migrations>` | `migration/src/lib.rs` |

Remove one and the command writes nothing at all — not the feature files either — and
prints the block to paste back:

```text
$ rbs generate feature notes --force
erreur : ancre // <rbs:routes> introuvable dans src/router.rs

dans src/router.rs :
// <rbs:routes>
// </rbs:routes>
```

[`rbs doctor`](./doctor.md) checks the five anchors, so a missing one can be found before a
generation trips over it.

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

Each of these exits with status 1.
