use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::routing::get;
use tower::ServiceExt;

use super::{Config, Error, build};

/// Une configuration qui n'autorise que `origines`, le reste au défaut.
fn autorisant(origines: &[&str]) -> Config {
    Config {
        origins: origines.iter().map(|o| (*o).to_string()).collect(),
        ..Config::default()
    }
}

/// Une application montée sur `config`, avec une route pour la traverser.
fn application(config: &Config) -> Router {
    Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(build(config).expect("la configuration du test est valide"))
}

/// L'en-tête `access-control-allow-origin` de la réponse à une requête venue d'`origine`.
async fn allow_origin(config: &Config, origine: &str) -> Option<String> {
    let response = application(config)
        .oneshot(
            Request::builder()
                .uri("/ping")
                .header(header::ORIGIN, origine)
                .body(Body::empty())
                .expect("requête constructible"),
        )
        .await
        .expect("l'application doit répondre");

    assert_eq!(response.status(), StatusCode::OK);

    response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .map(|valeur| valeur.to_str().expect("en-tête ASCII").to_string())
}

/// Le défaut de la section : rien n'est autorisé tant que le projet n'a pas nommé ses
/// clients. C'est le contraire d'un `Any` en dur, et c'est délibéré.
#[tokio::test]
async fn an_empty_configuration_allows_no_origin() {
    let refuse = allow_origin(&Config::default(), "https://ailleurs.example").await;

    assert_eq!(refuse, None);
}

#[tokio::test]
async fn a_listed_origin_is_echoed_back() {
    let config = autorisant(&["https://app.example"]);

    let autorisee = allow_origin(&config, "https://app.example").await;

    assert_eq!(autorisee.as_deref(), Some("https://app.example"));
}

#[tokio::test]
async fn an_origin_outside_the_list_gets_nothing_back() {
    let config = autorisant(&["https://app.example"]);

    let refusee = allow_origin(&config, "https://ailleurs.example").await;

    assert_eq!(refusee, None);
}

#[tokio::test]
async fn the_wildcard_opens_the_api_to_every_origin() {
    let config = autorisant(&["*"]);

    let autorisee = allow_origin(&config, "https://ailleurs.example").await;

    assert_eq!(autorisee.as_deref(), Some("*"));
}

/// Un navigateur ignore les identifiants envoyés à une origine joker : monter la
/// combinaison ferait croire à une protection qui n'existe pas.
#[test]
fn credentials_are_refused_alongside_the_wildcard() {
    let config = Config {
        credentials: true,
        ..autorisant(&["*"])
    };

    let error = build(&config).expect_err("la combinaison doit être refusée");

    assert!(matches!(error, Error::JokerAvecIdentifiants), "{error}");
}

#[test]
fn an_unreadable_origin_names_the_key_it_comes_from() {
    let config = autorisant(&["https://app.example\n"]);

    let error = build(&config).expect_err("une origine avec un saut de ligne est invalide");

    assert!(error.to_string().contains("cors.origins"), "{error}");
}
