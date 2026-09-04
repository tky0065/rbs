# Filtres et tri dans le CRUD engendré — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** faire que `rbs generate crud` engendre une route `POST /<module>/filter` qui
restreigne et trie la liste depuis un corps JSON typé par les colonnes de `--fields`.

**Architecture:** `rbs-core` reçoit les opérateurs, qui ne connaissent aucune colonne ;
le CLI engendre par feature un `filter.rs` qui les compose en un type nommé d'après
l'entité et les traduit en conditions SeaORM. `repository.rs` est le seul client de ce
fichier — la règle « seul le repository construit une requête » ne bouge pas.

**Tech Stack:** Rust, axum 0.8, SeaORM 2.0, serde, utoipa 5.5, minijinja (délimiteurs
alternatifs `{@ @}`), `include_dir`.

**Spec:** `docs/superpowers/specs/2026-09-01-filtres-crud-design.md`

## Global Constraints

- `rbs-core` porte `#![warn(missing_docs)]` : tout item public reçoit un `///` d'une à
  trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*. Le code engendré ne commente
  que ses points d'extension.
- Les templates minijinja utilisent les délimiteurs `{@ variable @}` et `{% bloc %}` :
  `{{ }}` est laissé à `format!`.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`
  sont bloquants ; le rendu des templates doit être *déjà* ce que rustfmt écrirait.
- Documentation bilingue : toute page modifiée en anglais l'est aussi en français, dans le
  même commit.
- Commits en Conventional Commits, sujet en français à l'impératif, sans identifiant de
  tâche, sans renvoi à un fichier de suivi, sans `Co-Authored-By`.
- Travailler sur une branche dédiée, jamais sur `main`.
- `rbs-core` reste en `1.2.0` : l'ajout est additif et la version n'est pas publiée.

---

### Task 1 : les opérateurs du noyau

**Files:**
- Create: `crates/rbs-core/src/filter.rs`
- Modify: `crates/rbs-core/src/lib.rs` (déclarer `pub mod filter;` après `pub mod extract;`
  et réexporter dans le bloc `pub use`)
- Test: `crates/rbs-core/src/filter.rs` (`mod tests` en fin de fichier, comme
  `pagination.rs`)

**Interfaces:**
- Consumes: rien.
- Produces:
  - `pub struct Comparison<T> { pub eq: Option<T>, pub gt: Option<T>, pub gte: Option<T>, pub lt: Option<T>, pub lte: Option<T>, pub is_null: Option<bool> }`
  - `pub struct TextMatch { pub eq: Option<String>, pub contains: Option<String>, pub is_null: Option<bool> }`
  - `pub struct Sort(Vec<SortKey>)` avec `pub struct SortKey { pub column: String, pub descending: bool }`
    et `pub fn keys(&self) -> &[SortKey]`
  - réexports : `pub use filter::{Comparison, Sort, SortKey, TextMatch};`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-core/src/filter.rs`, en fin de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// La forme courte est ce qu'un client écrit dans le cas courant : `"published": true`
    /// doit valoir `{ "eq": true }`, sans quoi le corps le plus fréquent serait le plus
    /// verbeux.
    #[test]
    fn a_bare_value_reads_as_an_equality() {
        let compare: Comparison<bool> = serde_json::from_str("true").expect("valeur nue lisible");

        assert_eq!(compare.eq, Some(true));
        assert_eq!(compare.gt, None);
    }

    #[test]
    fn an_object_names_its_operators() {
        let compare: Comparison<i32> =
            serde_json::from_str(r#"{"gte": 10, "lt": 100}"#).expect("objet lisible");

        assert_eq!(compare.gte, Some(10));
        assert_eq!(compare.lt, Some(100));
        assert_eq!(compare.eq, None);
    }

    #[test]
    fn a_bare_string_reads_as_an_equality_on_text() {
        let match_: TextMatch = serde_json::from_str(r#""rust""#).expect("chaîne nue lisible");

        assert_eq!(match_.eq.as_deref(), Some("rust"));
        assert_eq!(match_.contains, None);
    }

    /// Le préfixe `-` est la seule syntaxe de tri : elle tient dans une chaîne JSON, se
    /// lit sans documentation, et ne demande pas d'objet par colonne.
    #[test]
    fn the_minus_prefix_marks_a_descending_column() {
        let sort: Sort = serde_json::from_str(r#"["-views", "title"]"#).expect("tri lisible");

        assert_eq!(sort.keys().len(), 2);
        assert_eq!(sort.keys()[0].column, "views");
        assert!(sort.keys()[0].descending);
        assert_eq!(sort.keys()[1].column, "title");
        assert!(!sort.keys()[1].descending);
    }

    /// `Sort` ne connaît aucune colonne : c'est le `filter.rs` engendré qui refuse un nom
    /// inconnu, en les nommant tous. Ici, seule la syntaxe du préfixe est analysée.
    #[test]
    fn sort_keeps_a_column_it_knows_nothing_about() {
        let sort: Sort = serde_json::from_str(r#"["-inconnue"]"#).expect("tri lisible");

        assert_eq!(sort.keys()[0].column, "inconnue");
    }
}
```

- [ ] **Step 2: Lancer les tests et voir l'échec**

Run: `cargo test -p rbs-core filter`
Expected: FAIL — `cannot find type Comparison in this scope` (le module n'existe pas).

- [ ] **Step 3: Écrire le module**

Dans `crates/rbs-core/src/filter.rs` :

```rust
//! Opérateurs de filtrage et de tri, sans connaissance des colonnes.
//!
//! Les types de ce module ne nomment aucune colonne et n'en valident aucune : c'est le
//! `filter.rs` que le CLI engendre par feature qui les compose en un type dont chaque
//! champ vient de `--fields`, et qui seul sait traduire un nom en `Column`.

use serde::{Deserialize, Deserializer};
use utoipa::ToSchema;

/// Conditions portées sur une colonne scalaire ordonnée.
///
/// Se lit d'une valeur nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct Comparison<T> {
    /// Égalité stricte.
    pub eq: Option<T>,
    /// Strictement supérieur.
    pub gt: Option<T>,
    /// Supérieur ou égal.
    pub gte: Option<T>,
    /// Strictement inférieur.
    pub lt: Option<T>,
    /// Inférieur ou égal.
    pub lte: Option<T>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions portées sur une colonne textuelle.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct TextMatch {
    /// Égalité stricte.
    pub eq: Option<String>,
    /// Sous-chaîne, insensible à la casse.
    pub contains: Option<String>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Une colonne de tri et son sens.
#[derive(Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct SortKey {
    /// Nom de la colonne, préfixe retiré.
    pub column: String,
    /// Vrai quand le nom portait `-`.
    pub descending: bool,
}

/// Colonnes de tri, dans l'ordre où le client les a demandées.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct Sort(Vec<SortKey>);

impl Sort {
    /// Les colonnes demandées, dans l'ordre.
    pub fn keys(&self) -> &[SortKey] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Sort {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let colonnes = Vec::<String>::deserialize(deserializer)?;

        Ok(Self(
            colonnes
                .into_iter()
                .map(|colonne| match colonne.strip_prefix('-') {
                    Some(reste) => SortKey {
                        column: reste.to_owned(),
                        descending: true,
                    },
                    None => SortKey {
                        column: colonne,
                        descending: false,
                    },
                })
                .collect(),
        ))
    }
}
```

Les deux formes de `Comparison<T>` et de `TextMatch` passent par un `Deserialize` manuel
sur une énumération `#[serde(untagged)]` privée — `Default` ne suffit pas, et un
`#[serde(untagged)]` posé sur le type public exposerait la variante dans l'OpenAPI :

```rust
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOf<T, O> {
    Bare(T),
    Operators(O),
}

#[derive(Deserialize)]
struct ComparisonOperators<T> {
    eq: Option<T>,
    gt: Option<T>,
    gte: Option<T>,
    lt: Option<T>,
    lte: Option<T>,
    is_null: Option<bool>,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Comparison<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match OneOf::<T, ComparisonOperators<T>>::deserialize(deserializer)? {
                OneOf::Bare(valeur) => Self {
                    eq: Some(valeur),
                    ..Self::default()
                },
                OneOf::Operators(operateurs) => Self {
                    eq: operateurs.eq,
                    gt: operateurs.gt,
                    gte: operateurs.gte,
                    lt: operateurs.lt,
                    lte: operateurs.lte,
                    is_null: operateurs.is_null,
                },
            },
        )
    }
}
```

`TextMatch` suit le même moule, avec `OneOf::<String, TextMatchOperators>`.

Dans `crates/rbs-core/src/lib.rs`, après `pub mod extract;` :

```rust
/// Opérateurs de filtrage et de tri des listes.
pub mod filter;
```

et dans le bloc de réexports, en ordre alphabétique après `pub use extract::ValidatedJson;` :

```rust
pub use filter::{Comparison, Sort, SortKey, TextMatch};
```

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-core filter`
Expected: PASS, 5 tests.

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy -p rbs-core --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie, exit 0. `missing_docs` est actif sur cette crate : tout item
public sans `///` échoue ici.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-core/src/filter.rs crates/rbs-core/src/lib.rs
git commit -m "feat(core): ajoute les opérateurs de filtrage et de tri"
```

---

### Task 2 : le `filter.rs` engendré, sans la requête

Le type et son refus d'une colonne de tri inconnue. La traduction en conditions SeaORM
vient à la tâche 3 : un reviewer peut accepter le type et refuser la traduction.

**Files:**
- Create: `crates/rbs-cli/templates/feature/filter.rs.jinja`
- Create: `crates/rbs-cli/src/generate/filter.rs`
- Modify: `crates/rbs-cli/src/generate/mod.rs` (déclarer `pub(crate) mod filter;`)
- Test: `crates/rbs-cli/src/generate/filter.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Comparison`, `TextMatch`, `Sort`, `SortKey` de la tâche 1 ; `Feature` et
  `Field` de `crate::generate::{feature, fields}`.
- Produces: `pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error>`
  — le contenu de `src/<module>/filter.rs`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/filter.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::fields;

    const CHAMPS: &str = "title:string,body:text:optional,views:int,published:bool,\
                          author_id:uuid,published_at:datetime";

    fn filtre(name: &str, champs: &str) -> String {
        let fields = fields::parse(champs).expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le filtre doit se rendre")
    }

    /// Chaque champ de `--fields` devient un champ du filtre, avec l'opérateur de son
    /// type : un texte se cherche par sous-chaîne, un nombre se compare.
    #[test]
    fn each_field_earns_the_operator_of_its_type() {
        let rendered = filtre("articles", CHAMPS);

        for champ in [
            "pub title: Option<TextMatch>,",
            "pub body: Option<TextMatch>,",
            "pub views: Option<Comparison<i32>>,",
            "pub published: Option<Comparison<bool>>,",
            "pub author_id: Option<Comparison<Uuid>>,",
            "pub published_at: Option<Comparison<DateTimeUtc>>,",
        ] {
            assert!(rendered.contains(champ), "« {champ} » absent :\n{rendered}");
        }
    }

    /// Les trois colonnes que toute entité porte sont filtrables sans figurer dans
    /// `--fields` : elles existent dans chaque modèle engendré.
    #[test]
    fn the_three_columns_of_every_entity_are_filterable() {
        let rendered = filtre("articles", CHAMPS);

        for champ in [
            "pub id: Option<Comparison<Uuid>>,",
            "pub created_at: Option<Comparison<DateTimeUtc>>,",
            "pub updated_at: Option<Comparison<DateTimeUtc>>,",
        ] {
            assert!(rendered.contains(champ), "« {champ} » absent :\n{rendered}");
        }
    }

    /// Aucun nom de colonne ne vient de la requête : le tri passe par un `match` écrit à
    /// la génération, et un nom inconnu est refusé en nommant ceux qui sont acceptés.
    #[test]
    fn an_unknown_sort_column_is_refused_by_name() {
        let rendered = filtre("articles", CHAMPS);

        assert!(
            rendered.contains("fn column_of(name: &str) -> Result<Column>"),
            "la traduction du nom de colonne est absente :\n{rendered}"
        );
        assert!(
            rendered.contains(r#""title" => Column::Title,"#),
            "la colonne connue doit se traduire :\n{rendered}"
        );
        assert!(
            rendered.contains("Error::BadRequest"),
            "un nom inconnu doit rendre 400 :\n{rendered}"
        );
        assert!(
            rendered.contains("id, created_at, updated_at, title, body, views, published, author_id, published_at"),
            "le refus doit nommer les colonnes acceptées :\n{rendered}"
        );
    }

    /// Le coût n'est pas caché : filtrer une colonne sans index parcourt la table, et le
    /// fichier est fait pour être lu.
    #[test]
    fn the_cost_of_an_unindexed_column_is_written_down() {
        let rendered = filtre("articles", CHAMPS);

        assert!(
            rendered.contains("parcourt la table"),
            "le coût doit être énoncé :\n{rendered}"
        );
    }
}
```

- [ ] **Step 2: Lancer les tests et voir l'échec**

Run: `cargo test -p rbs-cli --lib generate::filter`
Expected: FAIL — le module `generate::filter` n'existe pas.

- [ ] **Step 3: Écrire la template et le générateur**

`crates/rbs-cli/templates/feature/filter.rs.jinja` :

```jinja
use rbs_core::{Comparison, Error, Result, Sort, TextMatch};
use sea_orm::prelude::{DateTimeUtc, Uuid};

use super::model::Column;

// Toute colonne est filtrable, indexée ou non : un filtre sur une colonne sans index
// parcourt la table. Ajoutez « index » au champ dans `--fields` si la table grandit.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct {@ entity @}Filter {
    pub id: Option<Comparison<Uuid>>,
    pub created_at: Option<Comparison<DateTimeUtc>>,
    pub updated_at: Option<Comparison<DateTimeUtc>>,
{%- for field in fields %}
    pub {@ field.name @}: Option<{@ field.operator @}>,
{%- endfor %}
    pub sort: Option<Sort>,
}

/// Traduit un nom de colonne reçu du client en `Column`.
///
/// Le `match` est écrit à la génération : aucun nom venu de la requête n'atteint la base,
/// et un nom inconnu est refusé en nommant ceux qui sont acceptés.
fn column_of(name: &str) -> Result<Column> {
    Ok(match name {
        "id" => Column::Id,
        "created_at" => Column::CreatedAt,
        "updated_at" => Column::UpdatedAt,
{%- for field in fields %}
        "{@ field.name @}" => Column::{@ field.pascal_name @},
{%- endfor %}
        inconnue => {
            return Err(Error::BadRequest(format!(
                "colonne de tri inconnue « {inconnue} » — {@ colonnes @}"
            )));
        }
    })
}
```

`crates/rbs-cli/src/generate/filter.rs` :

```rust
//! Rendu de `<name>/filter.rs` : le filtre typé par les colonnes de `--fields`.

use minijinja::context;
use serde::Serialize;

use crate::template::Renderer;

use super::feature::Feature;
use super::fields::{Field, FieldType};

const FILTER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/filter.rs.jinja"
));

/// Un champ vu par le filtre : son nom, sa variante de `Column`, son opérateur.
#[derive(Serialize)]
struct FilterField {
    name: String,
    pascal_name: String,
    operator: String,
}

/// Les trois colonnes que toute entité engendrée porte, filtrables sans `--fields`.
const COLONNES_DE_BASE: [&str; 3] = ["id", "created_at", "updated_at"];

/// Rend le filtre de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    let fields: Vec<FilterField> = feature.fields.iter().map(champ).collect();
    let colonnes = COLONNES_DE_BASE
        .iter()
        .map(|nom| (*nom).to_owned())
        .chain(feature.fields.iter().map(Field::column_name))
        .collect::<Vec<_>>()
        .join(", ");

    Renderer::new().render(
        FILTER,
        context! {
            entity => feature.entity(),
            fields => fields,
            colonnes => colonnes,
        },
    )
}

fn champ(field: &Field) -> FilterField {
    FilterField {
        name: field.column_name(),
        pascal_name: field.pascal_name(),
        operator: operator(field),
    }
}

/// Un texte se cherche par sous-chaîne, tout le reste se compare.
fn operator(field: &Field) -> String {
    if field.reference().is_none()
        && matches!(field.column_type(), FieldType::String | FieldType::Text)
    {
        return "TextMatch".to_owned();
    }

    format!("Comparison<{}>", scalar_type(field))
}
```

`scalar_type` rend le type Rust **sans** l'`Option` que `Field::rust_type()` ajoute pour un
champ `optional` : le filtre porte déjà le sien, et `Comparison<Option<i32>>` ne se compare
pas.

```rust
/// Le type comparé, sans l'`Option` d'un champ `optional` : le filtre porte déjà la
/// sienne, et une comparaison sur `Option<T>` n'aurait pas de sens.
fn scalar_type(field: &Field) -> &'static str {
    if field.reference().is_some() {
        return "Uuid";
    }

    match field.column_type() {
        FieldType::Datetime => "DateTimeUtc",
        FieldType::Uuid => "Uuid",
        autre => autre.rust_type(),
    }
}
```

Déclarer le module dans `crates/rbs-cli/src/generate/mod.rs`, en ordre alphabétique :

```rust
pub(crate) mod filter;
```

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli --lib generate::filter`
Expected: PASS, 4 tests.

- [ ] **Step 5: Vérifier que le rendu est déjà ce que rustfmt écrirait**

Ajouter au `mod tests` le contrôle que portent les autres générateurs
(`controller.rs:313`, `the_guarded_render_is_already_what_rustfmt_would_write`) : rendre,
passer par `crate::format::format_batch`, comparer sans diff.

Run: `cargo test -p rbs-cli --lib generate::filter`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature/filter.rs.jinja crates/rbs-cli/src/generate/filter.rs crates/rbs-cli/src/generate/mod.rs
git commit -m "feat(generate): engendre le type de filtre d'une feature"
```

---

### Task 3 : la traduction en conditions SeaORM

**Files:**
- Modify: `crates/rbs-cli/templates/feature/filter.rs.jinja` (ajouter `apply`)
- Test: `crates/rbs-cli/src/generate/filter.rs` (`mod tests`)

**Interfaces:**
- Consumes: le type `<Entity>Filter` et `column_of` de la tâche 2.
- Produces: dans le fichier engendré,
  `pub(super) fn apply(select: Select<Entity>, filtre: &<Entity>Filter) -> Result<Select<Entity>>`.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
    /// `apply` est le seul point où le filtre touche une requête, et il vit du côté du
    /// repository : le service et le controller ne font que transporter le type.
    #[test]
    fn the_filter_translates_into_seaorm_conditions() {
        let rendered = filtre("articles", CHAMPS);

        assert!(
            rendered.contains(
                "pub(super) fn apply(select: Select<Entity>, filtre: &ArticleFilter) \
                 -> Result<Select<Entity>>"
            ),
            "`apply` est absent :\n{rendered}"
        );
        for condition in [
            "Column::Views.gte(",
            "Column::Title.contains(",
            "Column::Title.is_null()",
            ".order_by_desc(",
            ".order_by_asc(",
        ] {
            assert!(
                rendered.contains(condition),
                "« {condition} » absent :\n{rendered}"
            );
        }
    }

    /// Sans `sort`, l'ordre reste celui de la liste : l'`id` est un UUIDv7, et c'est lui
    /// qui rend la pagination stable.
    #[test]
    fn the_default_order_stays_the_descending_id() {
        let rendered = filtre("articles", CHAMPS);

        assert!(
            rendered.contains("order_by_desc(Column::Id)"),
            "le tri par défaut doit rester `-id` :\n{rendered}"
        );
    }
```

- [ ] **Step 2: Lancer le test et voir l'échec**

Run: `cargo test -p rbs-cli --lib generate::filter`
Expected: FAIL — « `apply` est absent ».

- [ ] **Step 3: Écrire `apply` dans la template**

Ajouter à `filter.rs.jinja`, en complétant les `use` par
`use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, Select};` et
`use super::model::Entity;` :

```jinja
/// Applique le filtre à la requête de liste.
///
/// Le tri par défaut reste l'`id` décroissant : c'est un UUIDv7, et la pagination en
/// dépend.
pub(super) fn apply(select: Select<Entity>, filtre: &{@ entity @}Filter) -> Result<Select<Entity>> {
    let mut conditions = Condition::all();

    conditions = compare(conditions, Column::Id, filtre.id.as_ref());
    conditions = compare(conditions, Column::CreatedAt, filtre.created_at.as_ref());
    conditions = compare(conditions, Column::UpdatedAt, filtre.updated_at.as_ref());
{%- for field in fields %}
{%- if field.textual %}
    conditions = matches(conditions, Column::{@ field.pascal_name @}, filtre.{@ field.name @}.as_ref());
{%- else %}
    conditions = compare(conditions, Column::{@ field.pascal_name @}, filtre.{@ field.name @}.as_ref());
{%- endif %}
{%- endfor %}

    let select = select.filter(conditions);

    let Some(sort) = filtre.sort.as_ref().filter(|sort| !sort.keys().is_empty()) else {
        return Ok(select.order_by_desc(Column::Id));
    };

    sort.keys().iter().try_fold(select, |select, key| {
        let colonne = column_of(&key.column)?;

        Ok(match key.descending {
            true => select.order_by_desc(colonne),
            false => select.order_by_asc(colonne),
        })
    })
}
```

Les deux aides, écrites une fois et génériques sur la colonne — elles ne dépendent pas de
`--fields`, seuls leurs appels en dépendent :

```jinja
fn compare<T>(conditions: Condition, colonne: Column, compare: Option<&Comparison<T>>) -> Condition
where
    T: Into<sea_orm::Value> + Clone,
{
    let Some(compare) = compare else {
        return conditions;
    };

    let conditions = null_condition(conditions, colonne, compare.is_null);
    let conditions = add(conditions, compare.eq.clone(), |valeur| colonne.eq(valeur));
    let conditions = add(conditions, compare.gt.clone(), |valeur| colonne.gt(valeur));
    let conditions = add(conditions, compare.gte.clone(), |valeur| colonne.gte(valeur));
    let conditions = add(conditions, compare.lt.clone(), |valeur| colonne.lt(valeur));

    add(conditions, compare.lte.clone(), |valeur| colonne.lte(valeur))
}

fn matches(conditions: Condition, colonne: Column, match_: Option<&TextMatch>) -> Condition {
    let Some(match_) = match_ else {
        return conditions;
    };

    let conditions = null_condition(conditions, colonne, match_.is_null);
    let conditions = add(conditions, match_.eq.clone(), |valeur| colonne.eq(valeur));

    // `contains` de SeaORM rend un LIKE insensible à la casse et échappe la valeur.
    add(conditions, match_.contains.clone(), |valeur| {
        colonne.contains(valeur)
    })
}
```

Les deux dernières aides, elles aussi indépendantes de `--fields` :

```jinja
fn add<T>(
    conditions: Condition,
    valeur: Option<T>,
    condition: impl FnOnce(T) -> sea_orm::sea_query::SimpleExpr,
) -> Condition {
    match valeur {
        Some(valeur) => conditions.add(condition(valeur)),
        None => conditions,
    }
}

fn null_condition(conditions: Condition, colonne: Column, is_null: Option<bool>) -> Condition {
    match is_null {
        Some(true) => conditions.add(colonne.is_null()),
        Some(false) => conditions.add(colonne.is_not_null()),
        None => conditions,
    }
}
```

Le générateur gagne le drapeau `textual` sur `FilterField`, vrai pour un `TextMatch` :

```rust
#[derive(Serialize)]
struct FilterField {
    name: String,
    pascal_name: String,
    operator: String,
    textual: bool,
}
```

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli --lib generate::filter`
Expected: PASS, 6 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/templates/feature/filter.rs.jinja crates/rbs-cli/src/generate/filter.rs
git commit -m "feat(generate): traduit le filtre engendré en conditions SeaORM"
```

---

### Task 4 : le repository, le service et le controller

**Files:**
- Modify: `crates/rbs-cli/templates/feature/repository.rs.jinja` (`list` reçoit le filtre)
- Modify: `crates/rbs-cli/templates/feature/service.rs.jinja` (`filter` transporte)
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja` (le handler et sa
  déclaration OpenAPI)
- Modify: `crates/rbs-cli/templates/feature/mod.rs.jinja` (la route, et `pub mod filter;`)
- Modify: `crates/rbs-cli/src/generate/command.rs:490-497` (écrire le septième fichier)
- Test: les `mod tests` de `repository.rs`, `service.rs`, `controller.rs`, `command.rs`

**Interfaces:**
- Consumes: `filter::apply` et `<Entity>Filter` des tâches 2 et 3.
- Produces: dans le projet engendré,
  - `repository::filter(db, filtre: &<Entity>Filter, pagination) -> Result<(Vec<Model>, u64)>`
  - `service::filter(db, filtre, pagination) -> Result<Page<<Entity>Response>>`
  - `controller::filter` monté sur `POST /<module>/filter`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le `mod tests` de `crates/rbs-cli/src/generate/controller.rs` :

```rust
    /// La route littérale se monte avant `/{id}`, sans quoi `filter` serait lu comme un
    /// identifiant — c'est ce que fait déjà `broadcast` dans `examples/newsletter-queue`.
    #[test]
    fn the_filter_route_is_mounted_before_the_id_route() {
        let rendered = module("articles");

        let filtre = rendered.find("/articles/filter").expect("route de filtre montée");
        let id = rendered.find("/articles/{id}").expect("route d'identifiant montée");

        assert!(filtre < id, "`filter` doit précéder `{{id}}` :\n{rendered}");
    }

    /// Filtrer est une lecture : le garde de rôle ne la protège pas, comme il ne protège
    /// ni `list` ni `find`.
    #[test]
    fn the_filter_route_stays_open_under_a_role() {
        let rendered = guarded("articles", "Admin");

        assert!(
            !handler(&rendered, "filter").contains("require_role"),
            "filtrer est une lecture :\n{rendered}"
        );
    }
```

Dans le `mod tests` de `crates/rbs-cli/src/generate/repository.rs` :

```rust
    /// Un seul chemin de tri : `list` est le filtre vide. Deux `order_by` en dur
    /// divergeraient au premier changement, et la liste non filtrée est celle que
    /// personne ne pense à rejouer.
    #[test]
    fn the_list_is_the_empty_filter() {
        let rendered = repository("articles");

        assert!(
            rendered.contains("filter(db, &ArticleFilter::default(), pagination).await"),
            "`list` doit déléguer à `filter` :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("order_by_desc").count(),
            0,
            "le tri appartient désormais à `filter.rs` :\n{rendered}"
        );
    }
```

- [ ] **Step 2: Lancer les tests et voir l'échec**

Run: `cargo test -p rbs-cli --lib generate::`
Expected: FAIL sur les trois nouveaux tests.

- [ ] **Step 3: Modifier les quatre templates**

`repository.rs.jinja` — `list` devient un appel de `filter` avec un filtre vide, ce qui
laisse un seul chemin :

```jinja
pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {
    filter(db, &{@ entity @}Filter::default(), pagination).await
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &{@ entity @}Filter,
    pagination: &Pagination,
) -> Result<(Vec<Model>, u64)> {
    let requete = super::filter::apply(Entity::find(), filtre)?;

    let page = requete
        .clone()
        .offset(pagination.offset())
        .limit(pagination.per_page())
        .all(db);

    // Le total compte les lignes que le filtre retient, et non toute la table : les deux
    // requêtes partent ensemble, comme avant.
    let ({@ module @}, total) = tokio::try_join!(page, requete.count(db))?;

    Ok(({@ module @}, total))
}
```

`controller.rs.jinja` — le handler et sa déclaration :

```jinja
#[utoipa::path(
    post,
    path = "/{@ module @}/filter",
    tag = "{@ module @}",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = {@ entity @}Filter,
    responses(
        (status = 200, description = "page de {@ module @} filtrés", body = Page<{@ entity @}Response>),
        (status = 400, description = "filtre ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    pagination: Pagination,
    Json(filtre): Json<{@ entity @}Filter>,
) -> Result<Json<Page<{@ entity @}Response>>> {
    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}
```

`mod.rs.jinja` — `pub mod filter;` dans la liste des modules, et la route posée entre la
collection et l'identifiant, avec le commentaire qui dit pourquoi :

```jinja
        // Avant `/{@ module @}/{id}`, sans quoi `filter` serait lu comme un identifiant
        // et rendrait un 400 sur un chemin pourtant monté.
        .route("/{@ module @}/filter", post(controller::filter))
```

`command.rs` — ajouter la ligne au `vec!` des fichiers, après `dto.rs` :

```rust
        (dans("filter.rs"), filter::render(feature)),
```

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS. Les tests de `command.rs` qui énumèrent les fichiers engendrés
(`:778-781`) demandent d'ajouter `"filter.rs"` à leur liste ; les mettre à jour fait
partie de cette étape.

- [ ] **Step 5: Vérifier le rendu contre rustfmt et le lint**

Run: `cargo test -p rbs-cli --lib && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie, exit 0.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature crates/rbs-cli/src/generate
git commit -m "feat(generate): monte la route de filtrage du CRUD engendré"
```

---

### Task 5 : le document OpenAPI et le scénario de test engendré

**Files:**
- Modify: `crates/rbs-cli/src/generate/command.rs` (le bloc écrit à l'ancre
  `// <rbs:openapi>` : y déclarer `controller::filter` et le schéma `<Entity>Filter`)
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja`
- Modify: `crates/rbs-cli/src/generate/tests_http.rs`
- Test: `mod tests` de `command.rs` et de `tests_http.rs`

**Interfaces:**
- Consumes: `controller::filter` de la tâche 4.
- Produces: rien que les tâches suivantes consomment.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `tests_http.rs` :

```rust
    /// Le filtre se prouve par la route, et non par le SQL : une condition mal traduite
    /// rend une page vide, ce qu'aucun test de rendu ne verrait.
    #[test]
    fn a_filter_scenario_is_generated_when_a_field_can_carry_one() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("async fn the_filter_narrows_the_list()"),
            "le scénario de filtre est absent :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"request("POST", &format!("{collection}/filter")"#),
            "le scénario doit appeler la route de filtre :\n{rendered}"
        );
    }
```

Dans le `mod tests` de `crates/rbs-cli/src/generate/command.rs` :

```rust
    /// Une route absente du document est une route que le client engendré ne verra pas :
    /// l'ancre `<rbs:openapi>` doit recevoir le handler et le schéma du corps.
    #[test]
    fn the_filter_route_reaches_the_openapi_document() {
        let (root, _garde) = projet_avec_crud("articles");
        let openapi = read(&root.join("src/openapi.rs"));

        assert!(
            openapi.contains("articles::controller::filter"),
            "le handler doit être déclaré :\n{openapi}"
        );
        assert!(
            openapi.contains("ArticleFilter"),
            "le schéma du corps doit être déclaré :\n{openapi}"
        );
    }
```

`projet_avec_crud` est l'aide déjà présente dans ce `mod tests` ; reprendre le nom exact
qu'elle y porte.

- [ ] **Step 2: Lancer les tests et voir l'échec**

Run: `cargo test -p rbs-cli --lib`
Expected: FAIL sur les deux nouveaux tests.

- [ ] **Step 3: Écrire le scénario et compléter l'OpenAPI**

Dans `tests.rs.jinja`, sous `{%- if creatable %}` et derrière le drapeau `filterable` (vrai
dès qu'un champ est comparable), un scénario qui crée une ligne, la cherche par la valeur
qu'il vient d'envoyer, et vérifie qu'elle est dans la page — puis la supprime, comme ses
voisins :

```jinja
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let collection = "/{@ module @}";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let critere = json!({ "{@ filterable @}": sent["{@ filterable @}"] });
    let chemin = format!("{collection}/filter");
    let (status, page) = call(&api, request("POST", &chemin, critere)).await;
    assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

    let ids: Vec<&str> = page["data"]
        .as_array()
        .expect("la liste rend un tableau")
        .iter()
        .map(|ligne| ligne["id"].as_str().expect("identifiant rendu"))
        .collect();

    assert!(
        ids.contains(&created["id"].as_str().expect("identifiant rendu")),
        "la ligne créée doit satisfaire son propre critère : {page}"
    );

    let resource = format!("{collection}/{}", created["id"].as_str().unwrap_or_default());
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}
```

Le drapeau `filterable` porte le nom du premier champ comparable et non textuel du
scénario — un texte suffixé au hasard conviendrait aussi, mais un booléen ou un entier
donne un critère lisible.

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS.

- [ ] **Step 5: Prouver le scénario contre une vraie base**

Run: `cargo test -p rbs-cli --lib -- --ignored the_generated_tests_pass_untouched`
Expected: PASS. C'est le banc qui compile un projet et exécute ses tests contre un
PostgreSQL en conteneur : il exige qu'au moins un test tourne, et le scénario de filtre y
passe pour de bon. Docker est requis.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature/tests.rs.jinja crates/rbs-cli/src/generate
git commit -m "test(generate): éprouve la route de filtrage dans les tests engendrés"
```

