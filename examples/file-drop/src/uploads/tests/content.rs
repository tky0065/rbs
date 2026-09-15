use super::*;

use axum::http::HeaderMap;

/// La requête d'un dépôt : un corps binaire, `application/octet-stream`.
fn binary(method: &str, path: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/octet-stream")
        .body(Body::from(body))
        .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut, ses en-têtes et son corps
/// tel quel.
///
/// `call` lit le corps comme du JSON : le contenu déposé est binaire, et c'est l'octet
/// rendu qui se compare.
async fn call_raw(api: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (status, headers, bytes.to_vec())
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

    let (status, headers, read) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::OK, "le contenu déposé doit se relire");
    assert_eq!(headers["content-type"], "application/octet-stream");
    // Le corps part en flux : sans taille annoncée, un flux coupé en route passerait pour
    // complet.
    assert_eq!(
        headers
            .get("content-length")
            .and_then(|value| value.to_str().ok()),
        Some(deposited.len().to_string().as_str()),
        "la taille déposée doit être annoncée"
    );
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

/// Un binaire part tel quel, même vers un client qui accepte gzip : il garde sa taille, et
/// un contenu déjà compressé ne l'est pas une seconde fois.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_binary_content_is_served_uncompressed() {
    let api = application().await;
    let collection = "/uploads";

    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let content = format!("{resource}/content");

    // Au-delà du seuil sous lequel la compression ne s'applique jamais.
    let deposited = vec![b'a'; 4096];
    let (status, _, _) = call_raw(&api, binary("PUT", &content, deposited.clone())).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "dépôt refusé");

    let mut demande = without_body("GET", &content);
    demande
        .headers_mut()
        .insert("accept-encoding", "gzip".parse().expect("en-tête valide"));
    let (status, headers, read) = call_raw(&api, demande).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        headers.get("content-encoding").is_none(),
        "un binaire ne doit pas partir compressé : {headers:?}"
    );
    assert_eq!(
        headers
            .get("content-length")
            .and_then(|value| value.to_str().ok()),
        Some("4096"),
        "la taille déposée doit rester annoncée"
    );
    assert_eq!(read, deposited, "l'octet rendu diffère de l'octet déposé");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// Un octet de trop franchit `TAILLE_MAX`, et le dépôt est refusé.
///
/// La borne est celle que le `mod.rs` de la feature engendre, deux modules plus haut : la
/// relever garde ce test juste.
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

    let trop_grand = vec![b'x'; super::super::TAILLE_MAX + 1];
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
