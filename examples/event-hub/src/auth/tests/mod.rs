use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

mod password;
mod session;
mod verification;

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
        .expect("l'adresse est neuve")
}

/// Compte les jetons à usage unique d'**un** compte, jamais de la table entière.
///
/// Borné à un utilisateur parce que les tests partagent une base qu'ils ne vident pas et
/// que `cargo test` les exécute en parallèle : un compte global serait faux dès qu'un
/// autre test inscrit quelqu'un pendant la mesure.
pub(super) async fn one_time_tokens_count_for(db: &DatabaseConnection, user_id: Uuid) -> u64 {
    crate::auth::model::one_time_token::Entity::find()
        .filter(crate::auth::model::one_time_token::Column::UserId.eq(user_id))
        .count(db)
        .await
        .expect("le comptage aboutit")
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

/// `post_json`, porteur d'un jeton d'accès. `get_authenticated` et `delete_authenticated`
/// suivront le même modèle pour les routes protégées des autres méthodes.
fn post_json_authenticated(chemin: &str, jeton: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(chemin)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

/// `without_body`, porteur d'un jeton d'accès.
fn get_authenticated(chemin: &str, jeton: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(chemin)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée")
}

/// `without_body`, porteur d'un jeton d'accès.
fn delete_authenticated(chemin: &str, jeton: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(chemin)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Une adresse jamais inscrite : les tests partagent une base qu'ils ne vident pas.
fn fresh_email() -> String {
    format!("{}@exemple.test", Uuid::new_v4())
}

/// Attend qu'une condition devienne vraie, cinq secondes au plus.
///
/// Les émissions de jetons partent en tâche détachée : la réponse précède l'écriture.
async fn eventually<F, Fut>(mut condition: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..50 {
        if condition().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    false
}

/// Inscrit `email`, et rend statut et corps une fois le jeton de vérification écrit.
///
/// L'inscription émet ce jeton en tâche détachée : sans cette attente, l'émission
/// arriverait après coup et fermerait le jeton qu'un test vient de s'ouvrir. Une
/// inscription en double trouve celui de la première aussitôt.
async fn register(api: &Router, email: &str) -> (StatusCode, Value) {
    let (status, body) = call(
        api,
        post_json(
            "/auth/register",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;

    if status.is_success() {
        let db = connection().await;
        let adresse = crate::auth::service::normalise(email);
        let (db, adresse) = (&db, adresse.as_str());
        assert!(
            eventually(move || async move {
                match crate::auth::repository::find_by_email(db, adresse)
                    .await
                    .expect("la lecture aboutit")
                {
                    Some(compte) => one_time_tokens_count_for(db, compte.id).await > 0,
                    None => false,
                }
            })
            .await,
            "l'inscription n'a ouvert aucun jeton de vérification"
        );
    }

    (status, body)
}

/// Le compte que porte `email`, lu en base : l'inscription ne rend pas de corps.
async fn account(email: &str) -> crate::auth::repository::Model {
    crate::auth::repository::find_by_email(
        &connection().await,
        &crate::auth::service::normalise(email),
    )
    .await
    .expect("la lecture aboutit")
    .expect("le compte est inscrit")
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

/// Connecte `email` et rend la paire obtenue, en supposant la connexion réussie.
///
/// Les tests qui veulent observer un échec de connexion appellent `authenticate`
/// directement : celui-ci n'a rien à en faire.
async fn login(api: &Router, email: &str, mot_de_passe: &str) -> Value {
    let (status, paire) = authenticate(api, email, mot_de_passe).await;
    assert_eq!(status, StatusCode::OK, "{paire}");

    paire
}

/// Le jeton part dans le fragment : un navigateur ne l'envoie jamais au serveur, donc ni
/// journal d'accès ni en-tête `Referer` ne le portent.
#[test]
fn a_link_carries_its_token_in_the_fragment() {
    let flows = crate::auth::config::FlowConfig {
        app_url: "https://exemple.test/".to_owned(),
        ..Default::default()
    };

    assert_eq!(
        flows.link("reset-password", "abc"),
        "https://exemple.test/reset-password#token=abc"
    );
}
