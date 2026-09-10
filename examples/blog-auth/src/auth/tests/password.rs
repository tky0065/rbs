use super::*;

use chrono::Utc;

/// Deux consommations simultanées du même jeton : une seule passe.
///
/// C'est la garantie que la condition portée par l'`UPDATE` achète, et qu'une lecture
/// suivie d'une écriture ne donnerait pas — les deux franchiraient la lecture avant que
/// l'une ait écrit, et poseraient chacune leur mot de passe.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_token_is_consumed_once_even_under_concurrency() {
    let db = connection().await;
    let compte = registered_user(&db).await;

    let jeton = rbs_core::token::random();
    crate::auth::repository::one_time_token::issue(
        &db,
        compte.id,
        crate::auth::model::TokenPurpose::PasswordReset,
        rbs_core::token::fingerprint(&jeton),
        (Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("le jeton s'émet");

    let ligne = crate::auth::repository::one_time_token::find(
        &db,
        &rbs_core::token::fingerprint(&jeton),
        crate::auth::model::TokenPurpose::PasswordReset,
    )
    .await
    .expect("la lecture aboutit")
    .expect("le jeton vient d'être émis");

    let (un, deux) = tokio::join!(
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
    );

    let passes = [un.expect("pas d'erreur"), deux.expect("pas d'erreur")]
        .into_iter()
        .filter(|passe| *passe)
        .count();

    assert_eq!(passes, 1, "deux consommations concurrentes ont abouti");
}

/// Le parcours nominal : la paire rendue est utilisable, et l'ancienne ne l'est plus.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn changing_the_password_returns_a_usable_pair_and_closes_the_others() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            premiere["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": PASSWORD,
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK, "corps : {corps}");

    // L'ancienne session est fermée : son jeton de rafraîchissement ne tourne plus.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": premiere["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);

    // Celle que le changement vient de rendre, elle, tourne.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": corps["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);
}

/// Un mot de passe courant faux rend 403 et non 401 : l'appelant est identifié, son
/// Bearer est bon. Un 401 lui dirait que son jeton est mort et déclencherait un
/// rafraîchissement inutile.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_wrong_current_password_is_forbidden_not_unauthorized() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            paire["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": "ce n'est pas le bon mot de passe",
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::FORBIDDEN);
}

/// Le parcours entier, jeton compris.
///
/// Le test passe par la couche service pour l'émission : la base ne garde que
/// l'empreinte, et aucune lecture ne rendrait le jeton en clair que le courriel porte.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_sets_a_new_password_and_closes_every_session() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "corps : {corps}");

    // Le nouveau mot de passe ouvre, l'ancien non.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);

    let (refuse, _) = call(
        &api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    // Et la session ouverte avant la réinitialisation est tombée.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": paire["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);
}

/// Le même jeton, deux fois : la seconde est refusée.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_serves_once() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let corps = json!({ "token": jeton, "new_password": "un quatrieme mot de passe long" });

    let (premier, _) = call(&api, post_json("/auth/reset-password", corps.clone())).await;
    assert_eq!(premier, StatusCode::NO_CONTENT);

    let (second, _) = call(&api, post_json("/auth/reset-password", corps)).await;
    assert_eq!(second, StatusCode::UNAUTHORIZED);
}

/// Une seconde demande ferme la première : un lien parti dans une boîte qu'on ne
/// contrôle plus cesse de valoir dès qu'on en redemande un.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_new_request_invalidates_the_previous_link() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (compte, premier) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    let (_, second) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    // La fermeture du premier jeton ne l'efface pas : les deux lignes existent encore,
    // une seule reste consommable.
    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        2,
        "la fermeture du premier jeton ne doit pas en effacer la ligne"
    );

    let (refuse, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": premier, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": second, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::NO_CONTENT);
}

/// Une adresse qu'aucun compte ne porte rend 202 et n'écrit rien : distinguer les deux
/// cas ferait de cette route l'oracle d'énumération que le hash témoin de `login` écarte
/// de l'autre côté.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn forgetting_an_unknown_address_is_accepted_and_writes_nothing() {
    let api = application().await;
    let db = connection().await;
    let inconnue = fresh_email();

    let (statut, _) = call(
        &api,
        post_json("/auth/forgot-password", json!({ "email": inconnue })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);

    // Aucun compte ne porte l'adresse, donc aucun jeton n'a pu être émis : l'émission
    // part de `find_by_email`, et rien d'autre n'y mène.
    assert!(
        crate::auth::repository::find_by_email(&db, &inconnue)
            .await
            .expect("la lecture aboutit")
            .is_none(),
        "la demande a créé un compte"
    );
}
