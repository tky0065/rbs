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

    router(AppState::new(db, config))
}
// endregion: harnais

/// Fait traverser le routeur à `requete`, et rend son statut avec son corps.
async fn appeler(api: &Router, requete: Request<Body>) -> (StatusCode, Value) {
    let reponse = api
        .clone()
        .oneshot(requete)
        .await
        .expect("l'application doit répondre");
    let statut = reponse.status();
    let octets = to_bytes(reponse.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    // La suppression ne rend aucun corps : il se lit `null` plutôt que d'arrêter le test.
    let corps = serde_json::from_slice(&octets).unwrap_or(Value::Null);

    (statut, corps)
}

fn requete(methode: &str, chemin: &str, corps: Value) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .header("content-type", "application/json")
        .body(Body::from(corps.to_string()))
        .expect("requête bien formée")
}

fn sans_corps(methode: &str, chemin: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Compare à la réponse la valeur envoyée pour `champ`.
fn comparer(rendu: &Value, envoye: &Value, champ: &str) {
    assert_eq!(rendu[champ], envoye[champ], "« {champ} » mal rendu");
}

/// Corps de création dont les valeurs textuelles portent un suffixe tiré au sort.
///
/// Les champs uniques interdisent de rejouer deux fois la même valeur : le suffixe rend
/// chaque exécution indépendante des précédentes.
fn creation() -> Value {
    let suffixe = Uuid::new_v4();

    json!({
        "title": format!("title-{suffixe}"),
        "body": format!("body-{suffixe}"),
        "published": true,
    })
}

fn modification() -> Value {
    let suffixe = Uuid::new_v4();

    json!({
        "title": format!("title-modifie-{suffixe}"),
        "body": format!("body-modifie-{suffixe}"),
        "published": false,
    })
}

// region: cycle_de_vie
#[tokio::test]
async fn le_cycle_de_vie_complet_passe_par_l_api() {
    let api = application().await;
    let collection = "/articles";
    let envoye = creation();

    let (statut, cree) = appeler(&api, requete("POST", collection, envoye.clone())).await;
    assert_eq!(statut, StatusCode::CREATED, "création refusée : {cree}");
    comparer(&cree, &envoye, "title");
    comparer(&cree, &envoye, "body");
    comparer(&cree, &envoye, "published");

    let id = cree["id"].as_str().expect("identifiant rendu");
    let ressource = format!("{collection}/{id}");

    let (statut, lu) = appeler(&api, sans_corps("GET", &ressource)).await;
    assert_eq!(statut, StatusCode::OK, "relecture refusée : {lu}");
    assert_eq!(lu["id"], cree["id"], "l'identifiant doit être stable");

    // L'`id` est un UUIDv7 et la liste trie du plus récent au plus ancien : ce qui vient
    // d'être créé ouvre la première page.
    let premiere = format!("{collection}?per_page=1");
    let (statut, page) = appeler(&api, sans_corps("GET", &premiere)).await;
    assert_eq!(statut, StatusCode::OK, "liste refusée : {page}");
    assert_eq!(page["data"][0]["id"], cree["id"], "liste : {page}");
    assert!(
        page["meta"]["total"].as_u64().unwrap_or_default() >= 1,
        "la page doit compter au moins ce qui vient d'être créé : {page}"
    );

    let envoye = modification();
    let mise_a_jour = requete("PUT", &ressource, envoye.clone());
    let (statut, modifie) = appeler(&api, mise_a_jour).await;
    assert_eq!(statut, StatusCode::OK, "mise à jour refusée : {modifie}");
    comparer(&modifie, &envoye, "title");
    comparer(&modifie, &envoye, "body");
    comparer(&modifie, &envoye, "published");

    let (statut, _) = appeler(&api, sans_corps("DELETE", &ressource)).await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "suppression refusée");

    let (statut, _) = appeler(&api, sans_corps("GET", &ressource)).await;
    assert_eq!(statut, StatusCode::NOT_FOUND, "elle répond encore");
}
// endregion: cycle_de_vie

// region: erreur_404
#[tokio::test]
async fn un_identifiant_inconnu_rend_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let ressource = format!("/articles/{inconnu}");

    let (statut, corps) = appeler(&api, sans_corps("GET", &ressource)).await;

    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert_eq!(corps["status"], 404, "{corps}");
}
// endregion: erreur_404

// region: corps_illisible
#[tokio::test]
async fn un_corps_illisible_rend_400() {
    let api = application().await;
    let tronque = Request::builder()
        .method("POST")
        .uri("/articles")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (statut, corps) = appeler(&api, tronque).await;

    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(corps["status"], 400, "{corps}");
}
// endregion: corps_illisible
