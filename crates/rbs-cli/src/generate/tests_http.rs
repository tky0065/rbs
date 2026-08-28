//! Rendu de `<name>/tests.rs` : le CRUD complet exercé par HTTP.
//!
//! Le module se nomme `trials` et non `tests` : `generate::tests` se confondrait avec les
//! modules `#[cfg(test)]` que porte chaque générateur.

use minijinja::context;
use serde::Serialize;

use crate::template::Renderer;

use super::feature::Feature;
use super::fields::{Field, FieldType};

const TESTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/tests.rs.jinja"
));

/// Rend les tests d'intégration HTTP de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    let fields: Vec<TestField> = feature.fields.iter().map(TestField::from).collect();

    Renderer::new().render(
        TESTS,
        context! {
            module => feature.module(),
            fields => fields,
            compared => names(&feature.fields, |champ| !timestamp(champ)),
            timestamped => names(&feature.fields, timestamp),
            suffix => feature.fields.iter().any(textual),
        },
    )
}

/// Un champ vu par les tests : la valeur qu'ils envoient, et celle qu'ils réenvoient.
#[derive(Serialize)]
struct TestField {
    name: String,
    creation: String,
    modification: String,
}

impl TestField {
    fn from(champ: &Field) -> Self {
        Self {
            name: champ.name.clone(),
            creation: value(champ, ""),
            modification: value(champ, "modifie-"),
        }
    }
}

/// Expression Rust d'une valeur d'exemple pour `champ`.
///
/// Les valeurs textuelles portent un suffixe tiré au sort : sans lui, un champ `unique`
/// ferait échouer la seconde exécution des tests sur la première ligne restée en base.
fn value(champ: &Field, mark: &str) -> String {
    match champ.type_ {
        FieldType::String | FieldType::Text if champ.validates_email() => {
            format!("format!(\"{}-{mark}{{suffix}}@example.com\")", champ.name)
        }
        FieldType::String | FieldType::Text => {
            format!("format!(\"{}-{mark}{{suffix}}\")", champ.name)
        }
        FieldType::Int => if_modified(mark, "42", "43"),
        FieldType::Float => if_modified(mark, "4.2", "8.4"),
        FieldType::Bool => if_modified(mark, "true", "false"),
        FieldType::Uuid => "Uuid::new_v4().to_string()".to_string(),
        FieldType::Datetime => "chrono::Utc::now().to_rfc3339()".to_string(),
    }
}

fn if_modified(mark: &str, creation: &str, modification: &str) -> String {
    if mark.is_empty() {
        creation
    } else {
        modification
    }
    .to_string()
}

/// Un horodatage revient de PostgreSQL dans un autre format que celui envoyé : sa valeur
/// ne se compare pas, seule sa présence se vérifie.
fn timestamp(champ: &Field) -> bool {
    champ.type_ == FieldType::Datetime
}

fn textual(champ: &Field) -> bool {
    matches!(champ.type_, FieldType::String | FieldType::Text)
}

