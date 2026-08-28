use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

/// Monte l'application sur la base décrite par `.env`, sans écouter sur le réseau.
///
/// Les migrations sont supposées appliquées : elles précèdent `cargo test`.
async fn application() -> Router {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    router(AppState::new(db, config).expect("état partagé constructible"))
}

/// Fait traverser le routeur à `request`, et rend son statut avec son corps.
async fn call(api: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    // La suppression ne rend aucun corps : il se lit `null` plutôt que d'arrêter le test.
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, body)
}

fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

fn without_body(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Compare à la réponse la valeur envoyée pour `champ`.
fn compare(rendered: &Value, sent: &Value, champ: &str) {
    assert_eq!(rendered[champ], sent[champ], "« {champ} » mal rendu");
}

/// Corps de création dont les valeurs textuelles portent un suffixe tiré au sort.
///
/// Les champs uniques interdisent de rejouer deux fois la même valeur : le suffixe rend
/// chaque exécution indépendante des précédentes.
fn creation() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-{suffix}"),
        "owner_email": format!("owner_email-{suffix}@example.com"),
        "content_type": format!("content_type-{suffix}"),
        "size": 42,
    })
}

fn modification() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-modifie-{suffix}"),
        "owner_email": format!("owner_email-modifie-{suffix}@example.com"),
        "content_type": format!("content_type-modifie-{suffix}"),
        "size": 43,
    })
}

#[tokio::test]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/uploads";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "title");
    compare(&created, &sent, "owner_email");
    compare(&created, &sent, "content_type");
    compare(&created, &sent, "size");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");

    let (status, read) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::OK, "relecture refusée : {read}");
    assert_eq!(read["id"], created["id"], "l'identifiant doit être stable");

    // L'`id` est un UUIDv7 et la liste trie du plus récent au plus ancien : ce qui vient
    // d'être créé ouvre la première page.
    let premiere = format!("{collection}?per_page=1");
    let (status, page) = call(&api, without_body("GET", &premiere)).await;
    assert_eq!(status, StatusCode::OK, "liste refusée : {page}");
    assert_eq!(page["data"][0]["id"], created["id"], "liste : {page}");
    assert!(
        page["meta"]["total"].as_u64().unwrap_or_default() >= 1,
        "la page doit compter au moins ce qui vient d'être créé : {page}"
    );

    let sent = modification();
    let mise_a_jour = request("PUT", &resource, sent.clone());
    let (status, updated) = call(&api, mise_a_jour).await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {updated}");
    compare(&updated, &sent, "title");
    compare(&updated, &sent, "owner_email");
    compare(&updated, &sent, "content_type");
    compare(&updated, &sent, "size");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");

    let (status, _) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle répond encore");
}

/// Deux créations à la suite portent des identifiants croissants.
///
/// C'est ce qui sépare un UUIDv7 d'un v4, et ce dont dépend la liste : elle trie sur
/// l'`id` pour rendre le plus récent en tête. Un test qui se contenterait de constater
/// la présence d'un UUID laisserait passer la régression.
#[tokio::test]
async fn two_creations_in_a_row_carry_increasing_ids() {
    let api = application().await;
    let collection = "/uploads";

    let (status, premier) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {premier}");
    let (status, second) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {second}");

    let lire = |rendered: &Value| {
        Uuid::parse_str(rendered["id"].as_str().expect("identifiant rendu"))
            .expect("identifiant lisible")
    };
    let (premier, second) = (lire(&premier), lire(&second));

    assert_eq!(
        premier.get_version_num(),
        7,
        "{premier} n'est pas un UUIDv7"
    );
    assert!(second > premier, "{second} ne suit pas {premier}");
}

#[tokio::test]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/uploads/{inconnu}");

    let (status, body) = call(&api, without_body("GET", &resource)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "{body}");
}

#[tokio::test]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let truncated = Request::builder()
        .method("POST")
        .uri("/uploads")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
