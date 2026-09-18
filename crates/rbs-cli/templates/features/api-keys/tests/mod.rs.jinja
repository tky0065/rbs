use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rbs_core::HasCoreState;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};
use tower::ServiceExt;

use super::model::Entity;
use crate::auth::model::Role;
use crate::state::AppState;

mod accept;
mod routes;

// Les tests qui suivent joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Les tests se relaient sur l'unique table `api_keys` plutôt que de se voler leurs lignes.
fn verrou_base() -> &'static Mutex<()> {
    static VERROU: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    VERROU.get_or_init(Mutex::default)
}

/// La table vidée, et l'état du projet pour la tenir.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    let garde = verrou_base().lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table api_keys doit se vider");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Inscrit un compte au rôle voulu, et rend son identifiant.
///
/// Un compte réel : la clé est relue avec lui à chaque requête, et une clé sans porteur
/// est refusée.
async fn compte(db: &DatabaseConnection, role: Role) -> Uuid {
    let inscrit = crate::auth::repository::create(
        db,
        &format!("{}@exemple.test", Uuid::new_v4()),
        "hash sans valeur",
    )
    .await
    .expect("le compte s'insère")
    .expect("l'adresse est neuve");

    let mut promu: crate::auth::model::user::ActiveModel = inscrit.into();
    promu.role = Set(role);

    promu.update(db).await.expect("rôle posé").id
}

/// Tire une clé pour `porteur` par le service, et rend la valeur en clair.
async fn cle(state: &AppState, porteur: Uuid, createur: Role, role: Option<&str>) -> String {
    super::service::create(
        state,
        porteur,
        createur,
        super::dto::CreateApiKey {
            name: "test".to_owned(),
            role: role.map(str::to_owned),
            expires_in_days: None,
        },
    )
    .await
    .expect("la clé se tire")
    .key
}

/// Une clé déjà périmée : tirée avec une échéance, puis reculée dans le passé.
///
/// L'échéance ne se demande pas négative — le DTO la borne à un jour au moins — et c'est
/// bien le but : la seule façon d'obtenir une clé périmée est d'en vieillir une, comme le
/// temps le ferait.
async fn cle_perimee(state: &AppState, porteur: Uuid) -> String {
    let clair = super::service::create(
        state,
        porteur,
        Role::User,
        super::dto::CreateApiKey {
            name: "périmée".to_owned(),
            role: None,
            expires_in_days: Some(1),
        },
    )
    .await
    .expect("la clé se tire")
    .key;

    let passe = chrono::Utc::now().fixed_offset() - chrono::TimeDelta::days(1);
    let ligne = Entity::find()
        .all(state.core().db())
        .await
        .expect("lecture possible")
        .pop()
        .expect("la clé est en base");
    let mut vieillie: super::model::ActiveModel = ligne.into();
    vieillie.expires_at = Set(Some(passe));
    vieillie
        .update(state.core().db())
        .await
        .expect("l'échéance se recule");

    clair
}

/// Une requête portant la clé en `X-Api-Key`.
fn par_cle(method: &str, path: &str, cle: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header("x-api-key", cle);
    match body {
        Some(corps) => builder
            .header("content-type", "application/json")
            .body(Body::from(corps.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut, son corps et ses en-têtes.
async fn call(api: &Router, request: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let entetes = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        entetes,
    )
}
