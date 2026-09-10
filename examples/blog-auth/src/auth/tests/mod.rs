use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

mod password;
mod session;

/// Un mot de passe qui satisfait la validation du DTO, partagé par les tests.
const PASSWORD: &str = "un mot de passe assez long";

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

/// Ouvre une connexion à la même base que l'application.
///
/// Deux garanties de la rotation ne s'observent que dans la table : l'empreinte qui y est
/// stockée, et le refus d'un jeton dont la date est passée — qu'il faut y fabriquer.
async fn connection() -> DatabaseConnection {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable")
}

/// Insère un compte avec une adresse fraîche, en contournant l'API HTTP.
///
/// Le test de concurrence a besoin d'un `user_id` avant d'émettre un jeton, sans passer
/// par `/auth/register` ni se soucier du mot de passe qui en résulte.
pub(super) async fn registered_user(db: &DatabaseConnection) -> crate::auth::repository::Model {
    crate::auth::repository::create(db, &fresh_email(), "hash sans valeur")
        .await
        .expect("le compte s'insère")
}

/// Fait traverser le routeur à `requete`, et rend son statut avec son corps.
async fn call(api: &Router, requete: Request<Body>) -> (StatusCode, Value) {
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

fn without_body(methode: &str, chemin: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .body(Body::empty())
        .expect("requête bien formée")
}

fn post_json(chemin: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(chemin)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

/// Une adresse jamais inscrite : les tests partagent une base qu'ils ne vident pas.
fn fresh_email() -> String {
    format!("{}@exemple.test", Uuid::new_v4())
}

/// Inscrit `email` et rend le corps de la réponse.
async fn register(api: &Router, email: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json(
            "/auth/register",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await
}

/// Tente une connexion et rend statut et corps.
async fn authenticate(api: &Router, email: &str, mot_de_passe: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": mot_de_passe }),
        ),
    )
    .await
}