---

### Task 6 : l'intégration de bout en bout

**Files:**
- Modify: `crates/rbs-cli/tests/integration_crud.rs`

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: rien.

- [ ] **Step 1: Écrire l'assertion qui échoue**

Étendre le test existant, après le `cargo test --workspace -- --include-ignored` déjà
lancé sur le projet engendré :

```rust
    assert!(
        joues.contains("the_filter_narrows_the_list ... ok"),
        "le scénario de filtrage n'a pas tourné — une route montée sans requête valide \
         laisserait ce test au vert :\n{joues}"
    );
```

- [ ] **Step 2: Lancer le test et voir l'échec**

Run: `cargo test -p rbs-cli --test integration_crud -- --ignored`
Expected: FAIL tant que la template n'engendre pas le scénario — ou PASS directement si
les tâches 1 à 5 sont déjà en place, auquel cas c'est le témoin qui compte.

- [ ] **Step 3: Lancer la suite Docker complète**

Run: `cargo test --workspace --no-fail-fast -- --ignored`
Expected: 0 échec. Ne pas passer la sortie dans un `tail` ou un `head` : le code de retour
serait celui du filtre, et non celui de cargo. Rediriger dans un fichier et le lire.

- [ ] **Step 4: Commit**

```bash
git add crates/rbs-cli/tests/integration_crud.rs
git commit -m "test(cli): exige le scénario de filtrage dans le projet engendré"
```

