use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::auth::model::{Role, user};
use crate::router::router;
use crate::state::AppState;

// region: harnais
// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

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

/// Ouvre une connexion à la même base que l'application.
///
/// Elle ne sert qu'à promouvoir un compte : le rôle ne s'obtient par aucune route.
async fn connection() -> DatabaseConnection {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable")
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

// region: signee
/// Présente `token` sur une requête déjà construite.
///
/// Signer plutôt que construire garde les deux formes sous les yeux : ce qui n'est pas
/// signé dans ce fichier est ce que l'API laisse ouvert.
fn signed(mut request: Request<Body>, token: &str) -> Request<Body> {
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {token}")
            .parse()
            .expect("en-tête bien formé"),
    );

    request
}
// endregion: signee

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

/// Un mot de passe qui satisfait la validation du DTO d'inscription.
const PASSWORD: &str = "un mot de passe assez long";

/// Une adresse jamais inscrite : les tests partagent une base qu'ils ne vident pas.
fn fresh_email() -> String {
    format!("{}@exemple.test", Uuid::new_v4())
}

async fn register(api: &Router, email: &str) -> Value {
    let (status, profile) = call(
        api,
        request(
            "POST",
            "/auth/register",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "inscription refusée : {profile}"
    );

    profile
}

async fn log_in(api: &Router, email: &str) -> String {
    let (status, pair) = call(
        api,
        request(
            "POST",
            "/auth/login",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "connexion refusée : {pair}");

    pair["access_token"]
        .as_str()
        .expect("la paire doit porter un jeton d'accès")
        .to_owned()
}

/// Inscrit un compte ordinaire et rend son jeton d'accès.
async fn user_token(api: &Router) -> String {
    let email = fresh_email();
    register(api, &email).await;

    log_in(api, &email).await
}

// region: jeton_admin
/// Inscrit un compte, le promeut administrateur, et rend son jeton d'accès.
///
/// La promotion passe par la base : l'inscription rend toujours un `user`, par défaut de
/// la table, et le rôle ne voyage que dans un jeton émis après coup — se connecter avant
/// la promotion rendrait un jeton que la garde refuserait.
async fn admin_token(api: &Router, db: &DatabaseConnection) -> String {
    let email = fresh_email();
    let profile = register(api, &email).await;
    let id = Uuid::parse_str(
        profile["id"]
            .as_str()
            .expect("le profil porte un identifiant"),
    )
    .expect("identifiant lisible");

    let compte = user::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("la table doit être interrogeable")
        .expect("le compte inscrit doit exister");

    let mut promu: user::ActiveModel = compte.into();
    promu.role = Set(Role::Admin);
    promu.update(db).await.expect("compte promu");

    log_in(api, &email).await
}
// endregion: jeton_admin

// region: cycle_de_vie
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let db = connection().await;
    let token = admin_token(&api, &db).await;
    let collection = "/posts";
    let sent = creation();

    let creer = signed(request("POST", collection, sent.clone()), &token);
    let (status, created) = call(&api, creer).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "title");
    compare(&created, &sent, "body");
    compare(&created, &sent, "published");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");

    // Les lectures ne sont pas signées : c'est le partage que fait ce projet, et le test
    // le montre en ne présentant aucun jeton.
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
    let mise_a_jour = signed(request("PATCH", &resource, sent.clone()), &token);
    let (status, updated) = call(&api, mise_a_jour).await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {updated}");
    compare(&updated, &sent, "title");
    compare(&updated, &sent, "body");
    compare(&updated, &sent, "published");

    let supprimer = signed(without_body("DELETE", &resource), &token);
    let (status, _) = call(&api, supprimer).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");

    let (status, _) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle répond encore");

    // Une seconde suppression ne trouve plus rien à supprimer. L'assertion vaut des deux
    // côtés de `--soft-delete` : c'est elle qui attrape une suppression logique dont la
    // condition de garde manquerait, et qui rendrait alors 204 indéfiniment.
    let supprimer = signed(without_body("DELETE", &resource), &token);
    let (status, _) = call(&api, supprimer).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle se supprime deux fois");
}
// endregion: cycle_de_vie

// region: filtre
/// Filtrer reste ouvert, là où créer la ligne filtrée exige un jeton `admin`.
///
/// `POST /posts/filter` a la méthode d'une écriture sans en être une : le seul test qui
/// distingue les deux est celui qui crée avec un jeton, puis filtre sans en présenter
/// aucun. Et le rendu du filtre ne prouve rien de son côté — une condition mal traduite
/// rend une page vide, que seule une requête jouée contre la base montre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let db = connection().await;
    let token = admin_token(&api, &db).await;
    let collection = "/posts";
    let sent = creation();

    let creer = signed(request("POST", collection, sent.clone()), &token);
    let (status, created) = call(&api, creer).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let critere = json!({ "title": sent["title"] });
    let chemin = format!("{collection}/filter");
    let (status, page) = call(&api, request("POST", &chemin, critere)).await;
    assert_eq!(status, StatusCode::OK, "filtre refusé sans jeton : {page}");

    let ids: Vec<&str> = page["data"]
        .as_array()
        .expect("la liste rend un tableau")
        .iter()
        .map(|ligne| ligne["id"].as_str().expect("identifiant rendu"))
        .collect();

    let id = created["id"].as_str().expect("identifiant rendu");
    assert!(
        ids.contains(&id),
        "la ligne créée doit satisfaire son propre critère : {page}"
    );

    let resource = format!("{collection}/{id}");
    let supprimer = signed(without_body("DELETE", &resource), &token);
    let (status, _) = call(&api, supprimer).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}
// endregion: filtre

// region: refus
/// Sans jeton, la réponse dit « identifie-toi », et non « tu n'as pas le droit ».
///
/// Les deux se confondent aisément. Ici c'est l'extracteur `Identity` qui tranche, avant
/// que le corps du handler — et donc `require_role` — s'exécute.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_anonymous_write_returns_401() {
    let api = application().await;

    let (status, body) = call(&api, request("POST", "/posts", creation())).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_ne!(status, StatusCode::FORBIDDEN, "{body}");
}

/// Identifié, mais pas administrateur : c'est la garde qui répond, et elle rend 403.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_non_admin_write_returns_403() {
    let api = application().await;
    let token = user_token(&api).await;

    let creer = signed(request("POST", "/posts", creation()), &token);
    let (status, body) = call(&api, creer).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

/// La lecture reste ouverte à qui n'a pas de compte : c'est ce qui fait un blog.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_list_is_public() {
    let api = application().await;

    let (status, body) = call(&api, without_body("GET", "/posts?per_page=1")).await;

    assert_eq!(status, StatusCode::OK, "{body}");
}
// endregion: refus

// region: erreur_404
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/posts/{inconnu}");

    let (status, body) = call(&api, without_body("GET", &resource)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "{body}");
}
// endregion: erreur_404

// region: corps_illisible
/// La requête est signée : l'ordre des extracteurs veut que `Identity` passe avant le
/// corps, et sans jeton c'est 401 qui reviendrait — le 400 resterait invérifié.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let db = connection().await;
    let token = admin_token(&api, &db).await;

    let truncated = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, signed(truncated, &token)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
// endregion: corps_illisible
