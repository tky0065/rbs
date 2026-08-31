use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::{Extensions, HeaderMap, HeaderValue};

use super::config::{Config, Route};
use super::{FORWARDED_FOR, client, refus};

/// L'adresse du pair, telle qu'`axum::serve` la dépose dans la requête.
fn pair(ip: &str) -> Extensions {
    let mut extensions = Extensions::new();
    extensions.insert(ConnectInfo(SocketAddr::new(
        ip.parse().expect("adresse lisible"),
        54_321,
    )));

    extensions
}

fn transmise(valeur: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        FORWARDED_FOR,
        HeaderValue::from_str(valeur).expect("en-tête lisible"),
    );

    headers
}

fn ip(adresse: [u8; 4]) -> IpAddr {
    IpAddr::V4(Ipv4Addr::from(adresse))
}

/// Une configuration portant la règle stricte du fragment sur `/auth/login`.
fn avec_regle_login() -> Config {
    Config {
        routes: vec![Route {
            path: "/auth/login".to_string(),
            limit: 5,
            window_secs: 60,
        }],
        ..Config::default()
    }
}

#[test]
fn a_path_outside_every_route_falls_under_the_global_limit() {
    let config = avec_regle_login();

    let rule = config.rule("/users");

    assert_eq!(rule.limit, config.limit);
    assert_eq!(rule.scope, "");
}

/// Le cœur de la tâche : la route qui hache un Argon2 par requête anonyme est plus
/// serrée que le reste de l'API, et de loin.
#[test]
fn the_login_route_is_stricter_than_the_global_limit() {
    let config = avec_regle_login();

    let rule = config.rule("/auth/login");

    assert_eq!(rule.limit, 5);
    assert_eq!(rule.scope, "/auth/login");
    assert!(rule.limit < config.limit, "{rule:?}");
}

/// Les deux compteurs d'une même adresse ne se confondent pas : la portée de la règle
/// entre dans la clé, sans quoi les tentatives de connexion consommeraient la limite
/// globale et réciproquement.
#[test]
fn two_rules_of_one_address_count_apart() {
    let config = avec_regle_login();

    assert_ne!(
        config.rule("/auth/login").scope,
        config.rule("/users").scope
    );
}

#[test]
fn without_a_proxy_the_address_is_that_of_the_peer() {
    let config = Config::default();

    let client = client(&config, &transmise("203.0.113.9"), &pair("198.51.100.4"));

    assert_eq!(client, Some(ip([198, 51, 100, 4])));
}

/// Le drapeau levé, l'en-tête l'emporte : derrière un proxy, l'adresse du pair est celle
/// du proxy, et tous les clients partageraient un compteur.
#[test]
fn behind_a_trusted_proxy_the_forwarded_address_wins() {
    let config = Config {
        trust_forwarded_for: true,
        ..Config::default()
    };

    let client = client(
        &config,
        &transmise("203.0.113.9, 198.51.100.4"),
        &pair("10.0.0.1"),
    );

    assert_eq!(client, Some(ip([203, 0, 113, 9])));
}

/// Sans adresse, la requête passe : un compteur unique pour tout le monde ferait payer à
/// chacun ce qu'un seul consomme.
#[test]
fn a_request_without_any_address_is_not_counted() {
    let client = client(&Config::default(), &HeaderMap::new(), &Extensions::new());

    assert_eq!(client, None);
}

/// Le 429 doit rester lisible par un client qui ne connaît que le format d'erreur du
/// projet, et lui dire quand revenir.
#[test]
fn the_refusal_carries_the_problem_format_and_a_retry_after() {
    let response = refus(std::time::Duration::from_secs(60));

    assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|valeur| valeur.to_str().ok()),
        Some("application/problem+json")
    );
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::RETRY_AFTER)
            .and_then(|valeur| valeur.to_str().ok()),
        Some("60")
    );
}

/// La fenêtre compte, puis s'échoit : les deux moitiés de la garantie, sur un compteur
/// qui n'a besoin d'aucun serveur.
#[tokio::test]
async fn the_window_counts_then_expires() {
    let counter = super::Counter::new().expect("compteur constructible");
    let fenetre = std::time::Duration::from_millis(60);

    for attendu in 1..=3 {
        let compte = counter
            .hit("une-adresse", fenetre)
            .await
            .expect("le comptage aboutit");
        assert_eq!(compte, attendu);
    }

    tokio::time::sleep(fenetre * 2).await;

    let apres = counter
        .hit("une-adresse", fenetre)
        .await
        .expect("le comptage aboutit");

    assert_eq!(apres, 1, "la fenêtre échue doit repartir de zéro");
}

/// Deux clés ne se contaminent pas : c'est ce qui rend le compteur utilisable par
/// adresse.
#[tokio::test]
async fn two_keys_count_apart() {
    let counter = super::Counter::new().expect("compteur constructible");
    let fenetre = std::time::Duration::from_secs(60);

    counter
        .hit("premiere", fenetre)
        .await
        .expect("le comptage aboutit");
    let seconde = counter
        .hit("seconde", fenetre)
        .await
        .expect("le comptage aboutit");

    assert_eq!(seconde, 1);
}
