//! Rendu de `src/seeds/<name>.rs` : les données de démonstration d'une feature.
//!
//! Le seed passe par l'entité générée, et non par du SQL : un champ renommé casse à la
//! compilation plutôt qu'en silence à l'exécution — et rien de ce qui est écrit ici ne
//! dépend d'un dialecte.

use minijinja::context;
use serde::Serialize;

use crate::template::Renderer;

use super::feature::Feature;
use super::fields::{Field, FieldType};

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/seed.rs.jinja"
));

/// Nombre de lignes que le seed insère.
const LIGNES: usize = 2;

/// Rend le seed de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    let lignes: Vec<Vec<SeedField>> = (1..=LIGNES)
        .map(|rang| {
            feature
                .fields
                .iter()
                .map(|c| SeedField::at(c, rang))
                .collect()
        })
        .collect();

    Renderer::new().render(
        TEMPLATE,
        context! {
            module => feature.module(),
            lignes => lignes,
            uuid => feature.fields.iter().any(|c| c.type_ == FieldType::Uuid),
        },
    )
}

/// Un champ vu par le seed : son nom, et l'expression Rust de sa valeur.
#[derive(Serialize)]
struct SeedField {
    name: String,
    value: String,
}

impl SeedField {
    fn at(champ: &Field, rang: usize) -> Self {
        let value = value(champ, rang);

        Self {
            name: champ.name.clone(),
            // Un champ optionnel reste une colonne comme les autres : la renseigner rend
            // le seed lisible, là où un `None` ne montrerait rien.
            value: if champ.optionnel {
                format!("Some({value})")
            } else {
                value
            },
        }
    }
}

