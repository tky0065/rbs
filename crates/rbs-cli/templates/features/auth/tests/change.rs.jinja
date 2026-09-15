use super::*;

/// Le parcours nominal : la paire rendue est utilisable, et l'ancienne ne l'est plus.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_password_returns_a_usable_pair_and_closes_the_others() {
    let api = application().await;
    let email = fresh_email();

    signed_up(&api, &email).await;
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

    // La nouvelle paire d'abord, puis l'ancienne : l'ordre n'importe plus depuis qu'un
    // jeton fermé rejoué ne ferme rien d'autre, mais lire le succès avant le refus est ce
    // qu'un lecteur attend.

    // Celle que le changement vient de rendre tourne.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": corps["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);

    // Et son jeton d'accès ouvre : émis dans la seconde même de la révocation, il n'est
    // pas né mort — c'est le plancher d'`issue` qui le garantit.
    let (ouvre, profil) = call(
        &api,
        get_authenticated(
            "/auth/me",
            corps["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(ouvre, StatusCode::OK, "{profil}");

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
}

/// Une boîte compromise a pu recevoir une demande de réinitialisation d'un attaquant :
/// le titulaire qui reprend la main par `/auth/change-password` ferme ce lien-là, et pas
/// seulement ses sessions.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_password_closes_pending_reset_links() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    signed_up(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, corps) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            paire["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": PASSWORD,
                "new_password": "un mot de passe repris en main",
            }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::OK, "corps : {corps}");

    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "le mot de passe de l'attaquant" }),
        ),
    )
    .await;
    assert_eq!(
        rejet,
        StatusCode::UNAUTHORIZED,
        "le lien de réinitialisation aurait dû être fermé par le changement de mot de passe"
    );
}

/// Un mot de passe courant faux rend 403 et non 401 : l'appelant est identifié, son
/// Bearer est bon. Un 401 lui dirait que son jeton est mort et déclencherait un
/// rafraîchissement inutile.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_wrong_current_password_is_forbidden_not_unauthorized() {
    let api = application().await;
    let email = fresh_email();

    signed_up(&api, &email).await;
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
