//! Rendu de `<nom>/controller.rs` et du `mod.rs` qui monte ses routes.

use crate::template::Renderer;

use super::feature::Feature;

const CONTROLLER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../templates/feature/controller.rs.jinja"
));

const MODULE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../templates/feature/mod.rs.jinja"
));

/// Rend les handlers de `feature` et leurs annotations OpenAPI.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(CONTROLLER, feature)
}

/// Rend le `mod.rs` de `feature` : ses six fichiers et son `routes()`.
pub(crate) fn rendre_mod(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(MODULE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{banc, champs, dto, entite, repository, service};

    fn controller(nom: &str) -> String {
        let champs = champs::analyser("titre:string").expect("champs valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("le controller doit se rendre")
    }

    fn module(nom: &str) -> String {
        let champs = champs::analyser("titre:string").expect("champs valides");
        rendre_mod(&Feature::nouvelle(nom, champs)).expect("le mod.rs doit se rendre")
    }

    #[test]
    fn les_cinq_handlers_sont_declares() {
        let rendu = controller("articles");

        for signature in [
            "pub async fn list(",
            "pub async fn create(",
            "pub async fn find(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendu.contains(signature),
                "« {signature} » absent :\n{rendu}"
            );
        }
    }

    #[test]
    fn chaque_handler_porte_son_annotation_utoipa() {
        let rendu = controller("articles");

        assert_eq!(
            rendu.matches("#[utoipa::path(").count(),
            5,
            "les cinq handlers doivent être documentés :\n{rendu}"
        );
    }

    #[test]
    fn les_cinq_verbes_et_leurs_chemins_sont_documentes() {
        let rendu = controller("blog_posts");

        for annotation in [
            "    get,\n    path = \"/blog_posts\",",
            "    post,\n    path = \"/blog_posts\",",
            "    get,\n    path = \"/blog_posts/{id}\",",
            "    put,\n    path = \"/blog_posts/{id}\",",
            "    delete,\n    path = \"/blog_posts/{id}\",",
        ] {
            assert!(
                rendu.contains(annotation),
                "annotation attendue absente :\n{annotation}\n---\n{rendu}"
            );
        }
    }

    #[test]
    fn les_corps_de_reponse_nomment_les_dto() {
        let rendu = controller("articles");

        assert!(
            rendu.contains("body = Page<ArticleResponse>"),
            "la liste doit annoncer une page :\n{rendu}"
        );
        assert_eq!(
            rendu.matches("body = ArticleResponse").count(),
            3,
            "create, find et update rendent l'entité :\n{rendu}"
        );
        assert!(
            rendu.contains("request_body = CreateArticle")
                && rendu.contains("request_body = UpdateArticle"),
            "les corps de requête doivent être documentés :\n{rendu}"
        );
    }

    #[test]
    fn la_creation_repond_201_et_la_suppression_204() {
        let rendu = controller("articles");

        assert!(rendu.contains("status = 201"), "{rendu}");
        assert!(rendu.contains("status = 204"), "{rendu}");
        assert!(
            rendu.contains("Ok((StatusCode::CREATED, Json(article)))"),
            "la création doit rendre 201 :\n{rendu}"
        );
        assert!(
            rendu.contains("Ok(StatusCode::NO_CONTENT)"),
            "la suppression doit rendre 204 :\n{rendu}"
        );
    }

    #[test]
    fn l_absence_est_documentee_la_ou_elle_peut_survenir() {
        let rendu = controller("articles");

        assert_eq!(
            rendu.matches("status = 404").count(),
            3,
            "find, update et delete peuvent ne rien trouver :\n{rendu}"
        );
    }

    #[test]
    fn les_corps_entrants_passent_par_la_validation_du_noyau() {
        let rendu = controller("articles");

        assert!(
            rendu.contains("ValidatedJson(entree): ValidatedJson<CreateArticle>"),
            "la création doit valider son corps :\n{rendu}"
        );
        assert!(
            rendu.contains("ValidatedJson(entree): ValidatedJson<UpdateArticle>"),
            "la mise à jour doit valider son corps :\n{rendu}"
        );
    }

    #[test]
    fn aucune_requete_seaorm_n_atteint_la_couche_http() {
        let rendu = controller("articles");

        assert!(
            !rendu.contains("sea_orm::Entity") && !rendu.contains("ActiveModel"),
            "le controller ne connaît que service.rs :\n{rendu}"
        );
        assert!(
            !rendu.contains("super::repository") && !rendu.contains("super::model"),
            "le controller ne connaît que service.rs :\n{rendu}"
        );
    }

    #[test]
    fn le_module_monte_les_cinq_routes() {
        let rendu = module("articles");

        assert!(
            rendu.contains(".route(\"/articles\", get(controller::list).post(controller::create))"),
            "routes de collection absentes :\n{rendu}"
        );
        assert!(
            rendu.contains("\"/articles/{id}\"")
                && rendu.contains("get(controller::find)")
                && rendu.contains(".put(controller::update)")
                && rendu.contains(".delete(controller::delete)"),
            "routes unitaires absentes :\n{rendu}"
        );
    }

    #[test]
    fn le_module_declare_les_six_fichiers_de_la_feature() {
        let rendu = module("articles");

        for declaration in [
            "pub mod controller;",
            "pub mod dto;",
            "pub mod model;",
            "pub mod repository;",
            "pub mod service;",
        ] {
            assert!(
                rendu.contains(declaration),
                "« {declaration} » absent :\n{rendu}"
            );
        }
    }

    #[test]
    fn les_rendus_traversent_rustfmt_sans_diff() {
        let controller = controller("articles");
        let module = module("articles");

        assert_eq!(
            banc::formate(&controller),
            controller,
            "controller reformaté"
        );
        assert_eq!(banc::formate(&module), module, "mod.rs reformaté");
    }

    /// Ce que le projet généré vérifie de son propre document OpenAPI.
    ///
    /// Le projet est un binaire : un test d'intégration ne pourrait pas atteindre son
    /// `ApiDoc`. La vérification est donc posée comme module de test du binaire lui-même.
    const VERIFICATION: &str = r#"use utoipa::OpenApi;

use crate::openapi::ApiDoc;

#[test]
fn les_cinq_routes_de_la_feature_sont_documentees() {
    let doc = ApiDoc::openapi();

    let collection = doc
        .paths
        .paths
        .get("/articles")
        .expect("chemin de collection absent du document");
    assert!(collection.get.is_some(), "GET de collection absent");
    assert!(collection.post.is_some(), "POST de collection absent");

    let unitaire = doc
        .paths
        .paths
        .get("/articles/{id}")
        .expect("chemin unitaire absent du document");
    assert!(unitaire.get.is_some(), "GET unitaire absent");
    assert!(unitaire.put.is_some(), "PUT unitaire absent");
    assert!(unitaire.delete.is_some(), "DELETE unitaire absent");
}

#[test]
fn chaque_route_annonce_le_schema_qu_elle_rend() {
    let doc = ApiDoc::openapi();
    let composants = doc.components.expect("composants absents du document");
    let noms: Vec<&str> = composants.schemas.keys().map(String::as_str).collect();

    for attendu in ["ArticleResponse", "CreateArticle", "UpdateArticle"] {
        assert!(
            noms.contains(&attendu),
            "schema {attendu} absent, present : {noms:?}"
        );
    }
    assert!(
        noms.iter().any(|nom| nom.contains("Page")),
        "le schema de la page est absent, present : {noms:?}"
    );
}
"#;

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn les_cinq_routes_paraissent_dans_le_document_openapi() {
        let fields = "titre:string,email:string:unique,resume:text:optional,vues:int,\
                      publie:bool,auteur_id:uuid,publie_le:datetime";
        let champs = champs::analyser(fields).expect("champs valides");
        let feature = Feature::nouvelle("articles", champs);

        let projet = banc::Projet::neuf();
        projet.poser_feature(
            "articles",
            &[
                ("mod.rs", &rendre_mod(&feature).expect("mod.rs rendu")),
                (
                    "model.rs",
                    &entite::rendre(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &dto::rendre(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::rendre(&feature).expect("repository rendu"),
                ),
                (
                    "service.rs",
                    &service::rendre(&feature).expect("service rendu"),
                ),
                (
                    "controller.rs",
                    &rendre(&feature).expect("controller rendu"),
                ),
            ],
        );
        projet.monter_feature("articles", &["list", "create", "find", "update", "delete"]);
        projet.poser_test_unitaire("verification_openapi", VERIFICATION);
        projet.tester();
    }

    /// Monte un projet complet sous `target/atelier/`, pour la revue de Swagger UI.
    #[test]
    #[ignore = "atelier : laisse un projet démarrable derrière lui"]
    fn atelier() {
        let fields = "titre:string,email:string:unique,resume:text:optional,vues:int,\
                      publie:bool,auteur_id:uuid,publie_le:datetime";
        let champs = champs::analyser(fields).expect("champs valides");
        let feature = Feature::nouvelle("articles", champs);

        let projet = banc::Projet::neuf();
        projet.poser_feature(
            "articles",
            &[
                ("mod.rs", &rendre_mod(&feature).expect("mod.rs rendu")),
                (
                    "model.rs",
                    &entite::rendre(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &dto::rendre(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::rendre(&feature).expect("repository rendu"),
                ),
                (
                    "service.rs",
                    &service::rendre(&feature).expect("service rendu"),
                ),
                (
                    "controller.rs",
                    &rendre(&feature).expect("controller rendu"),
                ),
            ],
        );
        projet.monter_feature("articles", &["list", "create", "find", "update", "delete"]);

        println!("{}", projet.conserver().display());
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn apercu() {
        println!(
            "{}\n// ---- mod.rs ----\n{}",
            controller("articles"),
            module("articles")
        );
    }
}
