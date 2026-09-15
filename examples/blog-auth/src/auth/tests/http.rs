use super::*;

use axum::body::to_bytes;
use tower::ServiceExt;

/// Fait traverser le routeur à `requete`, et rend son statut avec son corps.
pub(super) async fn call(api: &Router, requete: Request<Body>) -> (StatusCode, Value) {
    let response = api
        .clone()
        .oneshot(requete)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let octets = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    // Une réponse sans corps se lit `null` plutôt que d'arrêter le test.
    let body = serde_json::from_slice(&octets).unwrap_or(Value::Null);

    (status, body)
}

pub(super) fn without_body(methode: &str, chemin: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .body(Body::empty())
        .expect("requête bien formée")
}

pub(super) fn post_json(chemin: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(chemin)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

/// `post_json`, porteur d'un jeton d'accès. `get_authenticated` et `delete_authenticated`
/// suivront le même modèle pour les routes protégées des autres méthodes.
pub(super) fn post_json_authenticated(chemin: &str, jeton: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(chemin)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

/// `without_body`, porteur d'un jeton d'accès.
pub(super) fn get_authenticated(chemin: &str, jeton: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(chemin)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée")
}

/// `without_body`, porteur d'un jeton d'accès.
pub(super) fn delete_authenticated(chemin: &str, jeton: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(chemin)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée")
}
