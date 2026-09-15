use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rbs_core::HasCoreState;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;
use tokio::sync::MutexGuard;
use tower::ServiceExt;

use super::delivery::Delivery;
use super::model::{ActiveModel, Entity, Model};
use crate::modules::jobs::Job;
use crate::state::AppState;

mod blocked;
mod emission;
mod routes;
mod signature;
mod target;

// Les tests qui suivent joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Les tests qui émettent partagent l'unique table `webhook_subscriptions` et l'unique file
/// `jobs` : ils se relaient plutôt que de se voler leurs lignes.
///
/// Le verrou est celui du fragment `jobs`, et non un second : une émission enfile dans la
/// file, dont les tests voisins vident la table. Deux verrous indépendants laisseraient
/// chaque suite effacer ce que l'autre vient d'écrire, et les deux paraîtraient fautives à
/// tour de rôle.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    let garde = crate::modules::jobs::tests::verrou_base().lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table webhook_subscriptions doit se vider");
    // De la file, seules les livraisons : les tests des fragments `jobs` et `scheduler`
    // tournent dans le même binaire et sur la même table, et un vidage sans filtre leur
    // retirerait sous les pieds les jobs qu'ils viennent d'enfiler.
    crate::modules::jobs::model::Entity::delete_many()
        .filter(crate::modules::jobs::model::Column::Kind.eq(Delivery::KIND))
        .exec(&db)
        .await
        .expect("la file doit se vider de ses livraisons");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Le profil des tests est `development`, qui tolère tout : la politique ne se lit que du
/// profil actif, donc éprouver un refus exige un état dont la configuration porte
/// `production`, et non celui que rend `table_a_soi`.
async fn table_a_soi_stricte() -> (MutexGuard<'static, ()>, AppState) {
    let (garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let mut config = state.core().config().clone();
    config.env = "production".to_string();

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Un abonnement actif, inséré sans passer par la route : ces tests-là n'authentifient rien.
async fn abonne(db: &DatabaseConnection, url: &str, events: &[&str]) -> Model {
    inserer(db, url, events, None).await
}

/// Le même, révoqué à l'instant.
async fn abonne_revoque(db: &DatabaseConnection, url: &str, events: &[&str]) -> Model {
    inserer(db, url, events, Some(chrono::Utc::now().fixed_offset())).await
}

async fn inserer(
    db: &DatabaseConnection,
    url: &str,
    events: &[&str],
    revoked_at: Option<DateTimeWithTimeZone>,
) -> Model {
    ActiveModel {
        url: Set(url.to_string()),
        events: Set(serde_json::json!(events)),
        secret: Set("secret".to_string()),
        revoked_at: Set(revoked_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("l'abonnement s'insère")
}

/// La charge utile des émissions de ces tests : ce qu'elle porte n'importe pas, seul compte
/// qu'elle voyage.
fn donnees() -> serde_json::Value {
    serde_json::json!({ "id": 1 })
}

/// Inscrit un compte au rôle voulu et signe un jeton pour lui.
///
/// Un compte réel, et non un `sub` tiré au hasard : `Identity` relit la ligne du compte
/// à chaque requête — rôle et révocations — et un jeton sans compte est refusé.
async fn token(db: &DatabaseConnection, role: &str) -> String {
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

async fn request(
    db: &DatabaseConnection,
    method: &str,
    path: &str,
    role: &str,
    body: Option<Value>,
) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {}", token(db, role).await));
    match body {
        Some(corps) => builder
            .header("content-type", "application/json")
            .body(Body::from(corps.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("requête bien formée")
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

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Les lignes de la file que ces tests ont fait naître, et elles seules.
async fn livraisons(db: &DatabaseConnection) -> usize {
    crate::modules::jobs::model::Entity::find()
        .filter(crate::modules::jobs::model::Column::Kind.eq(Delivery::KIND))
        .all(db)
        .await
        .expect("lecture possible")
        .len()
}

/// Un receveur HTTP sur un port éphémère de la boucle locale, et le compteur des requêtes
/// qu'il a reçues — partagé par les deux tests qui joignent une vraie cible.
async fn receveur_local() -> (u16, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use axum::routing::post;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port libre");
    let port = listener.local_addr().expect("adresse locale").port();
    let recu = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let compteur = recu.clone();

    tokio::spawn(async move {
        let app = Router::new().route(
            "/hook",
            post(move || {
                compteur.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { StatusCode::NO_CONTENT }
            }),
        );
        axum::serve(listener, app).await.expect("receveur");
    });

    (port, recu)
}
