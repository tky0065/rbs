use std::fs;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::routing::get;
use tower::ServiceExt;

use super::{Config, servir};

/// Un répertoire vide, propre à un test, sous la racine temporaire du système.
fn racine(nom: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("frontend-{}-{nom}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("le répertoire du test doit se créer");

    path
}

/// Une configuration visant `dir`, le reste au défaut.
fn visant(dir: &Path) -> Config {
    Config {
        dir: dir.display().to_string(),
        ..Config::default()
    }
}

/// Une application réduite au repli du module, sans état ni base.
///
/// C'est exactement ce que `routes()` monte, la sonde de la base déjà rendue : le repli
/// s'éprouve ainsi sans qu'un PostgreSQL ait à tourner.
fn application(config: Config, base_joignable: bool) -> Router {
    Router::new().fallback(move |request: Request| {
        let config = config.clone();
        async move { servir(&config, base_joignable, request).await }
    })
}

/// Le corps de la réponse de `app` à un `GET uri`, avec son statut.
async fn reponse(app: Router, uri: &str) -> (StatusCode, String, Option<String>) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("requête constructible"),
        )
        .await
        .expect("l'application doit répondre");

    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|valeur| valeur.to_str().expect("en-tête ASCII").to_string());
    let corps = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("le corps doit se lire");

    (
        status,
        String::from_utf8(corps.to_vec()).expect("le corps est de l'UTF-8"),
        content_type,
    )
}

/// Le premier démarrage : rien n'est construit, et la racine explique la situation au lieu
/// de rendre une erreur.
#[tokio::test]
async fn the_bootstrap_page_is_served_at_the_root_when_nothing_is_built() {
    let app = application(visant(&racine("neuf").join("absent")), true);

    let (status, corps, content_type) = reponse(app, "/").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
    assert!(corps.contains("admin-console"), "{corps}");
    assert!(corps.contains("npm run build"), "{corps}");
}

/// Rien de ce que la page affiche n'est distant : ni police, ni feuille, ni image. Elle est
/// rendue avant que la moindre dépendance du client ne soit installée.
#[tokio::test]
async fn the_bootstrap_page_loads_nothing_from_outside() {
    let app = application(visant(&racine("autonome").join("absent")), true);

    let (_, corps, _) = reponse(app, "/").await;

    for distant in ["http://", "https://", "<link", "<img", "<script", "@import"] {
        assert!(!corps.contains(distant), "`{distant}` dans :\n{corps}");
    }
}

/// Le répertoire affiché est celui de la configuration, et non une valeur recopiée : le
/// développeur qui déplace son client lit le bon chemin.
#[tokio::test]
async fn the_bootstrap_page_names_the_configured_directory() {
    let config = Config {
        dir: "web/build".to_string(),
        ..Config::default()
    };

    let (_, corps, _) = reponse(application(config, true), "/").await;

    assert!(corps.contains("web/build"), "{corps}");
    assert!(corps.contains("cd web"), "{corps}");
}

/// L'état de la base est celui du ping, pas une affirmation écrite d'avance.
#[tokio::test]
async fn the_bootstrap_page_tells_the_real_state_of_the_database() {
    let absent = racine("base").join("absent");

    let (_, joignable, _) = reponse(application(visant(&absent), true), "/").await;
    let (_, injoignable, _) = reponse(application(visant(&absent), false), "/").await;

    // L'API et la base répondent, le build manque ; puis l'API seule répond.
    assert_eq!(joignable.matches("class=\"bon\"").count(), 2, "{joignable}");
    assert_eq!(
        injoignable.matches("class=\"bon\"").count(),
        1,
        "{injoignable}"
    );
}

/// Le repli ne voit que ce que personne n'a réclamé : une route montée ailleurs — l'API,
/// `/health`, la documentation OpenAPI — est trouvée avant lui.
#[tokio::test]
async fn a_mounted_route_is_never_reached_by_the_fallback() {
    let config = visant(&racine("montees").join("absent"));

    for (uri, attendu) in [
        ("/health", "sonde"),
        ("/api-docs/openapi.json", "document"),
        ("/posts", "liste"),
    ] {
        let app = Router::new()
            .route("/health", get(|| async { "sonde" }))
            .route("/api-docs/openapi.json", get(|| async { "document" }))
            .route("/posts", get(|| async { "liste" }))
            .merge(application(config.clone(), true));

        let (status, corps, _) = reponse(app, uri).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(corps, attendu, "le repli a masqué {uri}");
    }
}

