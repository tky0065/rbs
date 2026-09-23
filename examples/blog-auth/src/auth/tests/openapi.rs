use super::*;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Le document tel que le serveur le publie.
async fn openapi_document(api: &Router) -> Value {
    let (status, document) = call(api, without_body("GET", "/api-docs/openapi.json")).await;

    assert_eq!(status, StatusCode::OK, "le document doit être exposé");

    document
}

/// Treize chemins, quinze points d'entrée : `/auth/sessions` porte à la fois la liste et
/// la révocation globale, et `/auth/me` la lecture du profil et l'écriture de l'adresse.
/// `/users/filter` en ajoute un seizième, pour les écrans qui résolvent une référence vers
/// un compte. Un `#[utoipa::path]` qui disparaîtrait du document sans que cette suite
/// rougisse laisserait le client TypeScript, déduit de ce même document, en silence sur la
/// route perdue.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_openapi_document_carries_the_sixteen_auth_operations() {
    let api = application().await;

    let document = openapi_document(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/registration",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/auth/me",
        "/auth/change-password",
        "/auth/forgot-password",
        "/auth/reset-password",
        "/auth/verify-email",
        "/auth/resend-verification",
        "/auth/sessions",
        "/auth/sessions/{id}",
        "/users/filter",
    ] {
        assert!(
            document["paths"][chemin].is_object(),
            "`{chemin}` ne figure pas dans le document"
        );
    }

    for (chemin, methodes) in [
        ("/auth/sessions", ["get", "delete"]),
        ("/auth/me", ["get", "patch"]),
    ] {
        for methode in methodes {
            assert!(
                document["paths"][chemin][methode].is_object(),
                "`{methode} {chemin}` ne figure pas dans le document"
            );
        }
    }
}

/// Un client généré depuis ce document doit savoir comment s'authentifier, et sur quelles
/// routes le faire.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_bearer_scheme_is_declared_and_me_carries_it() {
    let api = application().await;

    let document = openapi_document(&api).await;

    let schema = &document["components"]["securitySchemes"]["bearer"];
    assert_eq!(schema["type"], "http", "{schema}");
    assert_eq!(schema["scheme"], "bearer", "{schema}");
    assert_eq!(schema["bearerFormat"], "JWT", "{schema}");

    for methode in ["get", "patch"] {
        let securite = &document["paths"]["/auth/me"][methode]["security"];
        assert!(
            securite.is_array() && !securite.as_array().expect("tableau").is_empty(),
            "`{methode} /auth/me` ne déclare pas exiger de jeton : {securite}"
        );
        assert!(
            securite[0]["bearer"].is_array(),
            "`{methode} /auth/me` n'exige pas le schéma `bearer` : {securite}"
        );
    }
}

/// `refresh` et `logout` s'authentifient par leur corps : leur apposer le schéma
/// décrirait une exigence que le serveur ne pose pas.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_routes_without_a_header_do_not_declare_the_scheme() {
    let api = application().await;

    let document = openapi_document(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
    ] {
        assert!(
            document["paths"][chemin]["post"]["security"].is_null(),
            "`{chemin}` déclare exiger un jeton alors qu'il n'en lit aucun"
        );
    }
}
