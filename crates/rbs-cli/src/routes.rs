//! `rbs routes` : les routes du projet, lues dans son document OpenAPI.
//!
//! Le document plutôt que les sources : il porte, pour chaque opération, son
//! `operation_id` et l'exigence de jeton que son annotation déclare — ce que le router
//! ne dit pas.

use std::path::Path;

use serde::Serialize;

use crate::client::document::{self, Document};
use crate::openapi;

/// Ce qu'une route exige de qui l'appelle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Garde {
    /// Un jeton : l'opération, ou à défaut le document, déclare une `security`.
    Bearer,
    /// Rien.
    Public,
}

/// Une opération du document, réduite à ce qu'on en lit d'un coup d'œil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Route {
    pub methode: String,
    pub chemin: String,
    pub operation_id: Option<String>,
    pub garde: Garde,
}

/// L'ordre des méthodes sous un même chemin : lire, créer, remplacer, modifier, supprimer.
///
/// Les autres verbes viennent après, dans l'ordre fixe où le document les a livrés.
const ORDRE: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

fn rang(methode: &str) -> usize {
    ORDRE
        .iter()
        .position(|connue| *connue == methode)
        .unwrap_or(ORDRE.len())
}

/// Les routes du document, triées par chemin puis par méthode.
pub(crate) fn lister(document: &Document) -> Vec<Route> {
    let mut routes: Vec<Route> = document
        .paths
        .iter()
        .flat_map(|(chemin, item)| {
            item.operations
                .iter()
                .map(move |(methode, operation)| Route {
                    methode: methode.clone(),
                    chemin: chemin.clone(),
                    operation_id: operation.operation_id.clone(),
                    garde: if operation.secured {
                        Garde::Bearer
                    } else {
                        Garde::Public
                    },
                })
        })
        .collect();

    // Stable : deux verbes hors de `ORDRE` gardent l'ordre que l'analyse leur a donné.
    routes.sort_by(|a, b| {
        a.chemin
            .cmp(&b.chemin)
            .then_with(|| rang(&a.methode).cmp(&rang(&b.methode)))
    });

    routes
}

