//! Rendu de `<name>/model.rs` : l'entité SeaORM d'une feature.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/model.rs.jinja"
));

/// Rend l'entité SeaORM de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, fields};

    fn entity(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("l'entité doit se rendre")
    }

    #[test]
    fn the_primary_key_is_a_uuid_without_auto_increment() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("#[sea_orm(primary_key, auto_increment = false)]\n    pub id: Uuid,"),
            "clé primaire attendue en Uuid non auto-incrémenté :\n{rendered}"
        );
    }

    // Tous les chemins d'insertion du projet passent par `..Default::default()`, que la
    // macro fait déléguer à `ActiveModelBehavior::new()` : c'est le seul point à écrire.
    #[test]
    fn the_model_lays_the_v7_identifier_itself() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("Uuid::now_v7()"),
            "le modèle ne pose pas l'identifiant :\n{rendered}"
        );
        assert!(
            rendered.contains("fn new() -> Self"),
            "l'identifiant n'est pas posé par `new()`, seul point que `Default` appelle :\n{rendered}"
        );
    }

    #[test]
    fn the_table_carries_the_plural_name_of_the_feature() {
        let rendered = entity("blog_posts", "title:string");

        assert!(
            rendered.contains(r#"#[sea_orm(table_name = "blog_posts")]"#),
            "nom de table absent :\n{rendered}"
        );
    }

    #[test]
    fn each_type_of_the_grammar_projects_into_the_entity() {
        let rendered = entity(
            "samples",
            "title:string,quantity:int,price:float,active:bool,owner:uuid,\
             published_at:datetime,body:text",
        );

        for expected in [
            "pub title: String,",
            "pub quantity: i32,",
            "pub price: f64,",
            "pub active: bool,",
            "pub owner: Uuid,",
            "pub published_at: DateTimeWithTimeZone,",
            "pub body: String,",
        ] {
            assert!(
                rendered.contains(expected),
                "« {expected} » absent de :\n{rendered}"
            );
        }
    }

    #[test]
    fn an_optional_field_becomes_an_option() {
        let rendered = entity("users", "bio:string:optional");

        assert!(
            rendered.contains("pub bio: Option<String>,"),
            "champ optionnel non rendu en Option :\n{rendered}"
        );
    }

    #[test]
    fn a_unique_field_carries_the_matching_attribute() {
        let rendered = entity("users", "email:string:unique");

        assert!(
            rendered.contains("#[sea_orm(unique)]\n    pub email: String,"),
            "attribut unique absent :\n{rendered}"
        );
    }

    #[test]
    fn an_indexed_field_carries_the_matching_attribute() {
        let rendered = entity("users", "slug:string:index");

        assert!(
            rendered.contains("#[sea_orm(indexed)]\n    pub slug: String,"),
            "attribut indexed absent :\n{rendered}"
        );
    }

    #[test]
    fn a_text_field_forces_its_column_type() {
        let rendered = entity("articles", "corps:text");

        assert!(
            rendered.contains(r#"#[sea_orm(column_type = "Text")]"#),
            "type de colonne Text non forcé :\n{rendered}"
        );
    }

    #[test]
    fn stacked_modifiers_fit_in_a_single_attribute() {
        let rendered = entity("articles", "summary:text:index");

        assert!(
            rendered.contains(r#"#[sea_orm(column_type = "Text", indexed)]"#),
            "modificateurs non cumulés :\n{rendered}"
        );
    }

    #[test]
    fn the_timestamps_are_set_without_having_been_declared() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("pub created_at: DateTimeWithTimeZone,"),
            "{rendered}"
        );
        assert!(
            rendered.contains("pub updated_at: DateTimeWithTimeZone,"),
            "{rendered}"
        );
    }

    #[test]
    fn a_field_less_feature_renders_a_complete_entity() {
        let rendered = entity("tokens", "");

        assert!(rendered.contains("pub struct Model {"), "{rendered}");
        assert!(rendered.contains("pub enum Relation {}"), "{rendered}");
        assert!(
            rendered.contains("impl ActiveModelBehavior for ActiveModel {"),
            "{rendered}"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_entity_compiles_in_a_fresh_project() {
        let project = bench::Project::fresh();
        let rendered = entity(
            "articles",
            "title:string,slug:string:unique,summary:text:optional,views:int,published:bool,\
             auteur_id:uuid,published_at:datetime",
        );

        project.write_feature("articles", &[("model.rs", &rendered)]);
        project.compile();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn preview() {
        println!(
            "{}",
            entity(
                "articles",
                "title:string,slug:string:unique,summary:text:optional,views:int,published:bool"
            )
        );
    }

    #[test]
    fn the_render_ends_with_a_single_newline() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.ends_with("}\n"),
            "fin de fichier inattendue :\n{rendered}"
        );
        assert!(
            !rendered.ends_with("\n\n"),
            "ligne vide finale :\n{rendered}"
        );
    }
}
