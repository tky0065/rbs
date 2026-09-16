use super::*;

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
