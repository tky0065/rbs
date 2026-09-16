use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

// Les tests de ce répertoire joignent la base que décrit `.env`, et sont donc
// `#[ignore]` : `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre
// la base du projet, migrations appliquées.

/// Monte l'application sur la base décrite par `.env`, sans écouter sur le réseau.
///
/// Les migrations sont supposées appliquées : elles précèdent `cargo test`.
async fn application() -> Router {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    // Deux tests montent l'application en parallèle : le premier jeton posé sert à tous,
    // et le compte de l'autre reste simplement sans usage.
    if JETON.get().is_none() {
        let jeton = token(&db, "admin").await;
        let _ = JETON.set(jeton);
    }

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

/// Le jeton d'accès de ce binaire de test, posé par `application()`.
///
/// Un seul compte pour tous les tests : `Identity` relit la ligne du compte à chaque
/// requête — rôle et révocations — et un jeton signé pour un `sub` inventé est refusé.
static JETON: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Inscrit un compte au rôle voulu et signe un jeton pour lui.
async fn token(db: &DatabaseConnection, role: &str) -> String {
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};

    let config = rbs_core::Config::load().expect("configuration lisible");
    let compte = crate::auth::repository::create(
        db,
        &format!("{}@exemple.test", Uuid::new_v4()),
        "hash sans valeur",
    )
    .await
    .expect("le compte s'insère")
    .expect("l'adresse est neuve");
    let role_connu: crate::auth::model::Role =
        sea_orm::ActiveEnum::try_from_value(&role.to_owned()).expect("rôle connu");
    let mut promu: crate::auth::model::user::ActiveModel = compte.into();
    promu.role = Set(role_connu);
    let compte = promu.update(db).await.expect("rôle posé");

    let maintenant = chrono::Utc::now().timestamp();
    let claims = rbs_core::jwt::Claims {
        sub: compte.id.to_string(),
        role: role.to_string(),
        exp: maintenant + 300,
        iat: maintenant,
        jti: Uuid::new_v4().to_string(),
    };

    rbs_core::jwt::sign(&claims, &config.auth.secret).expect("jeton signable")
}

/// L'en-tête `Authorization` des requêtes que construisent les tests de ce répertoire.
fn bearer() -> String {
    format!(
        "Bearer {}",
        JETON
            .get()
            .expect("`application()` inscrit le compte avant toute requête")
    )
}

fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", bearer())
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

fn without_body(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", bearer())
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

mod access;
mod errors;
mod filter;
mod lifecycle;
