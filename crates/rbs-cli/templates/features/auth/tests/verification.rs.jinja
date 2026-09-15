use super::*;

use axum::routing::get;

/// L'inscription ouvre un jeton de vérification : c'est ce qui fait que le courriel part
/// sans qu'aucune route ne soit appelée.
#[tokio::test]
#[ignore = "joint la base du projet"]
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
#[ignore = "joint la base du projet"]
async fn verifying_marks_the_address_and_shows_on_me() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    assert!(
        account(&email).await.email_verified_at.is_none(),
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

/// Une adresse déjà vérifiée ne reçoit pas de jeton neuf : il ne servirait qu'à rajeunir
/// une date qui doit rester celle de la première preuve.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_verified_address_is_not_sent_a_new_token() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (compte, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte vient d'être créé");
    crate::auth::service::verification::verify(&db, &jeton)
        .await
        .expect("le jeton vérifie l'adresse");
    let avant = one_time_tokens_count_for(&db, compte.id).await;

    let renvoi = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit");

    assert!(
        renvoi.is_none(),
        "une adresse vérifiée a reçu un jeton neuf"
    );
    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        avant,
        "une adresse vérifiée a reçu un jeton neuf"
    );
}

/// Un second jeton consommé ne réécrit pas la date : c'est celle de la première preuve
/// qui dit l'âge de l'adresse.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn verifying_again_keeps_the_first_date() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (compte, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte vient d'être créé");
    crate::auth::service::verification::verify(&db, &jeton)
        .await
        .expect("le jeton vérifie l'adresse");
    let premiere = crate::auth::repository::find(&db, compte.id)
        .await
        .expect("la lecture aboutit")
        .expect("le compte existe")
        .email_verified_at
        .expect("l'adresse vient d'être vérifiée");

    // Par le dépôt : le service refuse d'émettre pour une adresse vérifiée.
    let second = rbs_core::token::random();
    crate::auth::repository::one_time_token::issue(
        &db,
        compte.id,
        crate::auth::model::TokenPurpose::EmailVerification,
        rbs_core::token::fingerprint(&second),
        (chrono::Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("le jeton s'émet");

    // `CURRENT_TIMESTAMP` est à la seconde sur SQLite et MySQL : sans cette attente, une
    // date réécrite retomberait sur la même et le test passerait sans rien prouver.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    crate::auth::service::verification::verify(&db, &second)
        .await
        .expect("le second jeton se consomme");

    let relue = crate::auth::repository::find(&db, compte.id)
        .await
        .expect("la lecture aboutit")
        .expect("le compte existe")
        .email_verified_at;
    assert_eq!(
        relue,
        Some(premiere),
        "la date de vérification a été réécrite"
    );
}

/// Un jeton de réinitialisation ne vaut pas comme jeton de vérification.
///
/// C'est ce que l'usage porté par la recherche achète : sans lui, la table unique serait
/// une faille au lieu d'une économie.
#[tokio::test]
#[ignore = "joint la base du projet"]
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
#[ignore = "joint la base du projet"]
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

/// Une adresse inscrite rend le même 202, et le renvoi émet un jeton neuf.
///
/// C'est la branche qui envoie le courriel : un échec d'envoi y rendrait un 500 que
/// l'adresse inconnue ne rend jamais, et l'écart dirait lesquelles sont inscrites.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn resending_to_a_registered_address_is_accepted() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;

    let (statut, _) = call(
        &api,
        post_json("/auth/resend-verification", json!({ "email": email })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);

    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");

    // Deux lignes : celle de l'inscription, que le renvoi a close, et la neuve.
    let (db, id) = (&db, compte.id);
    assert!(
        eventually(move || async move { one_time_tokens_count_for(db, id).await == 2 }).await,
        "le renvoi n'a ouvert aucun jeton"
    );
}

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
