use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

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

// region: jeton
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
    .expect("le compte s'insère");
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

/// L'en-tête `Authorization` des requêtes de ce fichier.
fn bearer() -> String {
    format!(
        "Bearer {}",
        JETON
            .get()
            .expect("`application()` inscrit le compte avant toute requête")
    )
}
// endregion: jeton

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

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/posts";
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

    // Une seconde suppression ne trouve plus rien à supprimer. L'assertion vaut des deux
    // côtés de `--soft-delete` : c'est elle qui attrape une suppression logique dont la
    // condition de garde manquerait, et qui rendrait alors 204 indéfiniment.
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle se supprime deux fois");
}

/// Deux créations à la suite portent des identifiants croissants.
///
/// C'est ce qui sépare un UUIDv7 d'un v4, et ce dont dépend la liste : elle trie sur
/// l'`id` pour rendre le plus récent en tête. Un test qui se contenterait de constater
/// la présence d'un UUID laisserait passer la régression.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn two_creations_in_a_row_carry_increasing_ids() {
    let api = application().await;
    let collection = "/posts";

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

/// La route de filtrage retient la ligne qui satisfait son propre critère.
///
/// Le rendu du filtre ne prouve rien : une condition mal traduite rend une page vide, et
/// seule une requête jouée contre la base le montre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let collection = "/posts";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let critere = json!({ "title": sent["title"] });
    let chemin = format!("{collection}/filter");
    let (status, page) = call(&api, request("POST", &chemin, critere)).await;
    assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

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
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// Une colonne de tri inconnue est une faute du client, et le refus la nomme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_sort_column_returns_400() {
    let api = application().await;
    let critere = json!({ "sort": ["-inconnue"] });

    let (status, body) = call(&api, request("POST", "/posts/filter", critere)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["status"], 400, "{body}");
}

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

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let truncated = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .header("authorization", bearer())
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}

// region: refus
/// Sans jeton, la requête est refusée avant même que le corps ne soit lu.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_anonymous_request_returns_401() {
    let api = application().await;
    let anonyme = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("requête bien formée");

    let (status, body) = call(&api, anonyme).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

/// La lecture aussi est fermée : sous `auth`, aucune route engendrée n'est publique.
///
/// C'est le renversement qu'opère la seule présence de la feature, et un `POST` refusé ne
/// l'atteste pas — une API qui protégerait ses seules écritures le passerait aussi. Rouvrir
/// cette route est une édition, celle que décrit le bandeau de tête du contrôleur ; ce test
/// tombe alors, et c'est ainsi qu'il doit tomber.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_anonymous_read_returns_401() {
    let api = application().await;
    let anonyme = Request::builder()
        .method("GET")
        .uri("/posts")
        .body(Body::empty())
        .expect("requête bien formée");

    let (status, body) = call(&api, anonyme).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

/// Identifié, mais sans le rôle qu'exige l'écriture : c'est la garde qui répond, et 403.
///
/// Les deux refus voisins ne disent pas la même chose — l'extracteur refuse avant le
/// handler, la garde refuse dedans. Les confondre laisserait passer un `--role admin`
/// devenu sans effet, les routes restant fermées aux anonymes. Le même jeton `user` lit
/// pourtant `/posts` : c'est le seuil, et non l'égalité, que ces deux régimes éprouvent.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_non_admin_write_returns_403() {
    let api = application().await;
    // Un compte `user` réel, à côté de l'`admin` que `application()` a inscrit : c'est
    // la ligne, et non le jeton, que `Identity` relit pour connaître le rôle.
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let jeton = token(&db, "user").await;
    let ordinaire = Request::builder()
        .method("POST")
        .uri("/posts")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::from(creation().to_string()))
        .expect("requête bien formée");

    let (status, body) = call(&api, ordinaire).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["status"], 403, "{body}");

    let lecture = Request::builder()
        .method("GET")
        .uri("/posts?per_page=1")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée");

    let (status, body) = call(&api, lecture).await;

    assert_eq!(status, StatusCode::OK, "{body}");
}
// endregion: refus