/// Expression Rust de la valeur de `champ` sur la ligne `rang`.
///
/// Les deux lignes doivent différer partout : un champ `unique` refuserait la seconde
/// sinon, et le seed échouerait à sa première exécution.
///
/// `uuid` passe par `from_u128` plutôt que par `new_v4` : le générateur v4 demande une
/// feature que le projet n'active que pour ses tests, et le binaire des seeds n'en est pas.
fn value(champ: &Field, rang: usize) -> String {
    match champ.type_ {
        FieldType::String | FieldType::Text if champ.validates_email() => {
            format!("\"{}-{rang}@example.com\".to_owned()", champ.name)
        }
        FieldType::String | FieldType::Text => format!("\"{}-{rang}\".to_owned()", champ.name),
        FieldType::Int => (41 + rang).to_string(),
        FieldType::Float => format!("{}.2", 3 + rang),
        FieldType::Bool => (rang % 2 == 1).to_string(),
        FieldType::Uuid => format!("Uuid::from_u128({rang})"),
        FieldType::Datetime => "chrono::Utc::now().into()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, fields};

    const CHAMPS: &str = "title:string,email:string:unique,summary:text:optional,views:int,\
                          note:float,published:bool,auteur_id:uuid,published_at:datetime";

    fn seed(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("le seed doit se rendre")
    }

    #[test]
    fn the_entity_is_reached_by_its_path_in_the_feature() {
        let rendered = seed("articles", CHAMPS);

        assert!(
            rendered.contains("#[path = \"../articles/model.rs\"]\nmod model;"),
            "l'entité doit être rejointe par son chemin :\n{rendered}"
        );
    }

    /// Le point du lot : c'est l'entité qui est construite, non une requête SQL.
    #[test]
    fn the_rows_go_through_the_generated_active_model() {
        let rendered = seed("articles", CHAMPS);

        assert_eq!(
            rendered.matches("model::ActiveModel {").count(),
            LIGNES,
            "{rendered}"
        );
        assert!(!rendered.contains("INSERT"), "aucun SQL :\n{rendered}");
        assert!(!rendered.contains("execute"), "aucun SQL :\n{rendered}");
    }

    /// Les trois colonnes que rbs pose lui-même ont un défaut en base : les renseigner
    /// ferait mentir la clé primaire ordonnée que le repository suppose.
    #[test]
    fn the_columns_rbs_lays_down_are_left_to_the_database() {
        let rendered = seed("articles", CHAMPS);

        for colonne in ["id: ", "created_at: ", "updated_at: "] {
            assert!(
                !rendered
                    .lines()
                    .any(|line| line.trim_start().starts_with(colonne)),
                "« {colonne} » ne doit pas être renseignée :\n{rendered}"
            );
        }
        assert_eq!(
            rendered.matches("..Default::default()").count(),
            LIGNES,
            "{rendered}"
        );
    }

    #[test]
    fn each_type_receives_a_value_of_its_own_type() {
        let rendered = seed("articles", CHAMPS);

        for value in [
            "title: Set(\"title-1\".to_owned())",
            "email: Set(\"email-1@example.com\".to_owned())",
            "summary: Set(Some(\"summary-1\".to_owned()))",
            "views: Set(42)",
            "note: Set(4.2)",
            "published: Set(true)",
            "auteur_id: Set(Uuid::from_u128(1))",
            "published_at: Set(chrono::Utc::now().into())",
        ] {
            assert!(rendered.contains(value), "« {value} » absent :\n{rendered}");
        }
    }

    /// Deux lignes identiques buteraient sur le premier champ `unique`.
    #[test]
    fn the_second_row_differs_from_the_first_everywhere_it_can() {
        let rendered = seed("articles", CHAMPS);

        for value in [
            "title: Set(\"title-2\".to_owned())",
            "email: Set(\"email-2@example.com\".to_owned())",
            "views: Set(43)",
            "note: Set(5.2)",
            "published: Set(false)",
            "auteur_id: Set(Uuid::from_u128(2))",
        ] {
            assert!(rendered.contains(value), "« {value} » absent :\n{rendered}");
        }
    }

    /// `new_v4` demanderait la feature `v4`, que le projet n'active que pour ses tests.
    #[test]
    fn no_uuid_generator_the_binary_does_not_have_is_called() {
        let rendered = seed("articles", CHAMPS);

        assert!(!rendered.contains("new_v4"), "{rendered}");
    }

    #[test]
    fn a_feature_without_a_uuid_does_not_import_one() {
        let rendered = seed("articles", "title:string");

        assert!(
            !rendered.contains("use sea_orm::prelude::Uuid;"),
            "une importation inutilisée laisserait un avertissement :\n{rendered}"
        );
    }

    #[test]
    fn a_field_less_feature_still_inserts_its_rows() {
        let rendered = seed("tokens", "");

        assert_eq!(
            rendered.matches("model::ActiveModel {").count(),
            LIGNES,
            "{rendered}"
        );
        assert!(
            !rendered.contains("Set("),
            "aucun champ à poser :\n{rendered}"
        );
    }

    #[test]
    fn the_render_ends_with_a_single_newline() {
        let rendered = seed("articles", CHAMPS);

        assert!(rendered.ends_with("}\n"), "fin inattendue :\n{rendered}");
        assert!(
            !rendered.ends_with("\n\n"),
            "ligne vide finale :\n{rendered}"
        );
    }

    /// Le test qui lit ce que le seed a réellement laissé dans la base.
    ///
    /// Le projet est un binaire : rien, du dehors, n'atteint son routeur. La lecture passe
    /// donc par un module de test posé dans le projet, monté comme au démarrage.
    const LECTURE: &str = r#"use axum::body::{Body, to_bytes};
use axum::http::Request;
use serde_json::Value;
use tower::ServiceExt;

use crate::router::router;
use crate::state::AppState;

#[tokio::test]
async fn les_semis_sont_rendus_par_l_api() {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let api = router(AppState::new(db, config).expect("état partagé constructible"));

    let request = Request::builder()
        .method("GET")
        .uri("/semis")
        .body(Body::empty())
        .expect("requête bien formée");

    let response = api.oneshot(request).await.expect("l'application doit répondre");
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");
    let body: Value = serde_json::from_slice(&bytes).expect("corps JSON");

    let rendu = body.to_string();
    for attendu in ["title-1", "title-2"] {
        assert!(rendu.contains(attendu), "« {attendu} » absent de {rendu}");
    }
}
"#;

    /// Le second critère du lot, et le seul que rien d'autre ne prouve : les lignes du
    /// seed traversent la base puis l'API.
    #[test]
    #[ignore = "démarre PostgreSQL 18 en conteneur et compile un projet complet"]
    fn the_seeded_rows_come_back_from_the_api() {
        let base = bench::TestDatabase::start();
        let project = bench::Project::fresh_on(base.url());

        project.rbs_ok(&["generate", "crud", "semis", "--fields", "title:string"]);
        project.rbs_ok(&["migrate", "up"]);
        project.rbs_ok(&["seed"]);

        project.write_unit_test("lecture_des_semis", LECTURE);
        project.test_matching("les_semis_sont_rendus_par_l_api");

        // Le seed déposé est du code que l'utilisateur n'a pas écrit : il ne doit pas
        // rendre rouge la CI que `rbs add ci` pose dans son projet.
        project.clippy();
    }

    #[test]
    fn the_render_needs_no_pass_of_rustfmt() {
        for (name, fields) in [
            ("tag", "title:string"),
            ("articles", CHAMPS),
            ("administrative_documents", "title:string,views:int"),
            ("tokens", ""),
        ] {
            let rendered = seed(name, fields);

            assert_eq!(
                bench::formatted(&rendered),
                rendered,
                "un `cargo fmt` reformaterait le seed de {name}"
            );
        }
    }
}