fn names(fields: &[Field], retenu: impl Fn(&Field) -> bool) -> Vec<&str> {
    fields
        .iter()
        .filter(|champ| retenu(champ))
        .map(|champ| champ.name.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, controller, dto, entity, fields, migration, repository, service};

    const CHAMPS: &str = "title:string,email:string:unique,summary:text:optional,views:int,\
                          note:float,published:bool,auteur_id:uuid,published_at:datetime";

    fn trials(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("les tests doivent se rendre")
    }

    #[test]
    fn the_four_scenarios_are_declared() {
        let rendered = trials("articles", CHAMPS);

        let scenarios = [
            "async fn the_full_lifecycle_goes_through_the_api()",
            // L'identifiant est posé par le modèle depuis que `uuidv7()` a quitté la
            // migration : la croissance des identifiants se prouve dans le projet.
            "async fn two_creations_in_a_row_carry_increasing_ids()",
            "async fn an_unknown_id_returns_404()",
            "async fn an_unreadable_body_returns_400()",
        ];

        for signature in scenarios {
            assert!(
                rendered.contains(signature),
                "« {signature} » absent :\n{rendered}"
            );
        }
        assert_eq!(
            rendered.matches("#[tokio::test]").count(),
            scenarios.len(),
            "chaque scénario est un test asynchrone :\n{rendered}"
        );
    }

    #[test]
    fn the_application_is_mounted_in_memory() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("router(AppState::new(db, config)"),
            "l'application doit être construite comme au démarrage :\n{rendered}"
        );
        assert!(
            rendered.contains(".oneshot(request)"),
            "les requêtes doivent traverser le routeur sans réseau :\n{rendered}"
        );
        assert!(
            !rendered.contains("TcpListener") && !rendered.contains("axum::serve"),
            "aucun serveur ne doit être lancé :\n{rendered}"
        );
    }

    #[test]
    fn the_lifecycle_exercises_the_five_routes_and_their_statuses() {
        let rendered = trials("blog_posts", CHAMPS);

        for appel in [
            r#"let collection = "/blog_posts";"#,
            r#"request("POST", collection, sent.clone())"#,
            r#"let resource = format!("{collection}/{id}");"#,
            r#"without_body("GET", &resource)"#,
            r#"let premiere = format!("{collection}?per_page=1");"#,
            r#"request("PUT", &resource, sent.clone())"#,
            r#"without_body("DELETE", &resource)"#,
        ] {
            assert!(rendered.contains(appel), "« {appel} » absent :\n{rendered}");
        }

        for statut in [
            "StatusCode::CREATED",
            "StatusCode::OK",
            "StatusCode::NO_CONTENT",
            "StatusCode::NOT_FOUND",
        ] {
            assert!(
                rendered.contains(statut),
                "« {statut} » absent :\n{rendered}"
            );
        }
    }

    #[test]
    fn each_textual_value_carries_a_unique_suffix() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("let suffix = Uuid::new_v4();"),
            "le suffixe rend chaque exécution indépendante de la précédente :\n{rendered}"
        );
        assert!(
            rendered.contains(r#""title": format!("title-{suffix}")"#),
            "le title doit porter le suffixe :\n{rendered}"
        );
        assert!(
            rendered.contains(r#""title": format!("title-modifie-{suffix}")"#),
            "la mise à jour doit envoyer une autre valeur :\n{rendered}"
        );
    }

    #[test]
    fn an_email_field_receives_a_valid_address() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains(r#""email": format!("email-{suffix}@example.com")"#),
            "la contrainte d'email refuserait toute autre valeur :\n{rendered}"
        );
    }

    #[test]
    fn each_type_receives_a_value_of_its_own_type() {
        let rendered = trials("articles", CHAMPS);

        for value in [
            r#""views": 42"#,
            r#""note": 4.2"#,
            r#""published": true"#,
            r#""auteur_id": Uuid::new_v4().to_string()"#,
            r#""published_at": chrono::Utc::now().to_rfc3339()"#,
        ] {
            assert!(rendered.contains(value), "« {value} » absent :\n{rendered}");
        }
    }

    #[test]
    fn the_update_sends_a_value_different_from_the_creation() {
        let rendered = trials("articles", CHAMPS);

        for value in [r#""views": 43"#, r#""note": 8.4"#, r#""published": false"#] {
            assert!(rendered.contains(value), "« {value} » absent :\n{rendered}");
        }
    }

    #[test]
    fn the_comparable_fields_are_compared_and_the_timestamps_are_not() {
        let rendered = trials("articles", CHAMPS);

        for champ in [
            "title",
            "email",
            "summary",
            "views",
            "note",
            "published",
            "auteur_id",
        ] {
            assert!(
                rendered.contains(&format!(r#"compare(&created, &sent, "{champ}");"#)),
                "« {champ} » doit être comparé à ce qui a été envoyé :\n{rendered}"
            );
        }
        assert!(
            !rendered.contains(r#"compare(&created, &sent, "published_at");"#),
            "PostgreSQL ne rend pas l'horodatage dans le format envoyé :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"filled(&created, "published_at");"#),
            "l'horodatage doit au moins être rendu :\n{rendered}"
        );
    }

    #[test]
    fn a_feature_without_a_timestamp_carries_no_presence_assertion() {
        let rendered = trials("articles", "title:string");

        assert!(
            !rendered.contains("fn filled("),
            "une aide inutilisée laisserait un avertissement :\n{rendered}"
        );
        assert!(rendered.contains("fn compare("), "{rendered}");
    }

    #[test]
    fn a_field_less_feature_carries_no_unused_helper() {
        let rendered = trials("articles", "");

        assert!(!rendered.contains("fn compare("), "{rendered}");
        assert!(!rendered.contains("fn filled("), "{rendered}");
        assert!(!rendered.contains("let suffixe ="), "{rendered}");
        assert!(
            rendered.contains("json!({})"),
            "le corps de création reste un objet vide :\n{rendered}"
        );
    }

    /// Le critère du lot : les tests générés passent sans retouche.
    ///
    /// Rien d'autre ne le prouve — un rendu qui contient les bonnes chaînes peut encore
    /// interroger une route qui n'existe pas, ou comparer une valeur que PostgreSQL rend
    /// autrement. Seul le projet compilé contre une vraie base tranche.
    #[test]
    #[ignore = "démarre PostgreSQL 18 en conteneur et compile un projet complet"]
    fn the_generated_tests_pass_untouched() {
        const HORODATAGE: &str = "20260826_090000";

        let fields = fields::parse(CHAMPS).expect("champs valides");
        let feature = Feature::fresh("billets", fields);
        let base = bench::TestDatabase::start();

        let project = bench::Project::fresh_on(base.url());
        project.write_feature(
            "billets",
            &[
                (
                    "mod.rs",
                    &controller::render_mod(&feature, true).expect("mod.rs rendu"),
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
                    &controller::render(&feature).expect("controller rendu"),
                ),
                ("tests.rs", &render(&feature).expect("tests rendus")),
            ],
        );
        project.mount_feature("billets");

        let migration = migration::render(&feature, HORODATAGE).expect("migration rendue");
        project.write_migration(&migration.module, &migration.content);
        project.migrate(base.url());

        project.test_of();
    }
}
