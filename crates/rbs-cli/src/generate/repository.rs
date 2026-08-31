//! Rendu de `<name>/repository.rs` : le seul fichier qui parle à la base.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/repository.rs.jinja"
));

/// Rend le repository de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{bench, entity, fields};

    fn repository(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("le repository doit se rendre")
    }

    #[test]
    fn the_repository_exposes_the_five_crud_operations() {
        let rendered = repository("articles", "title:string");

        for signature in [
            "pub async fn list(",
            "pub async fn find(",
            "pub async fn create(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendered.contains(signature),
                "« {signature} » absente :\n{rendered}"
            );
        }
    }

    #[test]
    fn no_axum_import_appears() {
        let rendered = repository("articles", "title:string,views:int");

        assert!(
            !rendered.contains("axum"),
            "le repository ignore la couche HTTP :\n{rendered}"
        );
    }

    #[test]
    fn the_repository_ignores_the_dtos_and_the_rendered_pagination() {
        let rendered = repository("articles", "title:string");

        assert!(
            !rendered.contains("super::dto"),
            "le repository ne connaît que model.rs :\n{rendered}"
        );
        assert!(
            !rendered.contains("Page<"),
            "assembler la page revient au service :\n{rendered}"
        );
    }

    #[test]
    fn the_list_returns_the_page_and_its_total() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {"),
            "signature de list inattendue :\n{rendered}"
        );
    }

    #[test]
    fn the_list_bounds_the_query_with_the_window_it_receives() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains(".offset(pagination.offset())")
                && rendered.contains(".limit(pagination.per_page())"),
            "la fenêtre de pagination n'est pas appliquée :\n{rendered}"
        );
    }

    #[test]
    fn the_ordering_follows_the_descending_id() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains(".order_by_desc(Column::Id)"),
            "l'ordre de la liste n'est pas déterministe :\n{rendered}"
        );
    }

    #[test]
    fn the_model_is_the_services_door_to_the_entity() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("pub use super::model::{ActiveModel, Model};"),
            "le service ne pourra pas atteindre l'entité sans nommer model.rs :\n{rendered}"
        );
    }

    #[test]
    fn deletion_reports_whether_a_row_disappeared() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains(
                "pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {"
            ),
            "signature de delete inattendue :\n{rendered}"
        );
        assert!(
            rendered.contains("rows_affected"),
            "la suppression doit constater son effet :\n{rendered}"
        );
    }

    /// Sans cette traduction, un second POST sur une colonne `unique` rend 500 : une
    /// faute du client servie comme une panne du serveur.
    #[test]
    fn a_unique_violation_becomes_a_conflict_rather_than_a_500() {
        let rendered = repository("articles", "email:string:unique");

        assert!(
            rendered.contains("Some(SqlErr::UniqueConstraintViolation(_)) => {"),
            "la violation d'unicité doit être reconnue :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"Error::Conflict("cette valeur est déjà prise".to_owned())"#),
            "elle doit devenir un conflit :\n{rendered}"
        );
        assert!(
            rendered.contains("use sea_orm::error::SqlErr;"),
            "`SqlErr` doit être importé :\n{rendered}"
        );
    }

    /// Le gabarit ne sait pas quelle colonne a fauté : le message ne peut pas la nommer,
    /// et surtout pas recopier celui, propre à l'email, du fragment `auth`.
    #[test]
    fn the_conflict_message_stays_generic() {
        let rendered = repository("articles", "email:string:unique");

        assert!(!rendered.contains("adresse"), "{rendered}");
        assert!(!rendered.contains("inscrite"), "{rendered}");
    }

    /// Les deux écritures passent par la même porte : sans cela, `PUT` rendrait 500 là où
    /// `POST` rend 409, pour la même contrainte.
    #[test]
    fn the_creation_and_the_update_share_the_same_translation() {
        let rendered = repository("articles", "email:string:unique");

        assert!(
            rendered.contains(".insert(db).await.map_err(conflict_on_duplicate)"),
            "la création doit traduire :\n{rendered}"
        );
        assert!(
            rendered.contains(".update(db).await.map_err(conflict_on_duplicate)"),
            "la mise à jour doit traduire :\n{rendered}"
        );
    }

    #[test]
    fn the_render_depends_only_on_the_feature_name() {
        let sans_champ = repository("articles", "");
        let avec_champs = repository("articles", "title:string,views:int,summary:text:optional");

        assert_eq!(
            sans_champ, avec_champs,
            "le CRUD est le même quels que soient les champs"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_repository_compiles_in_a_fresh_project() {
        let fields =
            fields::parse("title:string,views:int,summary:text:optional").expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature(
            "articles",
            &[
                (
                    "model.rs",
                    &entity::render(&feature).expect("entité rendue"),
                ),
                (
                    "repository.rs",
                    &render(&feature).expect("repository rendu"),
                ),
            ],
        );
        project.compile();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn preview() {
        println!("{}", repository("articles", "title:string,views:int"));
    }
}
