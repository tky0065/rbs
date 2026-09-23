use super::*;

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/tickets/{inconnu}");

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
        .uri("/tickets")
        .header("content-type", "application/json")
        .header("authorization", bearer())
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
