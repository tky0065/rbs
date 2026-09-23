use serde_json::json;

use super::*;

/// L'identifiant que porte `jeton` dans son `sub`.
fn sujet(jeton: &str) -> String {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::jwt::verify(jeton, &config.auth.secret)
        .expect("jeton lisible")
        .sub
}

// region: auteur
/// L'auteur d'un ticket est l'appelant, quoi que le corps prétende.
///
/// Le corps nomme un autre compte, réel : un identifiant inventé ferait refuser l'insertion
/// par la clé étrangère, et le test passerait sans rien prouver.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_author_is_the_caller_whatever_the_body_claims() {
    let api = application().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let jeton = token(&db, "user").await;
    let autrui = sujet(&token(&db, "user").await);

    let requete = Request::builder()
        .method("POST")
        .uri("/tickets")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::from(
            json!({
                "sujet": "L'imprimante du deuxième",
                "detail": "Elle imprime tout en double.",
                "statut": "ouvert",
                "priorite": "haute",
                "auteur_id": autrui,
            })
            .to_string(),
        ))
        .expect("requête bien formée");

    let (status, body) = call(&api, requete).await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["auteur_id"], sujet(&jeton).as_str(), "{body}");
}
// endregion: auteur

/// Un `PATCH` ne réécrit pas l'auteur : le champ n'est plus dans le contrat de mise à jour.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_author_survives_an_update_that_names_another() {
    let api = application().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let jeton = token(&db, "user").await;
    let autrui = sujet(&token(&db, "user").await);
    let envoi = |methode: &str, chemin: &str, corps: Value| {
        Request::builder()
            .method(methode)
            .uri(chemin)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {jeton}"))
            .body(Body::from(corps.to_string()))
            .expect("requête bien formée")
    };

    let (_, cree) = call(
        &api,
        envoi(
            "POST",
            "/tickets",
            json!({
                "sujet": "Le badge du parking",
                "detail": "Il ne s'ouvre plus.",
                "statut": "ouvert",
                "priorite": "normale",
            }),
        ),
    )
    .await;
    let chemin = format!("/tickets/{}", cree["id"].as_str().expect("id rendu"));
    let (status, body) = call(
        &api,
        envoi(
            "PATCH",
            &chemin,
            json!({ "statut": "en_cours", "auteur_id": autrui }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["statut"], "en_cours", "{body}");
    assert_eq!(body["auteur_id"], sujet(&jeton).as_str(), "{body}");
}
