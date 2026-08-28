//! Déclaration unique des réponses d'erreur du document OpenAPI.
//!
//! Le projet généré accroche [`CommonResponses`] une fois sur son `#[derive(OpenApi)]` ;
//! ses handlers n'ont alors plus à répéter les réponses que toute opération partage.

use std::collections::BTreeMap;

use serde::Serialize;
use utoipa::openapi::path::Operation;
#[cfg(feature = "auth")]
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{Content, RefOr, Response, ResponseBuilder, Schema};
use utoipa::{Modify, PartialSchema, ToSchema};

/// Type de média des réponses d'erreur, conformément à la RFC 9457.
const PROBLEM_JSON: &str = "application/problem+json";

/// Corps de réponse RFC 9457. Les champs absents ne sont pas sérialisés.
///
/// Ce type décrit le corps d'erreur *et* le produit : les deux ne peuvent donc pas
/// diverger, ce qui arriverait avec un schéma OpenAPI rédigé à côté du code.
#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ProblemDetails {
    /// URI identifiant le type de problème.
    pub r#type: &'static str,
    /// Résumé lisible du problème.
    pub title: String,
    /// Statut HTTP de la réponse.
    pub status: u16,
    /// Explication propre à cette occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Détail par champ, sur un échec de validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<BTreeMap<String, Vec<String>>>,
    /// Identifiant de la requête, de quoi retrouver la ligne de journal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Réponses d'erreur du runtime, déclarées une fois pour tout le document.
///
/// Complète chaque opération des réponses 422 et 500 — les seules que *toute* opération
/// peut produire, le runtime validant partout et pouvant défaillir partout — et enregistre
/// les autres dans `components/responses`, référençables par nom depuis un handler.
#[derive(Debug, Clone, Copy)]
pub struct CommonResponses;

/// Réponses enregistrées sous `components/responses`, avec leur description.
const NAMED: [(&str, &str); 5] = [
    ("BadRequest", "requête mal formée"),
    ("Unauthorized", "authentification requise"),
    ("Forbidden", "accès interdit"),
    ("NotFound", "ressource introuvable"),
    ("Conflict", "conflit avec l'état courant de la ressource"),
];

/// Nom du schéma de sécurité, tel que les handlers le référencent dans `security(...)`.
#[cfg(feature = "auth")]
pub const SCHEME_NAME: &str = "bearer";

/// Réponses ajoutées d'office à chaque opération.
const UNIVERSAL: [(&str, &str); 2] = [
    ("422", "échec de validation, détaillé par field"),
    ("500", "error interne"),
];

impl Modify for CommonResponses {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let composants = openapi.components.get_or_insert_with(Default::default);
        composants
            .schemas
            .entry(ProblemDetails::name().into_owned())
            .or_insert_with(ProblemDetails::schema);
        for (name, description) in NAMED {
            composants
                .responses
                .entry(name.to_owned())
                .or_insert_with(|| problem(description).into());
        }

        // Le schéma accompagne les réponses 401 et 403 déclarées juste au-dessus : un
        // document qui les annonce sans dire comment s'authentifier laisse le client
        // deviner. Il ne s'ajoute que si l'authentification est compilée.
        #[cfg(feature = "auth")]
        composants.add_security_scheme(
            SCHEME_NAME,
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );

        for path in openapi.paths.paths.values_mut() {
            // `PathItem` expose une option par verbe plutôt qu'une table : les parcourir
            // tous est le seul moyen d'atteindre chaque opération déclarée.
            let operations = [
                &mut path.get,
                &mut path.put,
                &mut path.post,
                &mut path.delete,
                &mut path.options,
                &mut path.head,
                &mut path.patch,
                &mut path.trace,
            ];
            for operation in operations.into_iter().flatten() {
                complete(operation);
            }
        }
    }
}

