use super::*;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// La liste ne montre que les sessions du seul appelant, et jamais l'empreinte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_session_list_shows_only_the_callers_own() {
    let api = application().await;

    let mien = fresh_email();
    signed_up(&api, &mien).await;
    let premiere = login(&api, &mien, PASSWORD).await;
    login(&api, &mien, PASSWORD).await;

    // Un second compte, dont les sessions ne doivent pas apparaître.
    let autre = fresh_email();
    signed_up(&api, &autre).await;
    login(&api, &autre, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK);
    let sessions = corps.as_array().expect("une liste");
    assert_eq!(sessions.len(), 2, "sessions rendues : {corps}");
    assert!(
        sessions
            .iter()
            .all(|session| session.get("token_hash").is_none()),
        "la liste porte l'empreinte d'un jeton : {corps}"
    );
}

/// Révoquer une session nommée ferme celle-ci, et laisse tourner les autres du même
/// compte.
///
/// La session sœur est vérifiée **avant** de rejouer celle qu'on vient de fermer : rejouer
/// un jeton de rafraîchissement révoqué arme la défense anti-rejeu de `refresh`, qui
/// referme tout le compte — sœur comprise. Inverser l'ordre ferait échouer cette
/// assertion pour une tout autre raison que celle qu'elle éprouve.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_a_named_session_leaves_the_others_of_the_same_account_running() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let premiere = login(&api, &email, PASSWORD).await;
    let seconde = login(&api, &email, PASSWORD).await;

    let (_, liste) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    let sessions = liste.as_array().expect("une liste");
    assert_eq!(sessions.len(), 2, "{liste}");

    // `open_sessions_of` trie la plus récente en tête : c'est la session de `seconde`.
    let cible = sessions[0]["id"]
        .as_str()
        .expect("un identifiant de session");

    let (statut, _) = call(
        &api,
        delete_authenticated(
            &format!("/auth/sessions/{cible}"),
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    // La session sœur, jamais nommée par la révocation, tourne toujours — vérifié avant
    // de toucher à celle qui vient de fermer.
    let (encore, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": premiere["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(encore, StatusCode::OK, "{corps}");

    // La session fermée, elle, refuse.
    let (rejet, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": seconde["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED, "{corps}");
}

/// La session d'autrui ne se révoque pas, et rend 404 plutôt que 403 : un identifiant qui
/// n'est pas le vôtre ne désigne, de votre côté, aucune session.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_someone_elses_session_is_not_found() {
    let api = application().await;

    let victime = fresh_email();
    signed_up(&api, &victime).await;
    let sienne = login(&api, &victime, PASSWORD).await;

    let attaquant = fresh_email();
    signed_up(&api, &attaquant).await;
    let paire = login(&api, &attaquant, PASSWORD).await;

    let (_, liste) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            sienne["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    let cible = liste[0]["id"].as_str().expect("un identifiant de session");

    let (statut, _) = call(
        &api,
        delete_authenticated(
            &format!("/auth/sessions/{cible}"),
            paire["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);

    // La tentative a rendu 404 sans rien révoquer : ceci n'est donc pas le rejeu d'un
    // jeton déjà fermé, et la défense anti-rejeu de `refresh` ne s'arme pas ici.
    let (encore, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": sienne["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(encore, StatusCode::OK, "{corps}");
}

/// La révocation globale ferme tout, y compris la session qui l'a demandée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_every_session_closes_them_all() {
    let api = application().await;
    let email = fresh_email();

    signed_up(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;
    let seconde = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        delete_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    for paire in [&premiere, &seconde] {
        let (rejet, corps) = call(
            &api,
            post_json(
                "/auth/refresh",
                json!({ "refresh_token": paire["refresh_token"] }),
            ),
        )
        .await;
        assert_eq!(rejet, StatusCode::UNAUTHORIZED, "{corps}");
    }
}
