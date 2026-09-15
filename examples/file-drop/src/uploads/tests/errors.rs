use super::*;

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
