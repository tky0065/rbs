//! Rendu de `<name>/controller.rs` et du `mod.rs` qui monte ses routes.

use minijinja::{Value, context};

use crate::template::Renderer;

use super::feature::Feature;

const CONTROLLER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/controller.rs.jinja"
));

const MODULE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/mod.rs.jinja"
));

/// Rend les handlers de `feature` et leurs annotations OpenAPI.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(CONTROLLER, feature)
}

/// Rend le `mod.rs` de `feature` : ses fichiers et son `routes()`.
///
/// `with_tests` déclare le module `tests` : une feature écrite à la main n'en porte pas,
/// et le déclarer empêcherait la compilation.
pub(crate) fn render_mod(feature: &Feature, with_tests: bool) -> Result<String, minijinja::Error> {
    Renderer::new().render(
        MODULE,
        context! { with_tests, ..Value::from_serialize(feature) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{bench, dto, entity, fields, repository, service};

    fn controller(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le controller doit se rendre")
    }

    fn module(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), false).expect("le mod.rs doit se rendre")
    }

    fn module_with_tests(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), true).expect("le mod.rs doit se rendre")
    }

    #[test]
    fn the_five_handlers_are_declared() {
        let rendered = controller("articles");

        for signature in [
            "pub async fn list(",
            "pub async fn create(",
            "pub async fn find(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendered.contains(signature),
                "« {signature} » absent :\n{rendered}"
            );
        }
    }

    #[test]
    fn each_handler_carries_its_utoipa_annotation() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("#[utoipa::path(").count(),
            5,
            "les cinq handlers doivent être documentés :\n{rendered}"
        );
    }

    #[test]
    fn the_five_verbs_and_their_paths_are_documented() {
        let rendered = controller("blog_posts");

        for annotation in [
            "    get,\n    path = \"/blog_posts\",",
            "    post,\n    path = \"/blog_posts\",",
            "    get,\n    path = \"/blog_posts/{id}\",",
            "    put,\n    path = \"/blog_posts/{id}\",",
            "    delete,\n    path = \"/blog_posts/{id}\",",
        ] {
            assert!(
                rendered.contains(annotation),
                "annotation attendue absente :\n{annotation}\n---\n{rendered}"
            );
        }
    }

    #[test]
    fn the_response_bodies_name_the_dtos() {
        let rendered = controller("articles");

        assert!(
            rendered.contains("body = Page<ArticleResponse>"),
            "la liste doit annoncer une page :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("body = ArticleResponse").count(),
            3,
            "create, find et update rendent l'entité :\n{rendered}"
        );
        assert!(
            rendered.contains("request_body = CreateArticle")
                && rendered.contains("request_body = UpdateArticle"),
            "les corps de requête doivent être documentés :\n{rendered}"
        );
    }

    #[test]
    fn creation_answers_201_and_deletion_204() {
        let rendered = controller("articles");

        assert!(rendered.contains("status = 201"), "{rendered}");
        assert!(rendered.contains("status = 204"), "{rendered}");
        assert!(
            rendered.contains("Ok((StatusCode::CREATED, Json(article)))"),
            "la création doit rendre 201 :\n{rendered}"
        );
        assert!(
            rendered.contains("Ok(StatusCode::NO_CONTENT)"),
            "la suppression doit rendre 204 :\n{rendered}"
        );
    }

    #[test]
    fn absence_is_documented_where_it_can_occur() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("status = 404").count(),
            3,
            "find, update et delete peuvent ne rien trouver :\n{rendered}"
        );
    }

    #[test]
    fn incoming_bodies_go_through_the_core_validation() {
        let rendered = controller("articles");

        assert!(
            rendered.contains("ValidatedJson(input): ValidatedJson<CreateArticle>"),
            "la création doit valider son corps :\n{rendered}"
        );
        assert!(
            rendered.contains("ValidatedJson(input): ValidatedJson<UpdateArticle>"),
            "la mise à jour doit valider son corps :\n{rendered}"
        );
    }

    #[test]
    fn no_seaorm_query_reaches_the_http_layer() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("sea_orm::Entity") && !rendered.contains("ActiveModel"),
            "le controller ne connaît que service.rs :\n{rendered}"
        );
        assert!(
            !rendered.contains("super::repository") && !rendered.contains("super::model"),
            "le controller ne connaît que service.rs :\n{rendered}"
        );
    }

    #[test]
    fn the_module_mounts_the_five_routes() {
        let rendered = module("articles");

        assert!(
            rendered
                .contains(".route(\"/articles\", get(controller::list).post(controller::create))"),
            "routes de collection absentes :\n{rendered}"
        );
        assert!(
            rendered.contains("\"/articles/{id}\"")
                && rendered.contains("get(controller::find)")
                && rendered.contains(".put(controller::update)")
                && rendered.contains(".delete(controller::delete)"),
            "routes unitaires absentes :\n{rendered}"
        );
    }

    #[test]
    fn the_module_declares_the_six_files_of_the_feature() {
        let rendered = module("articles");

        for declaration in [
            "pub mod controller;",
            "pub mod dto;",
            "pub mod model;",
            "pub mod repository;",
            "pub mod service;",
        ] {
            assert!(
                rendered.contains(declaration),
                "« {declaration} » absent :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_module_declares_the_tests_when_they_are_generated() {
        let avec = module_with_tests("articles");

        assert!(
            avec.contains("#[cfg(test)]\nmod tests;"),
            "le module de tests doit être déclaré :\n{avec}"
        );
    }

    /// Une feature écrite à la main n'a pas de `tests.rs` : le déclarer empêcherait la
    /// compilation du projet.
    #[test]
    fn the_module_declares_no_tests_when_there_are_none() {
        let sans = module("articles");

        assert!(!sans.contains("mod tests;"), "{sans}");
    }

    /// Ce que le projet généré vérifie de son propre document OpenAPI.
    ///
    /// Le projet est un binaire : un test d'intégration ne pourrait pas atteindre son
    /// `ApiDoc`. La vérification est donc posée comme module de test du binaire lui-même.
    const VERIFICATION: &str = r#"use utoipa::OpenApi;

use demo_api::openapi::ApiDoc;

#[test]
fn the_five_routes_of_the_feature_are_documented() {
    let doc = ApiDoc::openapi();

    let collection = doc
        .paths
        .paths
        .get("/articles")
        .expect("chemin de collection absent du document");
    assert!(collection.get.is_some(), "GET de collection absent");
    assert!(collection.post.is_some(), "POST de collection absent");

    let unit = doc
        .paths
        .paths
        .get("/articles/{id}")
        .expect("chemin unitaire absent du document");
    assert!(unit.get.is_some(), "GET unitaire absent");
    assert!(unit.put.is_some(), "PUT unitaire absent");
    assert!(unit.delete.is_some(), "DELETE unitaire absent");
}

#[test]
fn chaque_route_annonce_le_schema_qu_elle_rend() {
    let doc = ApiDoc::openapi();
    let composants = doc.components.expect("composants absents du document");
    let names: Vec<&str> = composants.schemas.keys().map(String::as_str).collect();

    for expected in ["ArticleResponse", "CreateArticle", "UpdateArticle"] {
        assert!(
            names.contains(&expected),
            "schema {expected} absent, present : {names:?}"
        );
    }
    assert!(
        names.iter().any(|name| name.contains("Page")),
        "le schema de la page est absent, present : {names:?}"
    );
}
"#;

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_five_routes_appear_in_the_openapi_document() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature(
            "articles",
            &[
                (
                    "mod.rs",
                    &render_mod(&feature, false).expect("mod.rs rendu"),
                ),
                (
                    "model.rs",
                    &entity::render(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &dto::render(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::render(&feature).expect("repository rendu"),
                ),
                (
                    "service.rs",
                    &service::render(&feature).expect("service rendu"),
                ),
                (
                    "controller.rs",
                    &render(&feature).expect("controller rendu"),
                ),
            ],
        );
        project.mount_feature("articles");
        project.write_unit_test("verification_openapi", VERIFICATION);
        project.test_of();
    }

    /// Monte un projet complet sous `target/workshop/`, pour la revue de Swagger UI.
    #[test]
    #[ignore = "atelier : laisse un projet démarrable derrière lui"]
    fn workshop() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature(
            "articles",
            &[
                (
                    "mod.rs",
                    &render_mod(&feature, false).expect("mod.rs rendu"),
                ),
                (
                    "model.rs",
                    &entity::render(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &dto::render(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::render(&feature).expect("repository rendu"),
                ),
                (
                    "service.rs",
                    &service::render(&feature).expect("service rendu"),
                ),
                (
                    "controller.rs",
                    &render(&feature).expect("controller rendu"),
                ),
            ],
        );
        project.mount_feature("articles");

        println!("{}", project.keep().display());
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn preview() {
        println!(
            "{}\n// ---- mod.rs ----\n{}",
            controller("articles"),
            module("articles")
        );
    }
}
