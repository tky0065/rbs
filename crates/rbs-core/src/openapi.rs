//! Déclaration unique des réponses d'erreur du document OpenAPI.
//!
//! Le projet généré accroche [`ReponsesCommunes`] une fois sur son `#[derive(OpenApi)]` ;
//! ses handlers n'ont alors plus à répéter les réponses que toute opération partage.

use std::collections::BTreeMap;

use serde::Serialize;
use utoipa::openapi::path::Operation;
use utoipa::openapi::{Content, RefOr, Response, ResponseBuilder, Schema};
use utoipa::{Modify, PartialSchema, ToSchema};

/// Type de média des réponses d'erreur, conformément à la RFC 9457.
const PROBLEM_JSON: &str = "application/problem+json";

/// Corps de réponse RFC 9457. Les champs absents ne sont pas sérialisés.
///
/// Ce type décrit le corps d'erreur *et* le produit : les deux ne peuvent donc pas
/// diverger, ce qui arriverait avec un schéma OpenAPI rédigé à côté du code.
#[derive(Debug, Serialize, ToSchema)]
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
pub struct ReponsesCommunes;

/// Réponses enregistrées sous `components/responses`, avec leur description.
const NOMMEES: [(&str, &str); 5] = [
    ("BadRequest", "requête mal formée"),
    ("Unauthorized", "authentification requise"),
    ("Forbidden", "accès interdit"),
    ("NotFound", "ressource introuvable"),
    ("Conflict", "conflit avec l'état courant de la ressource"),
];

/// Réponses ajoutées d'office à chaque opération.
const UNIVERSELLES: [(&str, &str); 2] = [
    ("422", "échec de validation, détaillé par champ"),
    ("500", "erreur interne"),
];

impl Modify for ReponsesCommunes {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let composants = openapi.components.get_or_insert_with(Default::default);
        composants
            .schemas
            .entry(ProblemDetails::name().into_owned())
            .or_insert_with(ProblemDetails::schema);
        for (nom, description) in NOMMEES {
            composants
                .responses
                .entry(nom.to_owned())
                .or_insert_with(|| probleme(description).into());
        }

        for chemin in openapi.paths.paths.values_mut() {
            // `PathItem` expose une option par verbe plutôt qu'une table : les parcourir
            // tous est le seul moyen d'atteindre chaque opération déclarée.
            let operations = [
                &mut chemin.get,
                &mut chemin.put,
                &mut chemin.post,
                &mut chemin.delete,
                &mut chemin.options,
                &mut chemin.head,
                &mut chemin.patch,
                &mut chemin.trace,
            ];
            for operation in operations.into_iter().flatten() {
                completer(operation);
            }
        }
    }
}

/// Ajoute à `operation` les réponses universelles qui lui manquent.
///
/// Seulement celles qui manquent : un handler qui documente son propre 422 en sait plus
/// sur son cas que le noyau, et sa description ne doit pas être écrasée.
fn completer(operation: &mut Operation) {
    for (statut, description) in UNIVERSELLES {
        if operation.responses.responses.contains_key(statut) {
            continue;
        }
        operation
            .responses
            .responses
            .insert(statut.to_owned(), probleme(description).into());
    }
}

/// Construit une réponse dont le corps est un [`ProblemDetails`].
fn probleme(description: &str) -> Response {
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
    fn lister() {}

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
    fn creer() {}

    #[derive(OpenApi)]
    #[openapi(paths(lister, creer), modifiers(&ReponsesCommunes))]
    struct Doc;

    fn document() -> Value {
        serde_json::to_value(Doc::openapi()).expect("document sérialisable")
    }

    #[test]
    fn le_document_decrit_422_et_500_sans_annotation_par_handler() {
        let doc = document();

        let reponses = &doc["paths"]["/things"]["get"]["responses"];
        assert!(
            reponses.get("422").is_some(),
            "422 absent du document : {reponses}"
        );
        assert!(
            reponses.get("500").is_some(),
            "500 absent du document : {reponses}"
        );
    }

    #[test]
    fn une_reponse_declaree_par_le_handler_n_est_pas_ecrasee() {
        let doc = document();

        let reponses = &doc["paths"]["/things"]["post"]["responses"];
        assert_eq!(
            reponses["422"]["description"], "le nom est déjà pris",
            "le handler qui documente son 422 doit garder le sien : {reponses}"
        );
        assert!(reponses.get("500").is_some(), "500 attendu : {reponses}");
    }

    #[test]
    fn les_reponses_communes_sont_referencables_par_nom() {
        let doc = document();

        let communes = &doc["components"]["responses"];
        for nom in [
            "BadRequest",
            "Unauthorized",
            "Forbidden",
            "NotFound",
            "Conflict",
        ] {
            assert!(
                communes.get(nom).is_some(),
                "réponse commune `{nom}` absente : {communes}"
            );
        }
    }

    #[test]
    fn le_schema_du_probleme_decrit_les_champs_rfc_9457() {
        let doc = document();

        let proprietes = &doc["components"]["schemas"]["ProblemDetails"]["properties"];
        for champ in ["type", "title", "status", "detail", "errors", "request_id"] {
            assert!(
                proprietes.get(champ).is_some(),
                "champ `{champ}` absent du schéma : {proprietes}"
            );
        }
    }
}
