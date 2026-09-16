use super::*;

use serde_json::json;

/// Le rôle par défaut des routes est `Admin` : un compte auto-inscrit, qui reçoit `User`,
/// ne peut ni se faire livrer les événements du projet ni couper les abonnements d'autrui.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_role_is_refused_on_the_three_routes() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let existant = abonne(&db, "https://example.test/hooks", &["*"]).await;
    let api = crate::router::router(state);

    let corps = json!({ "url": "https://example.test/mine", "events": ["*"] });
    let (creation, _) = call(
        &api,
        request(&db, "POST", "/webhooks/subscriptions", "user", Some(corps)).await,
    )
    .await;
    let (liste, _) = call(
        &api,
        request(&db, "GET", "/webhooks/subscriptions", "user", None).await,
    )
    .await;
    let (revocation, _) = call(
        &api,
        request(
            &db,
            "DELETE",
            &format!("/webhooks/subscriptions/{}", existant.id),
            "user",
            None,
        )
        .await,
    )
    .await;

    assert_eq!(creation, StatusCode::FORBIDDEN);
    assert_eq!(liste, StatusCode::FORBIDDEN);
    assert_eq!(revocation, StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_subscribes_then_reads_and_revokes() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let api = crate::router::router(state);

    let corps = json!({ "url": "https://example.test/hooks", "events": ["user.*"] });
    let (creation, cree) = call(
        &api,
        request(&db, "POST", "/webhooks/subscriptions", "admin", Some(corps)).await,
    )
    .await;
    assert_eq!(creation, StatusCode::CREATED, "{cree}");
    let id = cree["id"]
        .as_str()
        .expect("l'abonnement créé porte son identifiant");

    let (liste, abonnements) = call(
        &api,
        request(&db, "GET", "/webhooks/subscriptions", "admin", None).await,
    )
    .await;
    assert_eq!(liste, StatusCode::OK);
    assert_eq!(abonnements.as_array().map(Vec::len), Some(1));

    let (revocation, _) = call(
        &api,
        request(
            &db,
            "DELETE",
            &format!("/webhooks/subscriptions/{id}"),
            "admin",
            None,
        )
        .await,
    )
    .await;
    assert_eq!(revocation, StatusCode::NO_CONTENT);
}

/// Un motif blanc est une entrée non conforme au même titre qu'une URL malformée : le DTO
/// le refuse avant le service, et la réponse nomme le champ fautif.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_empty_pattern_is_refused_by_validation() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let api = crate::router::router(state);

    let corps = json!({ "url": "https://example.test/x", "events": ["user.*", "  "] });
    let (statut, reponse) = call(
        &api,
        request(&db, "POST", "/webhooks/subscriptions", "admin", Some(corps)).await,
    )
    .await;

    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{reponse}");
    assert!(
        reponse["errors"]["events"].is_array(),
        "la réponse doit nommer le champ `events` : {reponse}"
    );
}

/// La première révocation est celle qui compte : un client qui rejoue son appel ne doit
/// pas réécrire l'histoire de l'abonnement.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_twice_keeps_the_first_date() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let abonnement = abonne(db, "https://example.test/hooks", &["*"]).await;

    let premiere =
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z").expect("date lisible");
    let seconde =
        chrono::DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z").expect("date lisible");

    super::super::repository::revoke(db, abonnement.id, premiere)
        .await
        .expect("la première révocation aboutit");
    let relu = super::super::repository::revoke(db, abonnement.id, seconde)
        .await
        .expect("la seconde révocation aboutit");

    assert_eq!(relu.revoked_at, Some(premiere));
}
