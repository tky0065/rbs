use super::*;

use super::roles::login_as_admin;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]`.

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn listing_accounts_without_a_token_returns_401() {
    let api = application().await;

    let (status, body) = call(&api, post_json("/users/filter", json!({}))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// Lister les adresses est une donnée personnelle : un compte ordinaire ne les voit pas.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_listing_accounts_gets_403() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;

    let (status, body) = call(
        &api,
        post_json_authenticated("/users/filter", &access_for(&paire), json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

/// Un écran résout les auteurs d'une page en un appel : `in` sur l'identifiant, et rien
/// d'autre que l'identifiant et l'adresse en retour.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_resolves_accounts_by_their_ids_and_reads_only_id_and_email() {
    let api = application().await;
    let db = connection().await;
    let admin = login_as_admin(&api, &db).await;
    let premier = registered_user(&db).await;
    let second = registered_user(&db).await;

    let (status, body) = call(
        &api,
        post_json_authenticated(
            "/users/filter",
            &access_for(&admin),
            json!({ "id": { "in": [premier.id, second.id] } }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let lignes = body["data"].as_array().expect("une page");
    assert_eq!(lignes.len(), 2, "{body}");
    for ligne in lignes {
        let cles: Vec<&String> = ligne.as_object().expect("un objet").keys().collect();
        assert_eq!(cles, ["email", "id"], "{body}");
    }
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_searches_accounts_by_email() {
    let api = application().await;
    let db = connection().await;
    let admin = login_as_admin(&api, &db).await;
    let cible = registered_user(&db).await;
    let motif = &cible.email[..12];

    let (status, body) = call(
        &api,
        post_json_authenticated(
            "/users/filter",
            &access_for(&admin),
            json!({ "email": { "contains": motif } }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["data"]
            .as_array()
            .expect("une page")
            .iter()
            .any(|l| l["id"] == cible.id.to_string()),
        "{body}"
    );
}
