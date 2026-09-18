//! Déclaration unique des réponses d'erreur du document OpenAPI.
//!
//! Le projet généré accroche [`CommonResponses`] une fois sur son `#[derive(OpenApi)]` ;
//! ses handlers n'ont alors plus à répéter les réponses que toute opération partage.

use std::collections::BTreeMap;

use crate::lang::{self, Lang};
use serde::Serialize;
use utoipa::openapi::path::Operation;
#[cfg(feature = "auth")]
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
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

/// Réponses enregistrées sous `components/responses`, avec leur description dans `lang`.
fn named(lang: Lang) -> [(&'static str, &'static str); 6] {
    match lang {
        Lang::En => [
            ("BadRequest", "malformed request"),
            ("Unauthorized", "authentication required"),
            ("Forbidden", "access forbidden"),
            ("NotFound", "resource not found"),
            (
                "Conflict",
                "conflict with the current state of the resource",
            ),
            ("TooManyRequests", "too many requests"),
        ],
        Lang::Fr => [
            ("BadRequest", "requête mal formée"),
            ("Unauthorized", "authentification requise"),
            ("Forbidden", "accès interdit"),
            ("NotFound", "ressource introuvable"),
            ("Conflict", "conflit avec l'état courant de la ressource"),
            ("TooManyRequests", "trop de requêtes"),
        ],
    }
}

/// Nom du schéma de sécurité, tel que les handlers le référencent dans `security(...)`.
#[cfg(feature = "auth")]
pub const SCHEME_NAME: &str = "bearer";

/// Nom du schéma de la clé d'API, tel que les handlers le référencent dans `security(...)`.
#[cfg(feature = "auth")]
pub const KEY_SCHEME_NAME: &str = "api_key";

/// Réponses ajoutées d'office à chaque opération, avec leur description dans `lang`.
fn universal(lang: Lang) -> [(&'static str, &'static str); 2] {
    match lang {
        Lang::En => [
            ("422", "validation failed, detailed per field"),
            ("500", "internal error"),
        ],
        Lang::Fr => [
            ("422", "échec de validation, détaillé par champ"),
            ("500", "erreur interne"),
        ],
    }
}

impl Modify for CommonResponses {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        declare(openapi, lang::current());
    }
}

/// Applique à `openapi` les réponses communes et les descriptions dans `lang`.
///
/// Séparée de [`Modify::modify`] pour rester testable dans une langue choisie, sans
/// passer par le global de processus que les tests parallèles partagent.
fn declare(openapi: &mut utoipa::openapi::OpenApi, lang: Lang) {
    let composants = openapi.components.get_or_insert_with(Default::default);
    composants
        .schemas
        .entry(ProblemDetails::name().into_owned())
        .or_insert_with(ProblemDetails::schema);
    for (name, description) in named(lang) {
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

    // Le second justificatif que l'extracteur accepte. Déclaré inconditionnellement avec
    // `auth` : le noyau ne sait pas si le projet a installé le fragment `api-keys`, et un
    // schéma déclaré qu'aucune opération ne cite ne coûte qu'une ligne au document.
    #[cfg(feature = "auth")]
    composants.add_security_scheme(
        KEY_SCHEME_NAME,
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
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
            complete(operation, lang);
        }
    }
}

/// Ajoute à `operation` les réponses universelles qui lui manquent, décrites dans `lang`.
///
/// Seulement celles qui manquent : un handler qui documente son propre 422 en sait plus
/// sur son cas que le noyau, et sa description ne doit pas être écrasée.
fn complete(operation: &mut Operation, lang: Lang) {
    for (statut, description) in universal(lang) {
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
            (status = 422, description = "le nom est déjà pris"),
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

    /// Le même handler, sans le modificateur : `declare` s'y applique à la main, dans la
    /// langue voulue, sans passer par le global que les tests parallèles partagent.
    #[derive(OpenApi)]
    #[openapi(paths(list_all))]
    struct Bare;

    fn declared(lang: Lang) -> Value {
        let mut openapi = Bare::openapi();
        declare(&mut openapi, lang);
        serde_json::to_value(openapi).expect("document sérialisable")
    }

    #[test]
    fn in_english_the_common_descriptions_are_english() {
        let doc = declared(Lang::En);
        assert_eq!(
            doc["components"]["responses"]["NotFound"]["description"],
            "resource not found"
        );
        assert_eq!(
            doc["components"]["responses"]["TooManyRequests"]["description"],
            "too many requests"
        );
        assert_eq!(
            doc["paths"]["/things"]["get"]["responses"]["422"]["description"],
            "validation failed, detailed per field"
        );
        assert_eq!(
            doc["paths"]["/things"]["get"]["responses"]["500"]["description"],
            "internal error"
        );
    }

    #[test]
    fn in_french_the_common_descriptions_are_unchanged() {
        let doc = declared(Lang::Fr);
        assert_eq!(
            doc["components"]["responses"]["NotFound"]["description"],
            "ressource introuvable"
        );
        assert_eq!(
            doc["paths"]["/things"]["get"]["responses"]["422"]["description"],
            "échec de validation, détaillé par champ"
        );
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
            responses["422"]["description"], "le nom est déjà pris",
            "le handler qui documente son 422 doit garder le sien : {responses}"
        );
        assert!(responses.get("500").is_some(), "500 attendu : {responses}");
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

    /// Le document dit comment présenter une clé, comme il dit comment présenter un jeton :
    /// une opération qui annonce 401 sans nommer le justificatif laisse le client deviner.
    #[cfg(feature = "auth")]
    #[test]
    fn the_api_key_security_scheme_is_declared() {
        let doc = document();

        let schema = &doc["components"]["securitySchemes"][KEY_SCHEME_NAME];

        assert_eq!(schema["type"], "apiKey", "{schema}");
        assert_eq!(schema["in"], "header", "{schema}");
        assert_eq!(schema["name"], "X-Api-Key", "{schema}");
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
            "TooManyRequests",
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
                "champ `{field}` absent du schéma : {properties}"
            );
        }
    }
}
