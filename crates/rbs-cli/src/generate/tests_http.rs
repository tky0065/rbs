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
///
/// Une référence requise écarte les scénarios qui créent : ils POSTeraient un identifiant
/// inventé dans une colonne sous contrainte de clé étrangère, et rendraient 500 dès la
/// première exécution. Le fichier garde ce qui ne crée rien, et dit ce qui manque — le
/// seed s'écarte entièrement pour la même raison.
///
/// Un garde de rôle les écarte de même : ces scénarios n'émettent aucun jeton, et le
/// projet neuf échouerait à son propre `cargo test`. Le fichier éprouve alors le refus
/// d'une écriture anonyme, qui est ce que le garde promet.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    let blocking = feature.required_reference();
    let creatable = blocking.is_none() && feature.role.is_none();
    // Sans création, aucun champ n'est envoyé ni comparé : les aides qui les servent
    // resteraient inutilisées, et le projet engendré ne compile pas sous `-D warnings`.
    let sent: &[Field] = if creatable { &feature.fields } else { &[] };
    let fields: Vec<TestField> = sent.iter().map(TestField::from).collect();

    Renderer::new().render(
        TESTS,
        context! {
            module => feature.module(),
            creatable,
            role => feature.role,
            blocking_reference => blocking.map(|field| field.relation_name()),
            fields => fields,
            compared => names(sent, |champ| !timestamp(champ)),
            timestamped => names(sent, timestamp),
            suffix => sent.iter().any(textual),
            unique_number => sent.iter().any(drawn_number),
            // Le critère du scénario de filtrage : un champ dont la valeur envoyée se
            // rejoue telle quelle. Un horodatage en est écarté — PostgreSQL le rend dans
            // un autre format que la chaîne envoyée, et l'égalité porterait à faux.
            filterable => sent.iter().find(|champ| filterable(champ)).map(Field::column_name),
            // Les deux scénarios ci-dessous n'ont de sens que si `--fields` les rend
            // atteignables : sans contrainte d'e-mail rien ne rend 422, sans colonne
            // unique rien ne rend 409, et le test échouerait faute de refus à observer.
            email_field => sent.iter().find(|champ| champ.validates_email()).map(Field::column_name),
            unique_field => sent.iter().any(|champ| champ.unique),
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
            name: champ.column_name(),
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
    // Une référence optionnelle part à `null` : un identifiant tiré au hasard ne désigne
    // aucune ligne de la table visée, et la clé étrangère refuserait la création. Une
    // référence requise n'arrive jamais ici — elle a déjà écarté les scénarios qui créent.
    if champ.reference().is_some() {
        return "Value::Null".to_string();
    }

    // Une colonne `unique` non textuelle ne peut pas porter de valeur écrite : les
    // scénarios de ce fichier créent en parallèle sur la même base, et se refuseraient
    // l'un l'autre par un 409 dès la première exécution. Les textes tiennent déjà
    // l'invariant par leur suffixe, et un UUID se tire déjà à chaque appel.
    if drawn_number(champ) {
        return match champ.column_type() {
            FieldType::Int => "unique_number() as i32".to_string(),
            FieldType::Float => "unique_number() as f64 / 10.0".to_string(),
            _ => "(chrono::Utc::now() + chrono::Duration::microseconds(unique_number()))\n            .to_rfc3339()"
                .to_string(),
        };
    }

    match champ.column_type() {
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
    champ.column_type() == FieldType::Datetime
}

/// Un champ sur lequel le scénario de filtrage peut porter.
///
/// La référence en est écartée : elle part à `null`, et un filtre d'égalité sur `null` ne
/// retiendrait rien. L'horodatage aussi : sa valeur revient dans un autre format.
fn filterable(champ: &Field) -> bool {
    champ.reference().is_none() && !timestamp(champ)
}

/// Un scalaire `unique` dont la valeur d'exemple se tire au lieu de s'écrire.
///
/// Un booléen n'y figure pas : `--fields` refuse d'y poser « unique », faute de pouvoir
/// tenir plus de deux lignes dans la table.
fn drawn_number(champ: &Field) -> bool {
    champ.unique
        && champ.reference().is_none()
        && matches!(
            champ.column_type(),
            FieldType::Int | FieldType::Float | FieldType::Datetime
        )
}

fn textual(champ: &Field) -> bool {
    matches!(champ.column_type(), FieldType::String | FieldType::Text)
}

fn names(fields: &[Field], retenu: impl Fn(&Field) -> bool) -> Vec<String> {
    fields
        .iter()
        .filter(|champ| retenu(champ))
        .map(Field::column_name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, fields, migration};

    const CHAMPS: &str = "title:string,email:string:unique,summary:text:optional,views:int,\
                          note:float,published:bool,auteur_id:uuid,published_at:datetime";

    fn trials(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("champs valides");
        render(&Feature::fresh(name, fields)).expect("les tests doivent se rendre")
    }

    /// Trois bascules traversent ce fichier, toutes régies par les soixante colonnes de
    /// `fn_call_width` : deux macros d'assertion dont les arguments débordent quel que soit
    /// le nom — elles sont écrites éclatées — et les deux appels qui portent le module en
    /// dur, qui basculent à dix-huit et vingt-huit caractères.
    ///
    /// Au-delà de trente-trois, l'appel intérieur `request(…)` déborde à son tour et
    /// rustfmt l'éclate à un second niveau. Reproduire cet emboîtement reviendrait à
    /// réimplanter une répartition qu'une montée de rustfmt peut déplacer ;
    /// `format::format_batch` la rattrape à l'écriture, donc rien de mal formé n'atteint
    /// l'utilisateur. C'est cette frontière que l'intervalle fixe — mesurée, et non
    /// commentée.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| trials(name, CHAMPS));

        assert_eq!(
            divergentes,
            (34..=40).collect::<Vec<usize>>(),
            "la plage où les tests HTTP divergent de rustfmt a bougé"
        );
    }

    #[test]
    fn the_scenarios_are_declared() {
        let rendered = trials("articles", CHAMPS);

        // `CHAMPS` porte `email:string:unique` et des champs filtrables : les quatre
        // scénarios conditionnels y sont donc attendus, avec les quatre que toute feature
        // créable emporte.
        let scenarios = [
            "async fn the_full_lifecycle_goes_through_the_api()",
            // L'identifiant est posé par le modèle depuis que `uuidv7()` a quitté la
            // migration : la croissance des identifiants se prouve dans le projet.
            "async fn two_creations_in_a_row_carry_increasing_ids()",
            "async fn an_invalid_email_returns_422()",
            "async fn the_filter_narrows_the_list()",
            "async fn an_unknown_sort_column_returns_400()",
            "async fn a_replayed_unique_value_returns_409()",
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
    fn a_second_delete_is_exercised() {
        // `unique` allume le quatrième scénario de suppression (`a_replayed_unique_value_
        // returns_409`) : sans lui, `title` ne serait filtrable que par le troisième, et le
        // compte plafonnerait à 4 au lieu des 5 attendus.
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        );
        let rendered = render(&feature).expect("les tests doivent se rendre");

        assert_eq!(
            rendered
                .matches(r#"without_body("DELETE", &resource)"#)
                .count(),
            5,
            "chaque scénario de suppression doit rejouer le DELETE : c'est la seule \
             assertion qui distingue une suppression logique bien gardée d'une qui \
             rendrait 204 deux fois :\n{rendered}"
        );
    }

    /// `ValidatedJson` existe pour ce chemin : un corps lisible mais non conforme rend
    /// 422, là où un corps illisible rend 400. Rien ne l'éprouvait.
    #[test]
    fn an_email_field_earns_a_422_scenario() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("async fn an_invalid_email_returns_422()"),
            "le scénario 422 est absent :\n{rendered}"
        );
        assert!(
            rendered.contains("StatusCode::UNPROCESSABLE_ENTITY"),
            "le statut attendu doit être 422 :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"body["errors"]["email"]"#),
            "le refus doit nommer le champ fautif :\n{rendered}"
        );
    }

    /// Sans champ `unique`, un rejeu ne provoque aucun conflit : le scénario n'aurait
    /// rien à observer et échouerait.
    #[test]
    fn a_unique_field_earns_a_409_scenario() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("async fn a_replayed_unique_value_returns_409()"),
            "le scénario 409 est absent :\n{rendered}"
        );
        assert!(
            rendered.contains("StatusCode::CONFLICT"),
            "le statut attendu doit être 409 :\n{rendered}"
        );
    }

    /// Les deux scénarios sont conditionnés par ce que `--fields` demande.
    #[test]
    fn a_feature_without_email_or_unique_carries_neither_scenario() {
        let rendered = trials("articles", "title:string,body:text,published:bool");

        assert!(
            !rendered.contains("an_invalid_email_returns_422"),
            "aucun champ ne porte de contrainte d'e-mail :\n{rendered}"
        );
        assert!(
            !rendered.contains("a_replayed_unique_value_returns_409"),
            "aucun champ n'est unique :\n{rendered}"
        );
    }

    /// Sans création possible, les deux scénarios tombent avec les autres.
    #[test]
    fn a_required_reference_also_drops_the_422_and_409_scenarios() {
        let rendered = trials("posts", "email:string:unique,author:references:users");

        assert!(
            !rendered.contains("an_invalid_email_returns_422"),
            "{rendered}"
        );
        assert!(
            !rendered.contains("a_replayed_unique_value_returns_409"),
            "{rendered}"
        );
    }

    /// Les tests tournent sur la base de développement, sans transaction : ce qu'ils
    /// créent, ils le suppriment, faute de quoi la table enfle à chaque `cargo test`.
    #[test]
    fn the_uuid_scenario_deletes_what_it_created() {
        let rendered = trials("articles", CHAMPS);

        let scenario = rendered
            .split("async fn two_creations_in_a_row_carry_increasing_ids()")
            .nth(1)
            .and_then(|reste| reste.split("\n#[tokio::test]").next())
            .expect("le scénario doit être rendu");

        assert!(
            scenario.contains("for identifiant in [premier, second]"),
            "les deux lignes créées doivent être parcourues :\n{scenario}"
        );
        assert!(
            scenario.contains(r#"without_body("DELETE", &resource)"#),
            "les deux lignes créées doivent être supprimées :\n{scenario}"
        );
    }

    /// Tout scénario de ce fichier monte l'application sur la base du `.env` : sans
    /// `#[ignore]`, `cargo test` échouerait sur un projet neuf dont la base n'est pas
    /// démarrée, là où les fragments installés par `add` s'en gardent tous.
    #[test]
    fn every_scenario_is_ignored() {
        let rendered = trials("articles", CHAMPS);

        assert_eq!(
            rendered
                .matches(r#"#[ignore = "joint la base du projet"]"#)
                .count(),
            rendered.matches("#[tokio::test]").count(),
            "chaque scénario doit être ignoré sans base :\n{rendered}"
        );
    }

    /// Le doc-commentaire de `value` promet qu'un champ `unique` ne fera pas échouer une
    /// exécution sur ce qu'une autre a laissé. Une valeur en dur ne le tient pas : les
    /// scénarios de ce fichier créent en parallèle sur la même base, et se refuseraient
    /// l'un l'autre par un 409 dès la première exécution.
    #[test]
    fn a_unique_number_is_drawn_at_each_call() {
        let rendered = trials(
            "articles",
            "views:int:unique,note:float:unique,vu_le:datetime:unique",
        );

        assert!(
            rendered.contains("fn unique_number() -> i64"),
            "l'aide qui tire le nombre est absente :\n{rendered}"
        );
        for valeur in [
            r#""views": unique_number() as i32"#,
            r#""note": unique_number() as f64 / 10.0"#,
        ] {
            assert!(
                rendered.contains(valeur),
                "« {valeur} » absent :\n{rendered}"
            );
        }
        assert!(
            rendered.contains("chrono::Duration::microseconds(unique_number())"),
            "l'horodatage unique doit se décaler :\n{rendered}"
        );
        for en_dur in ["\"views\": 42", "\"views\": 43", "\"note\": 4.2"] {
            assert!(
                !rendered.contains(en_dur),
                "« {en_dur} » se rejouerait d'une exécution à l'autre :\n{rendered}"
            );
        }
    }

    /// Un scalaire qui n'est pas `unique` garde une valeur lisible : le code engendré est
    /// fait pour être lu et modifié.
    #[test]
    fn an_ordinary_number_keeps_its_readable_value() {
        let rendered = trials("articles", CHAMPS);

        assert!(rendered.contains(r#""views": 42"#), "{rendered}");
        assert!(rendered.contains(r#""views": 43"#), "{rendered}");
        assert!(
            !rendered.contains("unique_number()"),
            "aucun champ numérique n'est unique ici :\n{rendered}"
        );
    }

    /// Le filtre se prouve par la route, et non par le rendu : une condition mal traduite
    /// rend une page vide, ce qu'aucune comparaison de chaînes ne verrait.
    #[test]
    fn a_filter_scenario_is_generated_when_a_field_can_carry_one() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            rendered.contains("async fn the_filter_narrows_the_list()"),
            "le scénario de filtre est absent :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"let chemin = format!("{collection}/filter");"#),
            "le scénario doit appeler la route de filtre :\n{rendered}"
        );
        assert!(
            rendered.contains("async fn an_unknown_sort_column_returns_400()"),
            "le refus d'une colonne de tri inconnue n'est pas éprouvé :\n{rendered}"
        );
        // `clippy::useless_format` refuse un `format!` sans interpolation, et le projet
        // engendré compile sous `-D warnings`.
        assert!(
            !rendered.contains(r#"format!("/articles/filter")"#),
            "un chemin constant s'écrit sans `format!` :\n{rendered}"
        );
    }

    /// Sans champ dont la valeur se rejoue, le scénario n'aurait pas de critère : une
    /// référence part à `null` et un horodatage revient dans un autre format.
    #[test]
    fn a_feature_without_a_usable_criterion_carries_no_filter_scenario() {
        let rendered = trials("articles", "vu_le:datetime");

        assert!(
            !rendered.contains("the_filter_narrows_the_list"),
            "aucun champ ne peut porter le critère :\n{rendered}"
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
            r#"let premiere = format!("{collection}?per_page=50");"#,
            r#"request("PATCH", &resource, sent.clone())"#,
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

    /// Une référence est nommée par sa colonne, pas par le nom déclaré : le DTO n'a pas
    /// de champ `author`, seulement `author_id`.
    #[test]
    fn a_reference_is_named_by_its_column_not_its_relation_name() {
        let rendered = trials("posts", "author:references:users:optional");

        assert!(
            rendered.contains(r#""author_id": Value::Null,"#),
            "la colonne doit être nommée « author_id » :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"compare(&created, &sent, "author_id");"#),
            "la comparaison doit porter sur la colonne :\n{rendered}"
        );
        assert!(
            !rendered.contains(r#""author":"#),
            "le nom déclaré ne doit pas fuir :\n{rendered}"
        );
    }

    /// Un identifiant tiré au hasard ne désigne aucune ligne de la table visée : la clé
    /// étrangère refuserait la création, et le test rendrait 500 au lieu de 201.
    #[test]
    fn an_optional_reference_is_sent_null_rather_than_at_random() {
        let rendered = trials("posts", "title:string,author:references:users:optional");

        assert!(
            !rendered.contains(r#""author_id": Uuid::new_v4()"#),
            "un identifiant inventé violerait la clé étrangère :\n{rendered}"
        );
        assert!(
            rendered.contains(r#""author_id": Value::Null,"#),
            "la référence optionnelle part à null :\n{rendered}"
        );
    }

    /// Une référence requise ne peut pas partir à `null` : les scénarios qui créent sont
    /// écartés, et le fichier dit lequel des champs les a écartés.
    #[test]
    fn a_required_reference_drops_the_scenarios_that_create() {
        let rendered = trials("posts", "title:string,author:references:users");

        for absent in [
            "async fn the_full_lifecycle_goes_through_the_api()",
            "async fn two_creations_in_a_row_carry_increasing_ids()",
            "fn creation()",
            "fn modification()",
            "fn request(",
            "fn compare(",
        ] {
            assert!(
                !rendered.contains(absent),
                "« {absent} » suppose une création :\n{rendered}"
            );
        }

        for present in [
            "async fn an_unknown_id_returns_404()",
            "async fn an_unreadable_body_returns_400()",
        ] {
            assert!(
                rendered.contains(present),
                "« {present} » ne crée rien et doit rester :\n{rendered}"
            );
        }
    }

    #[test]
    fn a_required_reference_says_in_the_file_what_is_missing() {
        let rendered = trials("posts", "title:string,author:references:users");

        assert!(
            rendered.contains("« author »"),
            "la référence qui bloque doit être nommée :\n{rendered}"
        );
        assert!(
            rendered.contains("// "),
            "l'explication doit tenir en commentaire :\n{rendered}"
        );
    }

    /// Le fichier réduit reste compilable : `json!` n'y sert plus, et un import inutilisé
    /// ferait échouer le `cargo test` du projet engendré sous `-D warnings`.
    #[test]
    fn the_reduced_file_imports_only_what_it_uses() {
        let rendered = trials("posts", "title:string,author:references:users");

        assert!(
            !rendered.contains("json!"),
            "plus aucun corps n'est construit :\n{rendered}"
        );
        assert!(
            rendered.contains("use serde_json::Value;"),
            "`Value` sert encore au retour de `call` :\n{rendered}"
        );
        assert!(
            rendered.contains("use uuid::Uuid;"),
            "`Uuid` sert encore au scénario 404 :\n{rendered}"
        );
    }

    /// Le critère du lot : les tests générés passent sans retouche.
    ///
    /// Rien d'autre ne le prouve — un rendu qui contient les bonnes chaînes peut encore
    /// interroger une route qui n'existe pas, ou comparer une valeur que PostgreSQL rend
    /// autrement. Seul le projet compilé contre une vraie base tranche.
    #[test]
    #[ignore = "démarre PostgreSQL en conteneur et compile un projet complet"]
    fn the_generated_tests_pass_untouched() {
        const HORODATAGE: &str = "20260826_090000";

        let fields = fields::parse(CHAMPS).expect("champs valides");
        let feature = Feature::fresh("billets", fields);
        let base = bench::TestDatabase::start();

        let project = bench::Project::fresh_on(base.url());
        project.write_feature("billets", &bench::tous(&feature, true));
        project.mount_feature("billets");

        let migration = migration::render(&feature, HORODATAGE).expect("migration rendue");
        project.write_migration(&migration.module, &migration.content);
        project.migrate(base.url());

        project.test_of();
    }
}