/// Ajoute à `operation` les réponses universelles qui lui manquent.
///
/// Seulement celles qui manquent : un handler qui documente son propre 422 en sait plus
/// sur son cas que le noyau, et sa description ne doit pas être écrasée.
fn complete(operation: &mut Operation) {
    for (statut, description) in UNIVERSAL {
        if operation.responses.responses.contains_key(statut) {
            continue;
        }
        operation
            .responses
            .responses
            .insert(statut.to_owned(), problem(description).into());
    }
}

/// Construit une réponse dont le corps est un [`ProblemDetails`].
fn problem(description: &str) -> Response {
    ResponseBuilder::new()
        .description(description)
        .content(
            PROBLEM_JSON,
            Content::new(Some(RefOr::<Schema>::Ref(
                utoipa::openapi::Ref::from_schema_name(ProblemDetails::name()),
            ))),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use utoipa::OpenApi;

    /// Un handler documenté au strict minimum : ni 422 ni 500.
    #[utoipa::path(get, path = "/things", responses((status = 200, description = "ok")))]
    #[allow(dead_code)]
    fn list_all() {}

    /// Un handler qui documente lui-même son 422.
    #[utoipa::path(
        post,
        path = "/things",
        responses(
            (status = 201, description = "créé"),
            (status = 422, description = "le name est déjà pris"),
        )
    )]
    #[allow(dead_code)]
    fn create() {}

    #[derive(OpenApi)]
    #[openapi(paths(list_all, create), modifiers(&CommonResponses))]
    struct Doc;

    fn document() -> Value {
        serde_json::to_value(Doc::openapi()).expect("document sérialisable")
    }

    #[test]
    fn the_document_describes_422_and_500_without_per_handler_annotation() {
        let doc = document();

        let responses = &doc["paths"]["/things"]["get"]["responses"];
        assert!(
            responses.get("422").is_some(),
            "422 absent du document : {responses}"
        );
        assert!(
            responses.get("500").is_some(),
            "500 absent du document : {responses}"
        );
    }

    #[test]
    fn a_response_declared_by_the_handler_is_not_overwritten() {
        let doc = document();

        let responses = &doc["paths"]["/things"]["post"]["responses"];
        assert_eq!(
            responses["422"]["description"], "le name est déjà pris",
            "le handler qui documente son 422 doit garder le sien : {responses}"
        );
        assert!(responses.get("500").is_some(), "500 expected : {responses}");
    }

    /// Le document annonce 401 et 403 : sans ce schéma, il ne dit nulle part comment s'y
    /// conformer, et un client généré depuis lui n'a aucun moyen de le deviner.
    #[cfg(feature = "auth")]
    #[test]
    fn the_bearer_security_scheme_is_declared() {
        let doc = document();

        let schema = &doc["components"]["securitySchemes"][SCHEME_NAME];
        assert_eq!(schema["type"], "http", "{schema}");
        assert_eq!(schema["scheme"], "bearer", "{schema}");
        assert_eq!(schema["bearerFormat"], "JWT", "{schema}");
    }

    /// Le schéma se déclare, il ne s'impose pas : une opération qui ne l'a pas demandé ne
    /// doit pas se retrouver à exiger un jeton.
    #[cfg(feature = "auth")]
    #[test]
    fn the_declared_scheme_is_imposed_on_no_operation() {
        let doc = document();

        assert!(
            doc["paths"]["/things"]["get"]["security"].is_null(),
            "une opération sans `security` s'est vu imposer le schéma"
        );
    }

    #[test]
    fn the_common_responses_are_referenceable_by_name() {
        let doc = document();

        let common = &doc["components"]["responses"];
        for name in [
            "BadRequest",
            "Unauthorized",
            "Forbidden",
            "NotFound",
            "Conflict",
        ] {
            assert!(
                common.get(name).is_some(),
                "réponse commune `{name}` absente : {common}"
            );
        }
    }

    #[test]
    fn the_problem_schema_describes_the_rfc_9457_fields() {
        let doc = document();

        let properties = &doc["components"]["schemas"]["ProblemDetails"]["properties"];
        for field in ["type", "title", "status", "detail", "errors", "request_id"] {
            assert!(
                properties.get(field).is_some(),
                "field `{field}` absent du schéma : {properties}"
            );
        }
    }
}
