//! Rendu de `<nom>/tests.rs` : le CRUD complet exercé par HTTP.
//!
//! Le module se nomme `essais` et non `tests` : `generate::tests` se confondrait avec les
//! modules `#[cfg(test)]` que porte chaque générateur.

use minijinja::context;
use serde::Serialize;

use crate::template::Renderer;

use super::champs::{Champ, TypeChamp};
use super::feature::Feature;

const TESTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/tests.rs.jinja"
));

/// Rend les tests d'intégration HTTP de `feature`.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    let champs: Vec<ChampDeTest> = feature.champs.iter().map(ChampDeTest::depuis).collect();

    Renderer::new().rendre(
        TESTS,
        context! {
            module => feature.module(),
            champs => champs,
            compares => noms(&feature.champs, |champ| !horodatage(champ)),
            horodates => noms(&feature.champs, horodatage),
            suffix => feature.champs.iter().any(textuel),
        },
    )
}

/// Un champ vu par les tests : la valeur qu'ils envoient, et celle qu'ils réenvoient.
#[derive(Serialize)]
struct ChampDeTest {
    nom: String,
    creation: String,
    modification: String,
}

impl ChampDeTest {
    fn depuis(champ: &Champ) -> Self {
        Self {
            nom: champ.nom.clone(),
            creation: valeur(champ, ""),
            modification: valeur(champ, "modifie-"),
        }
    }
}

/// Expression Rust d'une valeur d'exemple pour `champ`.
///
/// Les valeurs textuelles portent un suffixe tiré au sort : sans lui, un champ `unique`
/// ferait échouer la seconde exécution des tests sur la première ligne restée en base.
fn valeur(champ: &Champ, marque: &str) -> String {
    match champ.type_ {
        TypeChamp::String | TypeChamp::Text if champ.valide_email() => {
            format!("format!(\"{}-{marque}{{suffix}}@example.com\")", champ.nom)
        }
        TypeChamp::String | TypeChamp::Text => {
            format!("format!(\"{}-{marque}{{suffix}}\")", champ.nom)
        }
        TypeChamp::Int => si_modifie(marque, "42", "43"),
        TypeChamp::Float => si_modifie(marque, "4.2", "8.4"),
        TypeChamp::Bool => si_modifie(marque, "true", "false"),
        TypeChamp::Uuid => "Uuid::new_v4().to_string()".to_string(),
        TypeChamp::Datetime => "chrono::Utc::now().to_rfc3339()".to_string(),
    }
}

fn si_modifie(marque: &str, creation: &str, modification: &str) -> String {
    if marque.is_empty() {
        creation
    } else {
        modification
    }
    .to_string()
}

/// Un horodatage revient de PostgreSQL dans un autre format que celui envoyé : sa valeur
/// ne se compare pas, seule sa présence se vérifie.
fn horodatage(champ: &Champ) -> bool {
    champ.type_ == TypeChamp::Datetime
}

fn textuel(champ: &Champ) -> bool {
    matches!(champ.type_, TypeChamp::String | TypeChamp::Text)
}

