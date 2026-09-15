use super::*;

use axum::routing::get;

/// La garde rejette avant la vérification et laisse passer après.
///
/// La route est montée ici et nulle part ailleurs : le fragment livre la garde sans
/// l'imposer, et c'est au projet de décider où elle s'applique.
#[tokio::test]
#[ignore = "joint la base du projet"]
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
    // `login` refuse une adresse non vérifiée : le jeton est signé ici, comme l'obtiendrait
    // un projet qui a mis `login_requires_verification` à `false`.
    let jeton_acces = access_token_for(&account(&email).await);

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

/// `Identity` laisse dans la requête le compte qu'`accept_in` a relu : c'est lui que la
/// garde reprend, au lieu de relire la même ligne.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn identity_leaves_the_account_it_read_for_the_verified_guard() {
    use axum::extract::FromRequestParts;

    let db = connection().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let state = AppState::new(db, config).expect("état partagé constructible");
    let publique = application().await;

    let email = fresh_email();
    signed_up(&publique, &email).await;
    let paire = login(&publique, &email, PASSWORD).await;
    let jeton_acces = paire["access_token"].as_str().expect("jeton d'accès");

    let (mut parts, ()) = Request::builder()
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {jeton_acces}"),
        )
        .body(())
        .expect("requête valide")
        .into_parts();

    let identite = rbs_core::Identity::from_request_parts(&mut parts, &state)
        .await
        .expect("le jeton est accepté");
    let crate::auth::guard::Accepted(compte) = parts
        .extensions
        .remove()
        .expect("accept_in n'a laissé aucun compte dans la requête");

    assert_eq!(compte.id, identite.user_uuid().expect("sub lisible"));
}
