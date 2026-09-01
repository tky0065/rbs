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
    use crate::generate::{bench, dto, entity, fields, filter, repository, service};

    fn controller(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le controller doit se rendre")
    }

    fn guarded(name: &str, role: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).guarded(role)).expect("le controller doit se rendre")
    }

    fn module(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), false).expect("le mod.rs doit se rendre")
    }

    fn module_with_tests(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render_mod(&Feature::fresh(name, fields), true).expect("le mod.rs doit se rendre")
    }

    /// `per_page=abc` rend 400 : un document qui ne l'annonce pas fait débugger au client
    /// une pagination qui « ne marche pas », sans rien pour l'aider.
    #[test]
    fn the_list_declares_the_400_of_the_pagination() {
        let rendered = controller("articles");

        let liste = rendered
            .split("pub async fn list(")
            .next()
            .expect("l'annotation précède le handler");

        assert!(
            liste.contains(r#"(status = 400, description = "pagination illisible""#),
            "le 400 de la pagination n'est pas déclaré :\n{liste}"
        );
    }

    /// Le service fusionne : un champ absent du corps garde sa valeur. `PUT` promettrait
    /// un remplacement que ce code ne fait pas ; `PATCH` dit exactement ce qu'il fait.
    #[test]
    fn the_update_is_a_patch_and_no_put_survives() {
        let rendered = controller("articles");
        let module = module("articles");

        assert!(rendered.contains("    patch,"), "{rendered}");
        assert!(!rendered.contains("    put,"), "{rendered}");
        assert!(
            !module.contains(".put("),
            "aucun alias `put` ne survit :\n{module}"
        );
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
            6,
            "les six handlers doivent être documentés :\n{rendered}"
        );
    }

    #[test]
    fn the_five_verbs_and_their_paths_are_documented() {
        let rendered = controller("blog_posts");

        for annotation in [
            "    get,\n    path = \"/blog_posts\",",
            "    post,\n    path = \"/blog_posts\",",
            "    get,\n    path = \"/blog_posts/{id}\",",
            "    patch,\n    path = \"/blog_posts/{id}\",",
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
    fn the_conflict_the_repository_can_raise_is_documented() {
        let rendered = controller("articles");

        assert_eq!(
            rendered.matches("status = 409").count(),
            2,
            "create et update traduisent un doublon en conflit, le contrat doit le \
             dire :\n{rendered}"
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

    /// Le bloc d'un handler : son annotation et sa fonction, isolées du reste du fichier.
    ///
    /// Ce que le garde doit prouver est distributif — trois routes le portent, deux ne le
    /// portent pas — et une recherche sur le fichier entier ne dirait pas laquelle.
    fn handler<'a>(rendered: &'a str, name: &str) -> &'a str {
        rendered
            .split("#[utoipa::path(")
            .find(|bloc| bloc.contains(&format!("pub async fn {name}(")))
            .unwrap_or_else(|| panic!("handler `{name}` absent :\n{rendered}"))
    }

    #[test]
    fn the_guard_protects_the_three_writes_and_spares_the_two_reads() {
        let rendered = guarded("articles", "admin");

        for name in ["create", "update", "delete"] {
            let bloc = handler(&rendered, name);

            assert!(
                bloc.contains("identite: Identity,"),
                "`{name}` doit extraire l'identité :\n{bloc}"
            );
            assert!(
                bloc.contains("identite.require_role(Role::Admin)?;"),
                "`{name}` doit exiger le rôle :\n{bloc}"
            );
        }

        for name in ["list", "find"] {
            let bloc = handler(&rendered, name);

            assert!(
                !bloc.contains("Identity") && !bloc.contains("require_role"),
                "`{name}` reste publique :\n{bloc}"
            );
        }
    }

    /// Le contrat OpenAPI n'annonce que les deux refus que le garde produit réellement :
    /// 401 de l'extracteur d'identité, 403 de `require_role`.
    #[test]
    fn the_guarded_routes_declare_the_bearer_and_the_two_refusals() {
        let rendered = guarded("articles", "admin");

        for annotation in [
            r#"security(("bearer" = []))"#,
            "status = 401",
            "status = 403",
        ] {
            assert_eq!(
                rendered.matches(annotation).count(),
                3,
                "« {annotation} » doit figurer sur les trois routes protégées :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_guard_names_the_role_in_pascal_case() {
        let rendered = guarded("articles", "super_admin");

        assert!(
            rendered.contains("identite.require_role(Role::SuperAdmin)?;"),
            "le rôle doit se traduire en variante de l'enum :\n{rendered}"
        );
    }

    #[test]
    fn without_a_role_the_controller_carries_nothing_of_the_guard() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("Identity")
                && !rendered.contains("require_role")
                && !rendered.contains("status = 401"),
            "sans `--role`, le rendu est inchangé :\n{rendered}"
        );
    }

    /// Le garde allonge trois signatures, et celle de `delete` franchit les 100 colonnes
    /// où rustfmt bascule : la template doit l'écrire déjà éclatée.
    ///
    /// Les noms exercés sont ceux dont le rendu sans garde traverse déjà rustfmt sans
    /// diff — au-delà, c'est la ligne `use super::dto::{…}` qui déborde, indépendamment du
    /// garde, et `format::format_batch` s'en charge à l'écriture.
    #[test]
    fn the_guarded_render_is_already_what_rustfmt_would_write() {
        for name in ["tag", "articles"] {
            let rendered = guarded(name, "admin");

            assert_eq!(
                bench::formatted(&rendered),
                rendered,
                "le rendu de `{name}` diverge de rustfmt"
            );
        }
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
                && rendered.contains(".patch(controller::update)")
                && rendered.contains(".delete(controller::delete)"),
            "routes unitaires absentes :\n{rendered}"
        );
    }

    /// La route littérale se monte avant `/{id}`, sans quoi `filter` serait lu comme un
    /// identifiant — c'est ce que fait déjà `broadcast` dans `examples/newsletter-queue`.
    #[test]
    fn the_filter_route_is_mounted_before_the_id_route() {
        let rendered = module("articles");

        // Les chemins sont cherchés entre guillemets : le commentaire qui précède la
        // route nomme `/articles/{id}` sans les siens, et serait trouvé le premier.
        let filtre = rendered
            .find(r#""/articles/filter""#)
            .expect("route de filtre montée");
        let id = rendered
            .find(r#""/articles/{id}""#)
            .expect("route d'identifiant montée");

        assert!(
            filtre < id,
            "`filter` doit précéder l'identifiant :\n{rendered}"
        );
    }

    /// Filtrer est une lecture : le garde de rôle ne la protège pas, comme il ne protège
    /// ni `list` ni `find`.
    #[test]
    fn the_filter_route_stays_open_under_a_role() {
        let rendered = guarded("articles", "admin");

        assert!(
            !handler(&rendered, "filter").contains("require_role"),
            "filtrer est une lecture :\n{rendered}"
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
    assert!(unit.patch.is_some(), "PATCH unitaire absent");
    assert!(unit.delete.is_some(), "DELETE unitaire absent");

    let filtre = doc
        .paths
        .paths
        .get("/articles/filter")
        .expect("la route de filtrage doit etre documentee");
    assert!(filtre.post.is_some(), "POST de filtrage absent");
}

#[test]
fn chaque_route_annonce_le_schema_qu_elle_rend() {
    let doc = ApiDoc::openapi();
    let composants = doc.components.expect("composants absents du document");
    let names: Vec<&str> = composants.schemas.keys().map(String::as_str).collect();

    for expected in [
        "ArticleResponse",
        "CreateArticle",
        "UpdateArticle",
        "ArticleFilter",
    ] {
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
                    "filter.rs",
                    &filter::render(&feature).expect("filtre rendu"),
                ),
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
                    "filter.rs",
                    &filter::render(&feature).expect("filtre rendu"),
                ),
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
}
