use super::*;

use serde_json::json;

/// La clé n'existe qu'une fois hors du processus de l'appelant : une seule lecture de la
/// liste livrerait sinon toutes les clés du compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_key_is_returned_once_and_never_by_the_list() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let premiere = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(&api, par_cle("GET", "/api-keys", &premiere, None)).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
    let rendu = corps.to_string();
    assert!(
        !rendu.contains(&premiere),
        "la liste livre la clé : {rendu}"
    );
    assert!(
        rendu.contains("\"prefix\""),
        "la liste doit porter le préfixe : {rendu}"
    );
}

/// La création porte `no-store` par son type : un mandataire qui la mettrait en cache
/// servirait la clé à l'appelant suivant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_creation_response_is_never_cached() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let sienne = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, entetes) = call(
        &api,
        par_cle("POST", "/api-keys", &sienne, Some(json!({ "name": "ci" }))),
    )
    .await;

    assert_eq!(statut, StatusCode::CREATED, "{corps}");
    assert_eq!(
        entetes
            .get(axum::http::header::CACHE_CONTROL)
            .expect("l'en-tête est posé"),
        "no-store"
    );
    assert!(corps["key"].is_string(), "la clé est rendue ici : {corps}");
}

/// 404 et non 403 : un 403 confirmerait que cette clé existe chez quelqu'un d'autre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_a_key_of_another_account_answers_404() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let mien = compte(&db, Role::User).await;
    let autre = compte(&db, Role::User).await;
    let ma_cle = cle(&state, mien, Role::User, None).await;
    cle(&state, autre, Role::User, None).await;

    let sienne = Entity::find()
        .all(&db)
        .await
        .expect("lecture possible")
        .into_iter()
        .find(|ligne| ligne.user_id == autre)
        .expect("la clé de l'autre compte est en base");
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(
        &api,
        par_cle("DELETE", &format!("/api-keys/{}", sienne.id), &ma_cle, None),
    )
    .await;

    assert_eq!(statut, StatusCode::NOT_FOUND, "{corps}");
}

/// La propriété qui fait tout le fragment : une route gardée par `Identity`, écrite sans
/// rien savoir des clés, s'ouvre à une clé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_key_opens_a_route_that_identity_guards() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let sienne = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(&api, par_cle("GET", "/auth/me", &sienne, None)).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["id"], porteur.to_string(), "{corps}");
}
