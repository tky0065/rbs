use std::net::IpAddr;

use reqwest::dns::{Name, Resolve};

use super::super::target::{Policy, Refusal, Resolver, is_public, refusal_in};

#[test]
fn private_loopback_and_link_local_addresses_are_not_public() {
    for adresse in [
        "127.0.0.1",
        "10.0.0.5",
        "172.16.3.4",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "224.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fd12::1",
        "fe80::1",
        "::ffff:10.0.0.1",
        "ff02::1",
        "64:ff9b::a00:5",
        "::7f00:1",
        "2002:a00:5::",
        "2001:0:4136:e378:8000:63bf:3fff:fdd2",
        "fec0::1",
        "0.0.0.1",
        "240.0.0.1",
        "192.0.0.9",
        "198.18.0.1",
        "192.88.99.1",
    ] {
        let ip: IpAddr = adresse.parse().expect("adresse lisible");
        assert!(!is_public(ip), "{adresse} devrait être refusée");
    }
}

#[test]
fn public_addresses_are_public() {
    for adresse in [
        "93.184.216.34",
        "8.8.8.8",
        "172.32.0.1",
        "2606:2800:220:1:248:1893:25c8:1946",
        "100.128.0.0",
        "192.0.1.1",
        "198.20.0.1",
        "64:ff9c::1",
        "2001:4860:4860::8888",
    ] {
        let ip: IpAddr = adresse.parse().expect("adresse lisible");
        assert!(is_public(ip), "{adresse} devrait être acceptée");
    }
}

#[test]
fn outside_development_only_https_to_a_public_host_passes() {
    let stricte = Policy::for_env("production");

    assert!(stricte.check("https://example.test/hooks").is_ok());
    assert_eq!(
        stricte.check("http://example.test/hooks").unwrap_err(),
        Refusal::Https
    );
    assert_eq!(
        stricte.check("ftp://example.test/hooks").unwrap_err(),
        Refusal::Scheme
    );
    assert_eq!(
        stricte.check("https://10.0.0.5/hooks").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://[::1]/hooks").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://localhost/hooks").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://169.254.169.254/latest").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte
            .check("https://[::ffff:10.0.0.1]/hooks")
            .unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://[64:ff9b::a00:5]/hooks").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://localhost./hooks").unwrap_err(),
        Refusal::PrivateHost
    );
    assert_eq!(
        stricte.check("https://api.localhost/hooks").unwrap_err(),
        Refusal::PrivateHost
    );
}

#[test]
fn in_development_http_and_private_hosts_pass_but_not_other_schemes() {
    let souple = Policy::for_env("development");

    assert!(souple.check("http://localhost:4000/hooks").is_ok());
    assert!(souple.check("http://127.0.0.1:4000/hooks").is_ok());
    assert_eq!(
        souple.check("file:///etc/passwd").unwrap_err(),
        Refusal::Scheme
    );
}

/// Le résolveur du client est le seul filtre de la connexion : il écarte `localhost` hors
/// de `development`, et le garde dedans.
#[tokio::test]
async fn the_resolver_drops_localhost_outside_development_and_keeps_it_in_development() {
    let refus = Resolver::new(Policy::for_env("production"))
        .resolve("localhost".parse::<Name>().expect("nom DNS valide"))
        .await;
    assert!(
        refus.is_err(),
        "le résolveur du client devrait refuser localhost en production"
    );

    let acceptees: Vec<_> = Resolver::new(Policy::for_env("development"))
        .resolve("localhost".parse::<Name>().expect("nom DNS valide"))
        .await
        .expect("le résolveur du client devrait accepter localhost en development")
        .collect();
    assert!(!acceptees.is_empty());
    assert!(
        acceptees.iter().all(|adresse| adresse.ip().is_loopback()),
        "chaque adresse résolue pour localhost doit être en boucle locale"
    );
}

/// Un client de livraison, sans le mandataire que l'environnement pourrait imposer : c'est
/// l'hôte de l'URL qui doit passer par le résolveur, pas celui d'un proxy.
fn client_filtre(env: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .dns_resolver(std::sync::Arc::new(Resolver::new(Policy::for_env(env))))
        .build()
        .expect("client constructible")
}

/// Le refus du résolveur arrive enveloppé dans l'erreur de reqwest : `post` ne le nomme
/// que si `refusal_in` le retrouve au bout de la chaîne des causes, dont la forme
/// appartient à reqwest et à hyper-util — seule une vraie requête le prouve.
#[tokio::test]
async fn a_refusal_from_the_resolver_is_found_in_the_client_error() {
    let erreur = client_filtre("production")
        .get("http://localhost:9/hook")
        .send()
        .await
        .expect_err("localhost est refusé en production");

    assert_eq!(
        refusal_in(&erreur),
        Some(Refusal::PrivateHost),
        "{erreur:?}"
    );
}

/// L'inverse : un port fermé est une panne de transport, que la file réessaie. La prendre
/// pour un refus abandonnerait une livraison qui aurait abouti plus tard.
#[tokio::test]
async fn a_connection_failure_is_not_a_refusal() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|ecoute| ecoute.local_addr())
        .expect("port libre")
        .port();

    let erreur = client_filtre("development")
        .get(format!("http://localhost:{port}/hook"))
        .send()
        .await
        .expect_err("rien n'écoute sur ce port");

    assert_eq!(refusal_in(&erreur), None, "{erreur:?}");
}
