use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
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
        "owner_email": format!("owner_email-{suffix}@example.com"),
        "content_type": format!("content_type-{suffix}"),
        "size": 42,
    })
}

fn modification() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-modifie-{suffix}"),
        "owner_email": format!("owner_email-modifie-{suffix}@example.com"),
        "content_type": format!("content_type-modifie-{suffix}"),
        "size": 43,
    })
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/uploads";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "title");
    compare(&created, &sent, "owner_email");
    compare(&created, &sent, "content_type");
    compare(&created, &sent, "size");

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
    compare(&updated, &sent, "owner_email");
    compare(&updated, &sent, "content_type");
    compare(&updated, &sent, "size");

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
    let collection = "/uploads";

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

/// Un corps lisible mais non conforme rend 422, et non le 400 du corps illisible.
///
/// C'est le chemin qu'ouvre `ValidatedJson` : il désérialise, puis valide, et le refus
/// nomme le champ fautif.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_invalid_email_returns_422() {
    let api = application().await;
    let mut sent = creation();
    sent["owner_email"] = Value::from("pas-une-adresse");

    let (status, body) = call(&api, request("POST", "/uploads", sent)).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["status"], 422, "{body}");
    assert!(
        body["errors"]["owner_email"].is_array(),
        "le refus doit nommer le champ fautif : {body}"
    );
}

/// La route de filtrage retient la ligne qui satisfait son propre critère.
///
/// Le rendu du filtre ne prouve rien : une condition mal traduite rend une page vide, et
/// seule une requête jouée contre la base le montre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let collection = "/uploads";
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

/// `%` et `_` sont des jokers de LIKE, et `!` le caractère qui les échappe : lus tels
/// quels, `contains: "%"` rendrait toute la table.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn contains_reads_percent_and_underscore_literally() {
    let api = application().await;
    let collection = "/uploads";
    let champ = "title";
    let marque = Uuid::new_v4().simple().to_string()[..10].to_string();
    let pourcent = format!("{marque}%");
    let souligne = format!("{marque}_");
    let exclamation = format!("{marque}!");
    let temoin = format!("{marque}x");

    for valeur in [&pourcent, &souligne, &exclamation, &temoin] {
        let mut sent = creation();
        sent[champ] = json!(valeur);
        let (status, created) = call(&api, request("POST", collection, sent)).await;
        assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    }

    // Lu comme un joker, chacun de ces motifs retiendrait aussi les trois autres lignes.
    let chemin = format!("{collection}/filter");
    let mut ecarts = Vec::new();
    for attendue in [&pourcent, &souligne, &exclamation] {
        let critere = json!({ champ: { "contains": attendue } });
        let (status, page) = call(&api, request("POST", &chemin, critere)).await;
        assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

        let valeurs: Vec<&str> = page["data"]
            .as_array()
            .expect("la liste rend un tableau")
            .iter()
            .map(|ligne| ligne[champ].as_str().expect("texte rendu"))
            .collect();
        if valeurs != [attendue.as_str()] {
            ecarts.push(format!("« {attendue} » rend {valeurs:?}"));
        }
    }
    assert!(
        ecarts.is_empty(),
        "`contains` doit se lire à la lettre : {ecarts:?}"
    );
}

/// Une colonne de tri inconnue est une faute du client, et le refus la nomme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_sort_column_returns_400() {
    let api = application().await;
    let critere = json!({ "sort": ["-inconnue"] });

    let (status, body) = call(&api, request("POST", "/uploads/filter", critere)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["status"], 400, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/uploads/{inconnu}");

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
        .uri("/uploads")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}

/// La requête d'un dépôt : un corps binaire, `application/octet-stream`.
fn binary(method: &str, path: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/octet-stream")
        .body(Body::from(body))
        .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut, son `Content-Type` et son
/// corps tel quel.
///
/// `call` lit le corps comme du JSON : le contenu déposé est binaire, et c'est l'octet
/// rendu qui se compare.
async fn call_raw(api: &Router, request: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (status, content_type, bytes.to_vec())
}

/// L'octet déposé par `PUT` est celui que `GET` rend, et `HEAD` reflète la présence d'un
/// contenu — avant le dépôt, après, et après son remplacement.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_content_round_trips_through_put_get_and_head() {
    let api = application().await;
    let collection = "/uploads";

    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let content = format!("{resource}/content");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "pas encore de contenu");
    let (status, _, _) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "pas encore de contenu");

    // Des octets qui ne forment pas de l'UTF-8 : un corps lu comme du texte les perdrait.
    let deposited = b"contenu binaire \xff\xfe\x00".to_vec();
    let (status, _, _) = call_raw(&api, binary("PUT", &content, deposited.clone())).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "dépôt refusé");

    let (status, content_type, read) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::OK, "le contenu déposé doit se relire");
    assert_eq!(content_type, "application/octet-stream");
    assert_eq!(read, deposited, "l'octet rendu diffère de l'octet déposé");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "HEAD doit voir le dépôt");

    let replaced = b"second contenu".to_vec();
    let (status, _, _) = call_raw(&api, binary("PUT", &content, replaced.clone())).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "remplacement refusé");
    let (status, _, read) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::OK, "le contenu remplacé doit se relire");
    assert_eq!(read, replaced, "le second dépôt doit remplacer le premier");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// Un octet de trop franchit `TAILLE_MAX`, et le dépôt est refusé.
///
/// La borne est celle que `mod.rs` engendre : la relever garde ce test juste.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_content_beyond_the_limit_returns_413() {
    let api = application().await;
    let collection = "/uploads";

    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let content = format!("{resource}/content");

    let trop_grand = vec![b'x'; super::TAILLE_MAX + 1];
    let (status, _, _) = call_raw(&api, binary("PUT", &content, trop_grand)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "un octet de trop");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "rien ne doit être déposé");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// Un identifiant jamais créé n'a pas de contenu, et n'en reçoit pas non plus.
///
/// Le `PUT` surtout : sans la lecture préalable de la ligne, il déposerait un objet
/// qu'aucune ressource ne réclame.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_has_no_content() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let content = format!("/uploads/{inconnu}/content");

    let (status, _, _) = call_raw(&api, binary("PUT", &content, b"orphelin".to_vec())).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "PUT sur un id inconnu");

    let (status, body) = call(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "GET sur un id inconnu");
    assert_eq!(body["status"], 404, "{body}");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "HEAD sur un id inconnu");
}
