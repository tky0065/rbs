//! Rendu de `<name>/service.rs` : les décisions métier de la feature.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/service.rs.jinja"
));

/// Rend le service de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{bench, dto, entity, fields, filter, repository};

    fn service(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("le service doit se rendre")
    }

    #[test]
    fn the_service_exposes_the_five_crud_operations() {
        let rendered = service("articles", "title:string");

        for signature in [
            "pub async fn list(\n    db: &DatabaseConnection,\n    pagination: &Pagination,\n) -> Result<Page<ArticleResponse>> {",
            "pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<ArticleResponse> {",
            "pub async fn create(db: &DatabaseConnection, input: CreateArticle) -> Result<ArticleResponse> {",
            "pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {",
        ] {
            assert!(
                rendered.contains(signature),
                "« {signature} » absente :\n{rendered}"
            );
        }
        assert!(
            rendered.contains("pub async fn update(") && rendered.contains("input: UpdateArticle"),
            "signature d'update inattendue :\n{rendered}"
        );
    }

    #[test]
    fn no_seaorm_query_is_built_here() {
        let rendered = service("articles", "title:string,views:int");

        assert!(
            !rendered.contains("EntityTrait"),
            "le service ne parle pas à la base :\n{rendered}"
        );
        for interdit in ["Entity::find", "Entity::delete", ".all(db)", ".one(db)"] {
            assert!(
                !rendered.contains(interdit),
                "« {interdit} » n'a rien à faire ici :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_entity_is_only_reached_through_the_repository() {
        let rendered = service("articles", "title:string");

        assert!(
            !rendered.contains("super::model"),
            "le service ne connaît que repository.rs et dto.rs :\n{rendered}"
        );
        assert!(
            rendered.contains("use super::repository::{self, ActiveModel};"),
            "l'ActiveModel doit venir du repository :\n{rendered}"
        );
    }

    #[test]
    fn absence_becomes_a_named_error() {
        let rendered = service("blog_posts", "title:string");

        assert_eq!(
            rendered.matches(r#"Error::NotFound("blog_post")"#).count(),
            3,
            "find, update et delete doivent chacun signaler l'absence :\n{rendered}"
        );
    }

    #[test]
    fn the_list_assembles_the_core_page() {
        let rendered = service("articles", "title:string");

        assert!(
            rendered.contains("let (articles, total) = repository::list(db, pagination).await?;"),
            "la liste doit venir du repository :\n{rendered}"
        );
        assert!(
            rendered.contains("Ok(Page::new("),
            "l'assemblage de la page revient au service :\n{rendered}"
        );
    }

    #[test]
    fn creation_sets_every_declared_field() {
        let rendered = service("articles", "title:string,summary:text:optional");

        assert!(rendered.contains("title: Set(input.title),"), "{rendered}");
        assert!(
            rendered.contains("summary: Set(input.summary),"),
            "{rendered}"
        );
        assert!(
            rendered.contains("..Default::default()"),
            "id et horodatages sont posés par la base :\n{rendered}"
        );
    }

    #[test]
    fn the_update_only_writes_the_fields_it_receives() {
        let rendered = service("articles", "title:string,views:int");

        assert!(
            rendered.contains(
                "if let Some(title) = input.title {\n        article.title = Set(title);\n    }"
            ),
            "champ obligatoire mal appliqué :\n{rendered}"
        );
        assert!(
            rendered.contains(
                "if let Some(views) = input.views {\n        article.views = Set(views);\n    }"
            ),
            "champ obligatoire mal appliqué :\n{rendered}"
        );
    }

    #[test]
    fn an_optional_field_stays_optional_in_the_column() {
        let rendered = service("articles", "summary:text:optional");

        assert!(
            rendered.contains("article.summary = Set(Some(summary));"),
            "la colonne est nullable : la valeur reçue doit y être réemballée :\n{rendered}"
        );
    }

    #[test]
    fn the_update_timestamps_the_change() {
        let rendered = service("articles", "title:string");

        assert!(
            rendered.contains("article.updated_at = Set(chrono::Utc::now().into());"),
            "`updated_at` doit être posé par le service :\n{rendered}"
        );
    }

    #[test]
    fn a_field_less_feature_renders_a_valid_service() {
        let rendered = service("tokens", "");

        assert!(rendered.contains("let token = ActiveModel {"), "{rendered}");
        assert!(
            !rendered.contains("if let Some("),
            "rien à appliquer :\n{rendered}"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_service_compiles_in_a_fresh_project() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
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
                    "filter.rs",
                    &filter::render(&feature).expect("filtre rendu"),
                ),
                ("dto.rs", &dto::render(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::render(&feature).expect("repository rendu"),
                ),
                ("service.rs", &render(&feature).expect("service rendu")),
            ],
        );
        project.compile();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn preview() {
        println!(
            "{}",
            service("articles", "title:string,summary:text:optional,views:int")
        );
    }
}
