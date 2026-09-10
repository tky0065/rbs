use super::*;

use axum::routing::get;

/// L'inscription ouvre un jeton de vérification : c'est ce qui fait que le courriel part
/// sans qu'aucune route ne soit appelée.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn registering_opens_a_verification_token() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;

    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");

    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        1,
        "l'inscription n'a ouvert aucun jeton"
    );
}

/// Le parcours nominal : `email_verified_at` passe de nul à daté, et `me` le montre.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn verifying_marks_the_address_and_shows_on_me() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    // `register` rend `(StatusCode, Value)` : le corps est le second membre.
    let (_, compte) = register(&api, &email).await;
    assert!(
        compte["email_verified_at"].is_null(),
        "une adresse fraîchement inscrite n'est pas vérifiée"
    );

    let (_, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte vient d'être créé");

    let (statut, _) = call(
        &api,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    let paire = login(&api, &email, PASSWORD).await;
    let (_, profil) = call(
        &api,
        get_authenticated("/auth/me", paire["access_token"].as_str().expect("jeton")),
    )
    .await;

    assert!(
        !profil["email_verified_at"].is_null(),
        "me ne montre pas la vérification : {profil}"
    );
}

/// Un jeton de réinitialisation ne vaut pas comme jeton de vérification.
///
/// C'est ce que l'usage porté par la recherche achète : sans lui, la table unique serait
/// une faille au lieu d'une économie.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_does_not_verify_an_address() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, _) = call(
        &api,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

/// Une adresse inconnue rend 202, comme `forgot-password` et pour la même raison.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn resending_to_an_unknown_address_is_accepted() {
    let api = application().await;

    let (statut, _) = call(
        &api,
        post_json(
            "/auth/resend-verification",
            json!({ "email": fresh_email() }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);
}

/// La garde rejette avant la vérification et laisse passer après.
///
/// La route est montée ici et nulle part ailleurs : le fragment livre la garde sans
/// l'imposer, et c'est au projet de décider où elle s'applique.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn the_verified_guard_opens_only_after_verification() {
    async fn protegee(_verifiee: crate::auth::guard::VerifiedIdentity) -> StatusCode {
        StatusCode::OK
    }

    let db = connection().await;
    let config = rbs_core::Config::load().expect("configuration lisible");

    let api = Router::new()
        .route("/protegee", get(protegee))
        .with_state(AppState::new(db.clone(), config).expect("état partagé constructible"));
    let publique = application().await;

    let email = fresh_email();
    register(&publique, &email).await;
    let paire = login(&publique, &email, PASSWORD).await;
    let jeton_acces = paire["access_token"]
        .as_str()
        .expect("jeton d'accès")
        .to_owned();

    let (avant, _) = call(&api, get_authenticated("/protegee", &jeton_acces)).await;
    assert_eq!(
        avant,
        StatusCode::FORBIDDEN,
        "une adresse non vérifiée passe la garde"
    );

    let (_, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    call(
        &publique,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;

    let (apres, _) = call(&api, get_authenticated("/protegee", &jeton_acces)).await;
    assert_eq!(apres, StatusCode::OK, "une adresse vérifiée est rejetée");
}