/// Dès qu'un index existe, c'est lui qui est servi : la page d'amorçage s'efface sans
/// qu'aucun réglage ne change.
#[tokio::test]
async fn the_build_erases_the_bootstrap_page_as_soon_as_it_exists() {
    let dir = racine("construit");
    let config = visant(&dir);
    let avant = reponse(application(config.clone(), true), "/").await.1;
    assert!(avant.contains("npm run build"), "{avant}");

    fs::write(dir.join("index.html"), "<!doctype html><p>le client</p>")
        .expect("l'index doit s'écrire");

    let (status, corps, _) = reponse(application(config, true), "/").await;

    assert_eq!(status, StatusCode::OK);
    assert!(corps.contains("le client"), "{corps}");
    assert!(!corps.contains("npm run build"), "{corps}");
}

/// Un rechargement sur une route profonde du client rend l'application, et non un 404 :
/// c'est le client qui connaît ses propres routes, pas le serveur.
#[tokio::test]
async fn an_unknown_deep_route_falls_back_to_the_index() {
    let dir = racine("profonde");
    fs::write(dir.join("index.html"), "<!doctype html><p>le client</p>")
        .expect("l'index doit s'écrire");

    let (status, corps, _) = reponse(application(visant(&dir), true), "/admin/sessions").await;

    assert_eq!(status, StatusCode::OK);
    assert!(corps.contains("le client"), "{corps}");
}

/// Un fichier du build est servi tel quel, et non remplacé par l'index.
#[tokio::test]
async fn an_asset_of_the_build_is_served_as_itself() {
    let dir = racine("asset");
    fs::write(dir.join("index.html"), "<!doctype html><p>le client</p>")
        .expect("l'index doit s'écrire");
    fs::create_dir_all(dir.join("assets")).expect("le répertoire des assets se crée");
    fs::write(dir.join("assets/app.js"), "export const a = 1;").expect("l'asset doit s'écrire");

    let (status, corps, _) = reponse(application(visant(&dir), true), "/assets/app.js").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(corps, "export const a = 1;");
}

/// Le repli monté dans le routeur réel du projet.
///
/// Les tests précédents exercent le repli seul ; celui-ci prouve qu'il cohabite avec ce
/// que le squelette monte déjà — la santé, la documentation OpenAPI et l'arbre de routes
/// qu'elle déploie, les couches du routeur. Aucune base n'est jointe : une connexion non
/// établie suffit, et la page dit alors que la base ne répond pas, ce qui est vrai.
#[tokio::test]
async fn the_project_router_serves_the_bootstrap_page_at_its_root() {
    let (status, corps, content_type) = reponse(routeur(), "/").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
    assert!(corps.contains("admin-console"), "{corps}");
}

/// Et dans ce routeur-là, la sonde de santé et le document OpenAPI répondent toujours pour
/// eux-mêmes : le repli ne prend que ce qu'aucune route n'a réclamé.
///
/// Le document est monté *après* le repli dans `src/router.rs`, et c'est précisément ce que
/// ce test garde : l'ordre du `.merge()` ne décide de rien, seule la correspondance de
/// route décide.
#[tokio::test]
async fn the_project_router_keeps_its_probe_and_its_openapi_document() {
    for uri in ["/health", "/api-docs/openapi.json"] {
        let (_, corps, content_type) = reponse(routeur(), uri).await;

        assert_eq!(
            content_type.as_deref(),
            Some("application/json"),
            "{uri} est passé au repli"
        );
        assert!(!corps.contains("npm run build"), "{uri} : {corps}");
    }
}

/// Un build qui sort à la racine du projet n'a aucun répertoire où se rendre : la page
/// n'imprime alors pas de `cd`, plutôt qu'un `cd` vers le répertoire de sortie lui-même.
#[tokio::test]
async fn a_build_at_the_project_root_gets_no_change_of_directory() {
    let config = Config {
        dir: "dist".to_string(),
        ..Config::default()
    };

    let (_, corps, _) = reponse(application(config, true), "/").await;

    assert!(!corps.contains("cd "), "{corps}");
    assert!(corps.contains("npm run build"), "{corps}");
}

/// Le routeur du projet, sur un état dont la base n'est pas établie.
fn routeur() -> Router {
    let config = rbs_core::Config::load().expect("la configuration du projet se lit");
    // Le défaut de `DatabaseConnection` est la connexion non établie : c'est ce qui
    // permet de monter le routeur sans qu'aucun serveur ne tourne.
    let state = crate::state::AppState::new(sea_orm::DatabaseConnection::default(), config)
        .expect("l'état se construit");

    crate::router::router(state)
}
