use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

mod change;
mod guard;
mod http;
mod login;
mod logout;
mod openapi;
mod refresh;
mod registration;
mod reset;
mod roles;
mod sessions;
mod tokens;
mod verification;

use http::*;

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

/// Pose la preuve d'adresse que le lien du courriel aurait apportée.
///
/// `login_requires_verification` vaut `true` par défaut : un compte qui doit se connecter
/// passe d'abord par là.
async fn verified(email: &str) {
    crate::auth::repository::user::mark_verified(&connection().await, account(email).await.id)
        .await
        .expect("l'adresse se vérifie");
}

/// Inscrit `email` et vérifie son adresse : un compte prêt à se connecter.
async fn signed_up(api: &Router, email: &str) {
    let (status, corps) = register(api, email).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{corps}");
    verified(email).await;
}

/// Signe un jeton d'accès pour `compte` sans passer par `login`, qui refuse une adresse
/// non vérifiée.
fn access_token_for(compte: &crate::auth::repository::Model) -> String {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let maintenant = chrono::Utc::now().timestamp();
    let claims = rbs_core::jwt::Claims {
        sub: compte.id.to_string(),
        role: sea_orm::ActiveEnum::to_value(&compte.role),
        exp: maintenant + 300,
        iat: maintenant,
        jti: Uuid::new_v4().to_string(),
    };

    rbs_core::jwt::sign(&claims, &config.auth.secret).expect("jeton signable")
}

/// Inscrit une adresse neuve et ouvre une session : l'identifiant du compte et sa paire.
async fn login_as(api: &Router) -> (Uuid, Value) {
    let email = fresh_email();
    signed_up(api, &email).await;
    let id = account(&email).await.id;

    let (status, paire) = authenticate(api, &email, PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{paire}");

    (id, paire)
}

/// Le jeton de rafraîchissement d'une paire.
fn refresh_for(paire: &Value) -> String {
    paire["refresh_token"]
        .as_str()
        .expect("la paire doit porter un jeton de rafraîchissement")
        .to_owned()
}

async fn refresh(api: &Router, token: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json("/auth/refresh", json!({ "refresh_token": token })),
    )
    .await
}

/// Le jeton d'accès d'une paire.
fn access_for(paire: &Value) -> String {
    paire["access_token"]
        .as_str()
        .expect("la paire doit porter un jeton d'accès")
        .to_owned()
}

fn with_token(methode: &str, chemin: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("requête bien formée")
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
