use super::*;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

async fn logout(api: &Router, token: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json("/auth/logout", json!({ "refresh_token": token })),
    )
    .await
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn logout_returns_204() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;

    let (status, body) = logout(&api, &refresh_for(&paire)).await;

    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(body, Value::Null, "un 204 ne porte pas de corps");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_revoked_refresh_returns_401() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;
    let token = refresh_for(&paire);

    let (status, body) = logout(&api, &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");

    let (status, body) = refresh(&api, &token).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// Un jeton fermé par `logout` puis rejoué n'est pas un jeton volé : c'est un client qui
/// réessaie. Le traiter comme un rejeu fermerait toutes les sessions du compte à la
/// demande de qui tient un jeton mort — trente jours durant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_refresh_closed_by_logout_when_replayed_leaves_the_other_sessions_open() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, fermee) = authenticate(&api, &email, PASSWORD).await;
    let (_, vivante) = authenticate(&api, &email, PASSWORD).await;

    let (status, _) = logout(&api, &refresh_for(&fermee)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (rejeu, body) = refresh(&api, &refresh_for(&fermee)).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&vivante)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la session sœur est tombée sur le rejeu d'un jeton fermé : {body}"
    );
}

/// Se déconnecter d'un appareil ne déconnecte pas les autres.
///
/// C'est la garantie que ce lot ajoute : la révocation porte sur la ligne présentée, et
/// non sur le compte qui la détient.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_other_sessions_of_the_same_account_stay_valid() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let (_, seconde) = authenticate(&api, &email, PASSWORD).await;

    let (status, body) = logout(&api, &refresh_for(&premiere)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&seconde)).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "la seconde session a été fermée avec la première : {body}"
    );
}
