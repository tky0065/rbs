use super::*;

use std::time::Instant;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// La garde tient avant même que le service existe : `Identity` refuse la requête sans
/// jamais atteindre le controller.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_without_a_token_returns_401() {
    let api = application().await;

    let (status, body) = call(&api, without_body("GET", "/auth/me")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

/// Un jeton que le service n'a pas signé ne vaut pas mieux qu'aucun jeton.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_with_an_unreadable_token_returns_401() {
    let api = application().await;
    let requete = Request::builder()
        .method("GET")
        .uri("/auth/me")
        .header("authorization", "Bearer pas-un-jeton")
        .body(Body::empty())
        .expect("requête bien formée");

    let (status, body) = call(&api, requete).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

/// Une adresse non vérifiée ne se connecte pas, et reçoit le 401 d'un mauvais mot de
/// passe : `application` pose `login_requires_verification` à `true`. La preuve d'adresse
/// l'ouvre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unverified_address_logs_in_only_after_verification() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (refus, corps) = authenticate(&api, &email, PASSWORD).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{corps}");

    verified(&email).await;
    login(&api, &email, PASSWORD).await;
}

/// `register` suivi de `login` ne dit plus si l'adresse était libre : la neuve n'est pas
/// vérifiée, la prise ne porte pas ce mot de passe, et les deux rendent le même 401.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn register_then_login_answers_the_same_for_a_free_and_a_taken_address() {
    const TITULAIRE: &str = "le mot de passe du titulaire";

    let api = application().await;
    let libre = fresh_email();
    let prise = fresh_email();
    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/register",
            json!({ "email": prise, "password": TITULAIRE }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");
    verified(&prise).await;

    let mut reponses = Vec::new();
    for adresse in [&libre, &prise] {
        register(&api, adresse).await;
        let (statut, mut corps) = authenticate(&api, adresse, PASSWORD).await;
        assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
        if let Some(objet) = corps.as_object_mut() {
            objet.remove("request_id");
        }
        reponses.push(corps);
    }

    assert_eq!(
        reponses[0], reponses[1],
        "la connexion distingue une adresse libre d'une prise"
    );
}

/// `login_requires_verification = false` connecte dès l'inscription : le choix d'un projet
/// qui accepte l'écart que la clé ferme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn without_the_rule_an_unverified_address_logs_in() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();
    register(&api, &email).await;

    let config = rbs_core::Config::load().expect("configuration lisible");
    let flows = crate::auth::config::FlowConfig {
        login_requires_verification: false,
        ..Default::default()
    };
    let requete = crate::auth::dto::LoginRequest {
        email,
        password: PASSWORD.to_owned(),
    };

    crate::auth::service::login(&db, &config.auth, &flows, requete)
        .await
        .expect("la connexion aboutit sans preuve d'adresse");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn login_ignores_the_case_of_the_address() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (status, paire) = authenticate(&api, &email.to_uppercase(), PASSWORD).await;

    assert_eq!(status, StatusCode::OK, "{paire}");
}

/// Les deux échecs sont indiscernables : un corps qui différerait dirait à un attaquant
/// quelles adresses sont inscrites.
///
/// `request_id` est retiré avant la comparaison — il change à chaque requête par
/// construction, et corréler une réponse à une ligne de journal ne renseigne personne sur
/// l'existence d'un compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_wrong_password_and_an_unknown_email_return_the_same_401() {
    let api = application().await;
    let inscrit = fresh_email();
    signed_up(&api, &inscrit).await;

    let (statut_faux, mut corps_faux) =
        authenticate(&api, &inscrit, "un tout autre mot de passe").await;
    let (statut_inconnu, mut corps_inconnu) = authenticate(&api, &fresh_email(), PASSWORD).await;

    assert_eq!(statut_faux, StatusCode::UNAUTHORIZED, "{corps_faux}");
    assert_eq!(statut_inconnu, StatusCode::UNAUTHORIZED, "{corps_inconnu}");

    for body in [&mut corps_faux, &mut corps_inconnu] {
        if let Some(objet) = body.as_object_mut() {
            objet.remove("request_id");
        }
    }

    assert_eq!(
        corps_faux, corps_inconnu,
        "les deux échecs doivent être indiscernables"
    );
}

/// Le temps de réponse ne renseigne pas davantage que le corps.
///
/// Sans vérification sur le chemin « adresse inconnue », la réponse tombe en une fraction
/// de milliseconde là où Argon2 en coûte plusieurs dizaines : cet écart seul énumère les
/// comptes. Le rapport toléré est large — il sépare l'absence de vérification, d'un ordre
/// de grandeur, d'un hachage à vide, qui coûte le même temps.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_email_costs_the_same_time_as_a_wrong_password() {
    let api = application().await;
    let inscrit = fresh_email();
    signed_up(&api, &inscrit).await;

    // Un tour à vide : le hash de comparaison se calcule au premier passage, et son coût
    // ne doit pas être imputé à la mesure.
    authenticate(&api, &fresh_email(), PASSWORD).await;

    let depart = Instant::now();
    authenticate(&api, &inscrit, "un tout autre mot de passe").await;
    let mot_de_passe_faux = depart.elapsed();

    let depart = Instant::now();
    authenticate(&api, &fresh_email(), PASSWORD).await;
    let email_inconnu = depart.elapsed();

    assert!(
        email_inconnu * 5 >= mot_de_passe_faux,
        "une adresse inconnue répond en {email_inconnu:?} contre {mot_de_passe_faux:?} \
         pour un mot de passe faux : l'écart énumère les comptes"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_returns_the_callers_profile() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;
    let inscrit = account(&email).await;
    let (_, paire) = authenticate(&api, &email, PASSWORD).await;

    let (status, profile) = call(&api, with_token("GET", "/auth/me", &access_for(&paire))).await;

    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["id"], inscrit.id.to_string(), "{profile}");
    assert_eq!(profile["email"], email, "{profile}");
    assert_eq!(profile["role"], "user", "{profile}");
}
