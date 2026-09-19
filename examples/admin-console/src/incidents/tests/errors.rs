use super::*;

/// Une colonne de tri inconnue est une faute du client, et le refus la nomme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_sort_column_returns_400() {
    let api = application().await;
    let critere = json!({ "sort": ["-inconnue"] });

    let (status, body) = call(&api, request("POST", "/incidents/filter", critere)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["status"], 400, "{body}");
}

/// Une valeur déjà prise sur une colonne `unique` est une faute du client, pas une panne.
///
/// Sans la traduction que pose le repository, le doublon remonterait en 500.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_replayed_unique_value_returns_409() {
    let api = application().await;
    let collection = "/incidents";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let (status, body) = call(&api, request("POST", collection, sent)).await;

    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["status"], 409, "{body}");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/incidents/{inconnu}");

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
        .uri("/incidents")
        .header("content-type", "application/json")
        .header("authorization", bearer())
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