---

### Task 7 : les exemples et la documentation

**Files:**
- Modify: `examples/{hello-crud,blog-auth,file-drop,newsletter-queue}` (fichiers engendrés)
- Modify: `examples/README.md` (le septième fichier dans la liste des fichiers engendrés)
- Create: `docs/docs/guides/filtering.md` et `docs/i18n/fr/.../filtering.md`
- Modify: le `sidebars.js` de Docusaurus, dans les deux langues

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: rien.

- [ ] **Step 1: Régénérer les exemples par diff**

Régénérer les quatre projets dans un répertoire temporaire avec le CLI de la branche, puis
reporter **les seuls hunks** dus au filtre sur ce qui est versionné. Ne jamais écraser :
`examples/blog-auth/src/posts/*` et les neuf fichiers de `file-drop` portent des éditions
manuelles qu'`examples/README.md` énumère, et qu'aucune génération ne reproduit.

Run: `cargo test -p rbs-cli --test integration_examples`
Expected: PASS — c'est l'oracle de la régénération, il compare octet à octet.

- [ ] **Step 2: Écrire la page de documentation, dans les deux langues**

Elle décrit le corps, les opérateurs, la pagination en chaîne de requête et le coût d'une
colonne sans index. Tous les extraits sont tirés d'`examples/`, aucun écrit à la main.

- [ ] **Step 3: Vérifier la parité**

Run: `node docs/scripts/parite.mjs`
Expected: aucune page signalée. L'instrument ne voit ni les tableaux ni le dernier commit
des paires racine : relire soi-même les deux pages côte à côte.

- [ ] **Step 4: Commit**

```bash
git add examples docs
git commit -m "docs: décrit le filtrage du CRUD engendré et régénère les exemples"
```

---

### Task 8 : cocher et finir

- [ ] **Step 1: Cocher la tâche 57 dans `IMPROVE.md`**

`- [x]` sur la ligne 91, suivi de ` — Fait le YYYY-MM-DD : ` et des preuves exécutées,
avec leurs nombres réels.

- [ ] **Step 2: Vérification finale**

Run, en redirigeant chacune dans un fichier plutôt que dans un `tail` :

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --no-fail-fast -- --ignored
```

Expected: exit 0 partout, aucune sortie pour les deux premières.

- [ ] **Step 3: Finir la branche**

Invoquer `superpowers:finishing-a-development-branch`, qui décide de l'intégration.
