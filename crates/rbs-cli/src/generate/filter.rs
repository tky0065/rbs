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

/// Les trois colonnes que toute entité engendrée porte, filtrables sans `--fields`.
const COLONNES_DE_BASE: [&str; 3] = ["id", "created_at", "updated_at"];

/// Un champ vu par le filtre : sa colonne, sa variante de `Column`, son opérateur.
#[derive(Serialize)]
struct FilterField {
    name: String,
    pascal_name: String,
    operator: String,
    textual: bool,
}

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
    let textual = textual(field);

    FilterField {
        name: field.column_name(),
        pascal_name: field.pascal_name(),
        operator: match textual {
            true => "TextMatch".to_owned(),
            false => format!("Comparison<{}>", scalar_type(field)),
        },
        textual,
    }
}

/// Un texte se cherche par sous-chaîne, tout le reste se compare.
///
/// Une référence n'en est jamais une : elle porte un identifiant, que l'on compare.
fn textual(field: &Field) -> bool {
    field.reference().is_none()
        && matches!(field.column_type(), FieldType::String | FieldType::Text)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::bench;
    use crate::generate::feature::Feature;
    use crate::generate::fields;

    const CHAMPS: &str = "title:string,body:text:optional,views:int,published:bool,\
                          author_id:uuid,published_at:datetime";

    fn filtre(name: &str, champs: &str) -> String {
        let fields = fields::parse(champs).expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le filtre doit se rendre")
    }

    /// Chaque champ de `--fields` devient un champ du filtre, avec l'opérateur de son
    /// type : un texte se cherche par sous-chaîne, un scalaire se compare.
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

    /// Un champ `optional` porte déjà l'`Option` du filtre : comparer un `Option<i32>`
    /// n'aurait pas de sens, et `body` est ici la colonne nullable.
    #[test]
    fn an_optional_field_is_compared_on_its_bare_type() {
        let rendered = filtre("articles", "views:int:optional");

        assert!(
            rendered.contains("pub views: Option<Comparison<i32>>,"),
            "le type comparé doit être nu :\n{rendered}"
        );
        assert!(
            !rendered.contains("Comparison<Option<"),
            "l'`Option` du champ ne doit pas se cumuler :\n{rendered}"
        );
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
            rendered.contains(
                "id, created_at, updated_at, title, body, views, published, author_id, \
                 published_at"
            ),
            "le refus doit nommer les colonnes acceptées :\n{rendered}"
        );
    }

    /// Une référence porte le nom de sa colonne, et non celui de la relation : c'est
    /// `author_id` que le client envoie.
    #[test]
    fn a_reference_is_filtered_on_its_column() {
        let rendered = filtre("posts", "author:references:users");

        assert!(
            rendered.contains("pub author_id: Option<Comparison<Uuid>>,"),
            "la référence se filtre sur sa colonne :\n{rendered}"
        );
        assert!(
            rendered.contains(r#""author_id" => Column::AuthorId,"#),
            "la colonne de la référence doit se traduire :\n{rendered}"
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
            ".add(compare(Column::Views, filtre.views.as_ref()))",
            ".add(matches(Column::Title, filtre.title.as_ref()))",
            "colonne.contains(v)",
            "colonne.is_not_null()",
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

    /// Une référence se compare, jamais ne se cherche par sous-chaîne : c'est un
    /// identifiant.
    #[test]
    fn a_reference_goes_through_the_comparison_helper() {
        let rendered = filtre("posts", "author:references:users");

        assert!(
            rendered.contains(".add(compare(Column::AuthorId, filtre.author_id.as_ref()))"),
            "la référence doit se comparer :\n{rendered}"
        );
    }

    /// Le projet engendré compile sous `-D warnings` : un `use` que le fichier n'emploie
    /// pas y est une erreur, et ne se verrait qu'à la compilation d'un exemple.
    #[test]
    fn the_render_imports_only_what_it_uses() {
        let rendered = filtre("articles", CHAMPS);

        let (imports, corps) = rendered
            .split_once("\n\n")
            .expect("le bloc d'imports précède le corps");

        // Les traits sont importés pour leurs méthodes : `.eq`, `.contains`, `.filter` et
        // `.order_by_*` ne nomment jamais celui qui les porte.
        const TRAITS: [&str; 3] = ["ColumnTrait", "QueryFilter", "QueryOrder"];

        for import in imports.lines().flat_map(decompose) {
            if TRAITS.contains(&import.as_str()) {
                continue;
            }
            assert!(
                corps.contains(&import),
                "« {import} » est importé sans servir :\n{rendered}"
            );
        }
    }

    /// Les noms d'un `use` groupé, un par un.
    fn decompose(ligne: &str) -> Vec<String> {
        let Some((_, groupe)) = ligne.split_once('{') else {
            return ligne
                .trim_end_matches(';')
                .rsplit("::")
                .next()
                .map(|nom| vec![nom.to_owned()])
                .unwrap_or_default();
        };

        groupe
            .trim_end_matches("};")
            .split(',')
            .map(|nom| nom.trim().to_owned())
            .filter(|nom| !nom.is_empty() && *nom != "self")
            .collect()
    }

    /// Le rendu est écrit tel que rustfmt l'écrirait : sans quoi le `cargo fmt --check` du
    /// projet engendré échouerait sur ce que le CLI vient de produire.
    ///
    /// La signature d'`apply` est la seule ligne de ce fichier qui suive le nom de
    /// l'entité : elle vaut quatre-vingt-huit caractères de plus que lui, et franchit donc
    /// les cent colonnes de `max_width` à treize. Deux noms ne le voyaient pas — le seuil
    /// est désormais mesuré, et le test affiche sa valeur quand elle bouge.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| filtre(name, CHAMPS));

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu du filtre diverge de rustfmt à ces longueurs de nom"
        );
    }
}
