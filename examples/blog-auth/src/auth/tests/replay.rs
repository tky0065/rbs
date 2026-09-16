use super::*;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Un jeton rejoué ferme le compte, et non la seule ligne qu'il présente.
///
/// Sans cela, un jeton volé et joué avant la rotation légitime laisse son porteur avec
/// une paire valide, qu'il renouvelle indéfiniment et sans bruit : c'est la détection de
/// réutilisation, et elle n'a de sens que si toute la famille tombe.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn replaying_a_refresh_closes_the_other_sessions_of_the_account() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let (_, seconde) = authenticate(&api, &email, PASSWORD).await;

    let ancien = refresh_for(&premiere);
    let (status, tournee) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK, "{tournee}");

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    // La paire née de la rotation légitime tombe elle aussi : rien ne dit lequel des deux
    // porteurs du jeton rejoué la détient.
    let (status, body) = refresh(&api, &refresh_for(&tournee)).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "la paire issue de la rotation survit au rejeu : {body}"
    );

    let (status, body) = refresh(&api, &refresh_for(&seconde)).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "la session sœur survit au rejeu : {body}"
    );
}

/// Le rejeu d'un jeton tourné est instruit une fois : la famille tombe, la ligne se ferme.
/// Le rejouer encore ne ferme plus rien — sinon qui tient un jeton mort déconnecterait le
/// titulaire à chaque reconnexion, jusqu'à l'échéance du jeton.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_replayed_token_presented_again_leaves_the_sessions_opened_since_alive() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let ancien = refresh_for(&premiere);
    let (status, _) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK);

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    // Le titulaire se reconnecte après l'expulsion.
    let (_, reconnexion) = authenticate(&api, &email, PASSWORD).await;

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&reconnexion)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la session ouverte après le rejeu est tombée sur un second rejeu : {body}"
    );
}

/// Une ligne tournée n'est pas une session ouverte : sans le filtre sur `replaced_at`,
/// la liste grossirait d'une ligne à chaque rafraîchissement, trente jours durant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rotated_session_is_listed_once_after_two_refreshes() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, paire) = authenticate(&api, &email, PASSWORD).await;
    let (status, tournee) = refresh(&api, &refresh_for(&paire)).await;
    assert_eq!(status, StatusCode::OK, "{tournee}");
    let (status, retournee) = refresh(&api, &refresh_for(&tournee)).await;
    assert_eq!(status, StatusCode::OK, "{retournee}");

    let (status, sessions) = call(
        &api,
        get_authenticated("/auth/sessions", &access_for(&retournee)),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{sessions}");
    assert_eq!(
        sessions.as_array().map(Vec::len),
        Some(1),
        "les lignes tournées sont listées : {sessions}"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_old_refresh_is_then_rejected() {
    let api = application().await;
    let ancien = refresh_for(&login_as(&api).await.1);

    let (status, body) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (rejeu, body) = refresh(&api, &ancien).await;

    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");
}