/// Le tableau des routes, colonnes alignées.
pub(crate) fn table(routes: &[Route]) -> String {
    let lignes: Vec<[String; 4]> =
        std::iter::once(["MÉTHODE", "CHEMIN", "OPERATION_ID", "GARDE"].map(String::from))
            .chain(routes.iter().map(|route| {
                [
                    route.methode.clone(),
                    route.chemin.clone(),
                    route
                        .operation_id
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                    match route.garde {
                        Garde::Bearer => "bearer",
                        Garde::Public => "public",
                    }
                    .to_string(),
                ]
            }))
            .collect();

    // En caractères et non en octets : « MÉTHODE » porte un É de deux octets.
    let largeur = |colonne: usize| {
        lignes
            .iter()
            .map(|ligne| ligne[colonne].chars().count())
            .max()
            .unwrap_or(0)
    };
    let largeurs = [largeur(0), largeur(1), largeur(2)];

    lignes
        .iter()
        .map(|[methode, chemin, operation_id, garde]| {
            format!(
                "{methode:<w0$}  {chemin:<w1$}  {operation_id:<w2$}  {garde}",
                w0 = largeurs[0],
                w1 = largeurs[1],
                w2 = largeurs[2],
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Les routes en JSON, seul document de la sortie standard.
pub(crate) fn json(routes: &[Route]) -> String {
    // Ni carte à clés non textuelles ni flottant : la sérialisation ne peut échouer que
    // sur un défaut de programmation, qu'il vaut mieux voir tomber ici.
    serde_json::to_string_pretty(routes).expect("les routes se sérialisent")
}

/// Les routes du projet qui contient `directory`, rendues en tableau ou en JSON.
pub(crate) fn run(directory: &Path, json: bool) -> Result<String, openapi::Error> {
    let texte = openapi::document(directory)?;
    let routes = lister(&document::parse(&texte)?);

    Ok(if json {
        self::json(&routes)
    } else {
        table(&routes)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un document fixe : des chemins écrits dans le désordre, des verbes dans l'ordre
    /// arbitraire du texte JSON, une opération sans `operationId`, et les deux gardes.
    const DOCUMENT: &str = r#"{
        "openapi": "3.1.0",
        "paths": {
            "/users/{id}": {
                "delete": {"operationId": "users_delete", "security": [{"bearer": []}], "responses": {}},
                "patch": {"operationId": "users_update", "security": [{"bearer": []}], "responses": {}},
                "get": {"operationId": "users_find", "security": [{"bearer": []}], "responses": {}},
                "put": {"security": [{"bearer": []}], "responses": {}}
            },
            "/users": {
                "post": {"operationId": "users_create", "security": [{"bearer": []}], "responses": {}},
                "get": {"operationId": "users_list", "security": [{"bearer": []}], "responses": {}}
            },
            "/health": {
                "get": {"operationId": "health", "responses": {}}
            }
        }
    }"#;

    fn routes() -> Vec<Route> {
        lister(&document::parse(DOCUMENT).expect("le document fixe s'analyse"))
    }

    #[test]
    fn the_routes_are_sorted_by_path_then_by_method() {
        let ordre: Vec<(String, String)> = routes()
            .into_iter()
            .map(|route| (route.methode, route.chemin))
            .collect();

        let attendu: Vec<(String, String)> = [
            ("GET", "/health"),
            ("GET", "/users"),
            ("POST", "/users"),
            ("GET", "/users/{id}"),
            ("PUT", "/users/{id}"),
            ("PATCH", "/users/{id}"),
            ("DELETE", "/users/{id}"),
        ]
        .into_iter()
        .map(|(methode, chemin)| (methode.to_string(), chemin.to_string()))
        .collect();

        assert_eq!(ordre, attendu);
    }

    #[test]
    fn a_declared_security_makes_the_route_bearer_and_its_absence_public() {
        let routes = routes();

        assert_eq!(routes[0].chemin, "/health");
        assert_eq!(routes[0].garde, Garde::Public);
        assert!(
            routes[1..].iter().all(|route| route.garde == Garde::Bearer),
            "{routes:?}"
        );
    }

    /// Les colonnes s'alignent sur leur plus longue valeur, en caractères : « MÉTHODE »
    /// porte un É de deux octets, qu'un calcul en octets décalerait d'un cran.
    #[test]
    fn the_table_aligns_its_columns_and_marks_a_missing_operation_id() {
        let attendu = "\
MÉTHODE  CHEMIN       OPERATION_ID  GARDE
GET      /health      health        public
GET      /users       users_list    bearer
POST     /users       users_create  bearer
GET      /users/{id}  users_find    bearer
PUT      /users/{id}  -             bearer
PATCH    /users/{id}  users_update  bearer
DELETE   /users/{id}  users_delete  bearer";

        assert_eq!(table(&routes()), attendu);
    }

    #[test]
    fn no_line_of_the_table_ends_with_a_space() {
        for ligne in table(&routes()).lines() {
            assert!(!ligne.ends_with(' '), "« {ligne} »");
        }
    }

    /// Les quatre clés sont toujours là, `operation_id` à `null` quand l'opération n'en
    /// déclare pas : un lecteur n'a pas à distinguer une clé absente d'une valeur absente.
    #[test]
    fn the_json_carries_the_four_keys_of_every_route() {
        let rendu: serde_json::Value =
            serde_json::from_str(&json(&routes())).expect("le rendu doit être un JSON valide");
        let routes = rendu.as_array().expect("le rendu est un tableau");

        assert_eq!(routes.len(), 7);
        assert_eq!(
            routes[0],
            serde_json::json!({
                "methode": "GET",
                "chemin": "/health",
                "operation_id": "health",
                "garde": "public"
            })
        );
        assert_eq!(routes[4]["operation_id"], serde_json::Value::Null);
        assert_eq!(routes[4]["garde"], "bearer");
    }

    #[test]
    fn a_document_without_paths_gives_a_table_reduced_to_its_header() {
        let vide = lister(&document::parse(r#"{"openapi":"3.1.0"}"#).expect("document valide"));

        assert_eq!(table(&vide), "MÉTHODE  CHEMIN  OPERATION_ID  GARDE");
        assert_eq!(json(&vide), "[]");
    }
}
