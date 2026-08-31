use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

// region: harnais
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
// endregion: harnais

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
        "body": format!("body-{suffix}"),
        "published": true,
    })
}

fn modification() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-modifie-{suffix}"),
        "body": format!("body-modifie-{suffix}"),
        "published": false,
    })
}

// region: cycle_de_vie
#[tokio::test]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/articles";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "title");
    compare(&created, &sent, "body");
    compare(&created, &sent, "published");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");

    let (status, read) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::OK, "relecture refusée : {read}");
    assert_eq!(read["id"], created["id"], "l'identifiant doit être stable");

    // L'`id` est un UUIDv7 et la liste trie du plus récent au plus ancien. Ce qui se
    // vérifie est que la ligne créée est sur la première page et que la page est
    // ordonnée — non qu'elle en occupe la première place : les tests tournent en
    // parallèle, et un autre peut écrire entre la création et la lecture.
    let premiere = format!("{collection}?per_page=50");
    let (status, page) = call(&api, without_body("GET", &premiere)).await;
    assert_eq!(status, StatusCode::OK, "liste refusée : {page}");

    let ids: Vec<&str> = page["data"]
        .as_array()
        .expect("la liste rend un tableau")
        .iter()
        .map(|ligne| ligne["id"].as_str().expect("identifiant rendu"))
        .collect();

    assert!(
        ids.contains(&created["id"].as_str().expect("identifiant rendu")),
        "la ligne créée est absente de la première page : {page}"
    );

    let mut decroissants = ids.clone();
    decroissants.sort_unstable_by(|gauche, droite| droite.cmp(gauche));
    assert_eq!(ids, decroissants, "la liste n'est pas triée : {page}");

    assert!(
        page["meta"]["total"].as_u64().unwrap_or_default() >= 1,
        "la page doit compter au moins ce qui vient d'être créé : {page}"
    );

    let sent = modification();
    let mise_a_jour = request("PATCH", &resource, sent.clone());
    let (status, updated) = call(&api, mise_a_jour).await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {updated}");
    compare(&updated, &sent, "title");
    compare(&updated, &sent, "body");
    compare(&updated, &sent, "published");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");

    let (status, _) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle répond encore");
}
// endregion: cycle_de_vie

/// Deux créations à la suite portent des identifiants croissants.
///
/// C'est ce qui sépare un UUIDv7 d'un v4, et ce dont dépend la liste : elle trie sur
/// l'`id` pour rendre le plus récent en tête. Un test qui se contenterait de constater
/// la présence d'un UUID laisserait passer la régression.
#[tokio::test]
async fn two_creations_in_a_row_carry_increasing_ids() {
    let api = application().await;
    let collection = "/articles";

    let (status, premier) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {premier}");
    let (status, second) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {second}");

    let lire = |rendered: &Value| {
        Uuid::parse_str(rendered["id"].as_str().expect("identifiant rendu"))
            .expect("identifiant lisible")
    };
    let (premier, second) = (lire(&premier), lire(&second));

    // Les tests jouent sur la base du `.env`, hors transaction : sans ces suppressions, la
    // table enfle de deux lignes à chaque exécution.
    for identifiant in [premier, second] {
        let resource = format!("{collection}/{identifiant}");
        let (status, _) = call(&api, without_body("DELETE", &resource)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
    }

    assert_eq!(
        premier.get_version_num(),
        7,
        "{premier} n'est pas un UUIDv7"
    );
    assert!(second > premier, "{second} ne suit pas {premier}");
}

// region: erreur_404
#[tokio::test]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/articles/{inconnu}");

    let (status, body) = call(&api, without_body("GET", &resource)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "{body}");
}
// endregion: erreur_404

// region: corps_illisible
#[tokio::test]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let truncated = Request::builder()
        .method("POST")
        .uri("/articles")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
// endregion: corps_illisible
