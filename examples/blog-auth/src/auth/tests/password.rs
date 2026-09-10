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
