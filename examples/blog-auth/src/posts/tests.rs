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
async fn connexion() -> DatabaseConnection {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable")
}

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

// region: signee
/// Présente `jeton` sur une requête déjà construite.
///
/// Signer plutôt que construire garde les deux formes sous les yeux : ce qui n'est pas
/// signé dans ce fichier est ce que l'API laisse ouvert.
fn signee(mut requete: Request<Body>, jeton: &str) -> Request<Body> {
    requete.headers_mut().insert(
        "authorization",
        format!("Bearer {jeton}")
            .parse()
            .expect("en-tête bien formé"),
    );

    requete
}
// endregion: signee

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

/// Un mot de passe qui satisfait la validation du DTO d'inscription.
const MOT_DE_PASSE: &str = "un mot de passe assez long";

/// Une adresse jamais inscrite : les tests partagent une base qu'ils ne vident pas.
fn email_neuf() -> String {
    format!("{}@exemple.test", Uuid::new_v4())
}

async fn inscrire(api: &Router, email: &str) -> Value {
    let (statut, profil) = appeler(
        api,
        requete(
            "POST",
            "/auth/register",
            json!({ "email": email, "password": MOT_DE_PASSE }),
        ),
    )
    .await;

    assert_eq!(
        statut,
        StatusCode::CREATED,
        "inscription refusée : {profil}"
    );

    profil
}

async fn connecter(api: &Router, email: &str) -> String {
    let (statut, paire) = appeler(
        api,
        requete(
            "POST",
            "/auth/login",
            json!({ "email": email, "password": MOT_DE_PASSE }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK, "connexion refusée : {paire}");

    paire["access_token"]
        .as_str()
        .expect("la paire doit porter un jeton d'accès")
        .to_owned()
}

/// Inscrit un compte ordinaire et rend son jeton d'accès.
async fn jeton_utilisateur(api: &Router) -> String {
    let email = email_neuf();
    inscrire(api, &email).await;

    connecter(api, &email).await
}

// region: jeton_admin
/// Inscrit un compte, le promeut administrateur, et rend son jeton d'accès.
///
/// La promotion passe par la base : l'inscription rend toujours un `user`, par défaut de
/// la table, et le rôle ne voyage que dans un jeton émis après coup — se connecter avant
/// la promotion rendrait un jeton que la garde refuserait.
async fn jeton_admin(api: &Router, db: &DatabaseConnection) -> String {
    let email = email_neuf();
    let profil = inscrire(api, &email).await;
    let id = Uuid::parse_str(
        profil["id"]
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

    connecter(api, &email).await
}
// endregion: jeton_admin

// region: cycle_de_vie
#[tokio::test]
async fn le_cycle_de_vie_complet_passe_par_l_api() {
    let api = application().await;
    let db = connexion().await;
    let jeton = jeton_admin(&api, &db).await;
    let collection = "/posts";
    let envoye = creation();

    let creer = signee(requete("POST", collection, envoye.clone()), &jeton);
    let (statut, cree) = appeler(&api, creer).await;
    assert_eq!(statut, StatusCode::CREATED, "création refusée : {cree}");
    comparer(&cree, &envoye, "title");
    comparer(&cree, &envoye, "body");
    comparer(&cree, &envoye, "published");

    let id = cree["id"].as_str().expect("identifiant rendu");
    let ressource = format!("{collection}/{id}");

    // Les lectures ne sont pas signées : c'est le partage que fait ce projet, et le test
    // le montre en ne présentant aucun jeton.
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
    let mise_a_jour = signee(requete("PUT", &ressource, envoye.clone()), &jeton);
    let (statut, modifie) = appeler(&api, mise_a_jour).await;
    assert_eq!(statut, StatusCode::OK, "mise à jour refusée : {modifie}");
    comparer(&modifie, &envoye, "title");
    comparer(&modifie, &envoye, "body");
    comparer(&modifie, &envoye, "published");

    let supprimer = signee(sans_corps("DELETE", &ressource), &jeton);
    let (statut, _) = appeler(&api, supprimer).await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "suppression refusée");

    let (statut, _) = appeler(&api, sans_corps("GET", &ressource)).await;
    assert_eq!(statut, StatusCode::NOT_FOUND, "elle répond encore");
}
// endregion: cycle_de_vie

// region: refus
/// Sans jeton, la réponse dit « identifie-toi », et non « tu n'as pas le droit ».
///
/// Les deux se confondent aisément. Ici c'est l'extracteur `Identity` qui tranche, avant
/// que le corps du handler — et donc `require_role` — s'exécute.
#[tokio::test]
async fn sans_jeton_la_creation_rend_401() {
    let api = application().await;

    let (statut, corps) = appeler(&api, requete("POST", "/posts", creation())).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
    assert_ne!(statut, StatusCode::FORBIDDEN, "{corps}");
}

/// Identifié, mais pas administrateur : c'est la garde qui répond, et elle rend 403.
#[tokio::test]
async fn un_user_ne_peut_pas_creer_403() {
    let api = application().await;
    let jeton = jeton_utilisateur(&api).await;

    let creer = signee(requete("POST", "/posts", creation()), &jeton);
    let (statut, corps) = appeler(&api, creer).await;

    assert_eq!(statut, StatusCode::FORBIDDEN, "{corps}");
}

/// La lecture reste ouverte à qui n'a pas de compte : c'est ce qui fait un blog.
#[tokio::test]
async fn la_liste_est_publique() {
    let api = application().await;

    let (statut, corps) = appeler(&api, sans_corps("GET", "/posts?per_page=1")).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
}
// endregion: refus

// region: erreur_404
#[tokio::test]
async fn un_identifiant_inconnu_rend_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let ressource = format!("/posts/{inconnu}");

    let (statut, corps) = appeler(&api, sans_corps("GET", &ressource)).await;

    assert_eq!(statut, StatusCode::NOT_FOUND);
    assert_eq!(corps["status"], 404, "{corps}");
}
// endregion: erreur_404

// region: corps_illisible
/// La requête est signée : l'ordre des extracteurs veut que `Identity` passe avant le
/// corps, et sans jeton c'est 401 qui reviendrait — le 400 resterait invérifié.
#[tokio::test]
async fn un_corps_illisible_rend_400() {
    let api = application().await;
    let db = connexion().await;
    let jeton = jeton_admin(&api, &db).await;

    let tronque = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (statut, corps) = appeler(&api, signee(tronque, &jeton)).await;

    assert_eq!(statut, StatusCode::BAD_REQUEST);
    assert_eq!(corps["status"], 400, "{corps}");
}
// endregion: corps_illisible
