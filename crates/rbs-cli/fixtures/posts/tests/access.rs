use super::*;

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