fn noms(champs: &[Champ], retenu: impl Fn(&Champ) -> bool) -> Vec<&str> {
    champs
        .iter()
        .filter(|champ| retenu(champ))
        .map(|champ| champ.nom.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{banc, champs, controller, dto, entite, migration, repository, service};

    const CHAMPS: &str = "titre:string,email:string:unique,resume:text:optional,vues:int,\
                          note:float,publie:bool,auteur_id:uuid,publie_le:datetime";

    fn essais(nom: &str, fields: &str) -> String {
        let champs = champs::analyser(fields).expect("champs valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("les tests doivent se rendre")
    }

    #[test]
    fn les_trois_scenarios_sont_declares() {
        let rendu = essais("articles", CHAMPS);

        for signature in [
            "async fn the_full_lifecycle_goes_through_the_api()",
            "async fn an_unknown_id_returns_404()",
            "async fn an_unreadable_body_returns_400()",
        ] {
            assert!(
                rendu.contains(signature),
                "« {signature} » absent :\n{rendu}"
            );
        }
        assert_eq!(
            rendu.matches("#[tokio::test]").count(),
            3,
            "chaque scénario est un test asynchrone :\n{rendu}"
        );
    }

    #[test]
    fn l_application_est_montee_en_memoire() {
        let rendu = essais("articles", CHAMPS);

        assert!(
            rendu.contains("router(AppState::new(db, config)"),
            "l'application doit être construite comme au démarrage :\n{rendu}"
        );
        assert!(
            rendu.contains(".oneshot(request)"),
            "les requêtes doivent traverser le routeur sans réseau :\n{rendu}"
        );
        assert!(
            !rendu.contains("TcpListener") && !rendu.contains("axum::serve"),
            "aucun serveur ne doit être lancé :\n{rendu}"
        );
    }

    #[test]
    fn le_cycle_de_vie_exerce_les_cinq_routes_et_leurs_statuts() {
        let rendu = essais("blog_posts", CHAMPS);

        for appel in [
            r#"let collection = "/blog_posts";"#,
            r#"request("POST", collection, sent.clone())"#,
            r#"let resource = format!("{collection}/{id}");"#,
            r#"without_body("GET", &resource)"#,
            r#"let premiere = format!("{collection}?per_page=1");"#,
            r#"request("PUT", &resource, sent.clone())"#,
            r#"without_body("DELETE", &resource)"#,
        ] {
            assert!(rendu.contains(appel), "« {appel} » absent :\n{rendu}");
        }

        for statut in [
            "StatusCode::CREATED",
            "StatusCode::OK",
            "StatusCode::NO_CONTENT",
            "StatusCode::NOT_FOUND",
        ] {
            assert!(rendu.contains(statut), "« {statut} » absent :\n{rendu}");
        }
    }

    #[test]
    fn chaque_valeur_textuelle_porte_un_suffixe_unique() {
        let rendu = essais("articles", CHAMPS);

        assert!(
            rendu.contains("let suffix = Uuid::new_v4();"),
            "le suffixe rend chaque exécution indépendante de la précédente :\n{rendu}"
        );
        assert!(
            rendu.contains(r#""titre": format!("titre-{suffix}")"#),
            "le titre doit porter le suffixe :\n{rendu}"
        );
        assert!(
            rendu.contains(r#""titre": format!("titre-modifie-{suffix}")"#),
            "la mise à jour doit envoyer une autre valeur :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_email_recoit_une_adresse_valide() {
        let rendu = essais("articles", CHAMPS);

        assert!(
            rendu.contains(r#""email": format!("email-{suffix}@example.com")"#),
            "la contrainte d'email refuserait toute autre valeur :\n{rendu}"
        );
    }

    #[test]
    fn chaque_type_recoit_une_valeur_de_son_type() {
        let rendu = essais("articles", CHAMPS);

        for valeur in [
            r#""vues": 42"#,
            r#""note": 4.2"#,
            r#""publie": true"#,
            r#""auteur_id": Uuid::new_v4().to_string()"#,
            r#""publie_le": chrono::Utc::now().to_rfc3339()"#,
        ] {
            assert!(rendu.contains(valeur), "« {valeur} » absent :\n{rendu}");
        }
    }

    #[test]
    fn la_mise_a_jour_envoie_une_valeur_differente_de_la_creation() {
        let rendu = essais("articles", CHAMPS);

        for valeur in [r#""vues": 43"#, r#""note": 8.4"#, r#""publie": false"#] {
            assert!(rendu.contains(valeur), "« {valeur} » absent :\n{rendu}");
        }
    }

    #[test]
    fn les_champs_comparables_le_sont_et_les_horodatages_ne_le_sont_pas() {
        let rendu = essais("articles", CHAMPS);

        for champ in [
            "titre",
            "email",
            "resume",
            "vues",
            "note",
            "publie",
            "auteur_id",
        ] {
            assert!(
                rendu.contains(&format!(r#"compare(&created, &sent, "{champ}");"#)),
                "« {champ} » doit être comparé à ce qui a été envoyé :\n{rendu}"
            );
        }
        assert!(
            !rendu.contains(r#"compare(&created, &sent, "publie_le");"#),
            "PostgreSQL ne rend pas l'horodatage dans le format envoyé :\n{rendu}"
        );
        assert!(
            rendu.contains(r#"filled(&created, "publie_le");"#),
            "l'horodatage doit au moins être rendu :\n{rendu}"
        );
    }

    #[test]
    fn une_feature_sans_horodatage_ne_porte_pas_l_assertion_de_presence() {
        let rendu = essais("articles", "titre:string");

        assert!(
            !rendu.contains("fn filled("),
            "une aide inutilisée laisserait un avertissement :\n{rendu}"
        );
        assert!(rendu.contains("fn compare("), "{rendu}");
    }

    #[test]
    fn une_feature_sans_champ_ne_porte_aucune_aide_inutilisee() {
        let rendu = essais("articles", "");

        assert!(!rendu.contains("fn compare("), "{rendu}");
        assert!(!rendu.contains("fn filled("), "{rendu}");
        assert!(!rendu.contains("let suffixe ="), "{rendu}");
        assert!(
            rendu.contains("json!({})"),
            "le corps de création reste un objet vide :\n{rendu}"
        );
    }

    /// Le critère du lot : les tests générés passent sans retouche.
    ///
    /// Rien d'autre ne le prouve — un rendu qui contient les bonnes chaînes peut encore
    /// interroger une route qui n'existe pas, ou comparer une valeur que PostgreSQL rend
    /// autrement. Seul le projet compilé contre une vraie base tranche.
    #[test]
    #[ignore = "démarre PostgreSQL 18 en conteneur et compile un projet complet"]
    fn les_tests_generes_passent_sans_retouche() {
        const HORODATAGE: &str = "20260826_090000";

        let champs = champs::analyser(CHAMPS).expect("champs valides");
        let feature = Feature::nouvelle("billets", champs);
        let base = banc::BaseDeTest::demarrer();

        let projet = banc::Projet::neuf_sur(base.url());
        projet.poser_feature(
            "billets",
            &[
                (
                    "mod.rs",
                    &controller::rendre_mod(&feature, true).expect("mod.rs rendu"),
                ),
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
                    &controller::rendre(&feature).expect("controller rendu"),
                ),
                ("tests.rs", &rendre(&feature).expect("tests rendus")),
            ],
        );
        projet.monter_feature("billets");

        let migration = migration::rendre(&feature, HORODATAGE).expect("migration rendue");
        projet.poser_migration(&migration.module, &migration.contenu);
        projet.migrer(base.url());

        projet.tester();
    }
}
