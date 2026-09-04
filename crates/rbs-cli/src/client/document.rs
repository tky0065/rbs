//! Le document OpenAPI d'un projet, lu en un modèle Rust propre.
//!
//! L'analyse passe par `serde_json::Value` et non par des `#[derive(Deserialize)]` :
//! OpenAPI 3.1 écrit le type d'un champ tantôt en chaîne, tantôt en tableau
//! (`["string","null"]`) pour dire qu'il est nullable, et un `enum` serde pour ce seul
//! cas coûterait plus qu'une conversion explicite, fonction par fonction.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Le document, réduit aux chemins et aux schémas de composants — tout ce qu'un client
/// a besoin de lire.
#[derive(Debug)]
pub(crate) struct Document {
    pub paths: BTreeMap<String, PathItem>,
    pub schemas: BTreeMap<String, Schema>,
}

/// Les opérations déclarées sur un chemin.
#[derive(Debug)]
pub(crate) struct PathItem {
    /// Les opérations de ce chemin, méthode HTTP en majuscules.
    pub operations: Vec<(String, Operation)>,
}

/// Une opération : une méthode HTTP sur un chemin.
#[derive(Debug)]
pub(crate) struct Operation {
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<Schema>,
    /// Statut → schéma du corps, `None` quand la réponse n'a pas de contenu.
    pub responses: BTreeMap<u16, Option<Schema>>,
    pub secured: bool,
}

/// Un paramètre de chemin, de requête, ou d'un autre emplacement que le client ignore.
// Champs seulement écrits par `parse_parameter` tant qu'aucun appelant ne les lit :
// tombe avec le lot qui traduit ce modèle en méthodes du client.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Parameter {
    pub name: String,
    pub location: Location,
    pub description: Option<String>,
    pub required: bool,
    pub schema: Schema,
}

/// L'emplacement d'un paramètre, tel que porté par le champ `in` du document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Location {
    Path,
    Query,
    Autre(String),
}

/// Un schéma, tel que le document l'écrit — aucune résolution, aucun jugement.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Schema {
    /// Le nom du composant, `#/components/schemas/` retiré.
    Ref(String),
    Primitive {
        kind: String,
        nullable: bool,
        enumeration: Vec<String>,
    },
    Array {
        items: Box<Schema>,
        nullable: bool,
    },
    Object {
        /// L'ordre rendu par `serde_json` : alphabétique en l'absence de la feature
        /// `preserve_order`, que le workspace n'active pas — pas l'ordre du texte du
        /// document.
        properties: Vec<(String, Schema)>,
        required: BTreeSet<String>,
        additional: Option<Box<Schema>>,
        nullable: bool,
        description: Option<String>,
    },
    /// `oneOf`, `anyOf`.
    Union(Vec<Schema>),
    /// `allOf`.
    Intersection(Vec<Schema>),
    Inconnu,
}

/// Les erreurs que peut rendre `parse`.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    #[error("la sortie du binaire openapi n'est pas du JSON : {0}")]
    Json(#[from] serde_json::Error),
    #[error("la sortie du binaire openapi n'est pas un document OpenAPI : `{champ}` manque")]
    ChampManquant { champ: &'static str },
}

/// Analyse la sortie du binaire `openapi` d'un projet en un modèle exploitable.
// Aucun appelant tant que `generate client` n'existe pas : tombe avec la commande.
#[allow(dead_code)]
pub(crate) fn parse(json: &str) -> Result<Document, Erreur> {
    let value: Value = serde_json::from_str(json)?;
    if value.get("openapi").is_none() {
        return Err(Erreur::ChampManquant { champ: "openapi" });
    }

    let schemas = value
        .pointer("/components/schemas")
        .and_then(Value::as_object)
        .map(|schemas| {
            schemas
                .iter()
                .map(|(nom, schema)| (nom.clone(), parse_schema(schema)))
                .collect()
        })
        .unwrap_or_default();

    let paths = value
        .get("paths")
        .and_then(Value::as_object)
        .map(|paths| {
            paths
                .iter()
                .map(|(chemin, item)| (chemin.clone(), parse_path_item(item)))
                .collect()
        })
        .unwrap_or_default();

    Ok(Document { paths, schemas })
}

/// Les méthodes HTTP qu'OpenAPI autorise sous un chemin, dans un ordre fixe — celui du
/// document ne porte aucun sens, `paths` étant un objet JSON.
const VERBES: &[&str] = &[
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

fn parse_path_item(value: &Value) -> PathItem {
    let Some(item) = value.as_object() else {
        return PathItem {
            operations: Vec::new(),
        };
    };

    let operations = VERBES
        .iter()
        .filter_map(|verbe| {
            item.get(*verbe)
                .map(|operation| (verbe.to_uppercase(), parse_operation(operation)))
        })
        .collect();

    PathItem { operations }
}

fn parse_operation(value: &Value) -> Operation {
    let operation_id = texte(value, "operationId");
    let summary = texte(value, "summary");
    let description = texte(value, "description");

    let parameters = value
        .get("parameters")
        .and_then(Value::as_array)
        .map(|parametres| parametres.iter().map(parse_parameter).collect())
        .unwrap_or_default();

    let request_body = value
        .pointer("/requestBody/content/application~1json/schema")
        .map(parse_schema);

    let responses = value
        .get("responses")
        .and_then(Value::as_object)
        .map(|responses| {
            responses
                .iter()
                .filter_map(|(statut, reponse)| {
                    statut
                        .parse::<u16>()
                        .ok()
                        .map(|statut| (statut, parse_response_body(reponse)))
                })
                .collect()
        })
        .unwrap_or_default();

    // Une opération sans clé `security` hérite de celle, globale, du document — que ce
    // lot ne lit pas encore. Seule une liste explicite et non vide la marque protégée.
    let secured = value
        .get("security")
        .and_then(Value::as_array)
        .is_some_and(|exigences| !exigences.is_empty());

    Operation {
        operation_id,
        summary,
        description,
        parameters,
        request_body,
        responses,
        secured,
    }
}

fn parse_response_body(value: &Value) -> Option<Schema> {
    // `application/problem+json` décrit le corps d'erreur, que le client ne rend jamais
    // — il le jette dans `ApiError` plutôt que de le typer.
    value
        .pointer("/content/application~1json/schema")
        .map(parse_schema)
}

fn parse_parameter(value: &Value) -> Parameter {
    let name = texte(value, "name").unwrap_or_default();
    let location = match value.get("in").and_then(Value::as_str) {
        Some("path") => Location::Path,
        Some("query") => Location::Query,
        Some(autre) => Location::Autre(autre.to_string()),
        None => Location::Autre(String::new()),
    };
    let description = texte(value, "description");
    let required = value
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let schema = value
        .get("schema")
        .map(parse_schema)
        .unwrap_or(Schema::Inconnu);

    Parameter {
        name,
        location,
        description,
        required,
        schema,
    }
}

fn parse_schema(value: &Value) -> Schema {
    if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
        return Schema::Ref(nom_du_composant(reference));
    }
    if let Some(variantes) = value
        .get("oneOf")
        .or_else(|| value.get("anyOf"))
        .and_then(Value::as_array)
    {
        return Schema::Union(variantes.iter().map(parse_schema).collect());
    }
    if let Some(variantes) = value.get("allOf").and_then(Value::as_array) {
        return Schema::Intersection(variantes.iter().map(parse_schema).collect());
    }

    let (kind, nullable) = parse_type(value);
    match kind.as_deref() {
        Some("object") => parse_object(value, nullable),
        Some("array") => {
            let items = value
                .get("items")
                .map(|items| Box::new(parse_schema(items)))
                .unwrap_or_else(|| Box::new(Schema::Inconnu));
            Schema::Array { items, nullable }
        }
        Some(kind) => Schema::Primitive {
            kind: kind.to_string(),
            nullable,
            enumeration: value
                .get("enum")
                .and_then(Value::as_array)
                .map(|valeurs| {
                    valeurs
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        },
        // Pas de "type" : un schéma qui porte `properties` sans le dire est un objet
        // implicite qu'utoipa peut produire ; le reste est hors périmètre de ce lot.
        None if value.get("properties").is_some() => parse_object(value, nullable),
        None => Schema::Inconnu,
    }
}

/// Retire le préfixe `#/components/schemas/` d'une référence, dont seul le nom du
/// composant survit dans le modèle.
fn nom_du_composant(reference: &str) -> String {
    reference
        .rsplit('/')
        .next()
        .unwrap_or(reference)
        .to_string()
}

/// Le `type` d'un schéma OpenAPI 3.1 : une chaîne, ou un tableau contenant `"null"`
/// pour dire nullable et l'autre entrée pour le type effectif.
fn parse_type(value: &Value) -> (Option<String>, bool) {
    match value.get("type") {
        Some(Value::String(kind)) => (Some(kind.clone()), false),
        Some(Value::Array(kinds)) => {
            let nullable = kinds.iter().any(|kind| kind.as_str() == Some("null"));
            let kind = kinds
                .iter()
                .filter_map(Value::as_str)
                .find(|kind| *kind != "null")
                .map(str::to_string);
            (kind, nullable)
        }
        _ => (None, false),
    }
}

fn parse_object(value: &Value, nullable: bool) -> Schema {
    let properties = value
        .get("properties")
        .and_then(Value::as_object)
        .map(|proprietes| {
            proprietes
                .iter()
                .map(|(nom, schema)| (nom.clone(), parse_schema(schema)))
                .collect()
        })
        .unwrap_or_default();

    let required = value
        .get("required")
        .and_then(Value::as_array)
        .map(|noms| {
            noms.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let additional = match value.get("additionalProperties") {
        Some(Value::Bool(false)) | None => None,
        Some(Value::Bool(true)) => Some(Box::new(Schema::Inconnu)),
        Some(schema) => Some(Box::new(parse_schema(schema))),
    };

    Schema::Object {
        properties,
        required,
        additional,
        nullable,
        description: texte(value, "description"),
    }
}

fn texte(value: &Value, champ: &str) -> Option<String> {
    value.get(champ).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(json: &str) -> Document {
        parse(json).expect("le document doit s'analyser")
    }

    #[test]
    fn a_nullable_string_is_read_as_a_nullable_primitive() {
        let document = parse_ok(
            r#"{"openapi":"3.1.0","components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"type":["string","null"]}}}}}}"#,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        let (_, champ) = &properties[0];
        assert!(
            matches!(champ, Schema::Primitive { kind, nullable: true, .. } if kind == "string"),
            "{champ:?}"
        );
    }

    #[test]
    fn a_reference_keeps_only_the_component_name() {
        let document = parse_ok(
            r##"{"openapi":"3.1.0","components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"$ref":"#/components/schemas/Autre"}}}}}}"##,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        assert_eq!(properties[0].1, Schema::Ref("Autre".to_string()));
    }

    // Le workspace n'active pas la feature `preserve_order` de `serde_json` (voir
    // `Cargo.toml`) : `Map` y est une `BTreeMap`, et le document sorti d'utoipa est de
    // toute façon déjà trié — relevé sur une sortie réelle, `CreatePost` rend
    // `body, published, title` quand le DTO déclare `title, body, published`. Rien
    // n'est donc perdu à lire les propriétés dans cet ordre plutôt que dans celui,
    // arbitraire, du texte JSON.
    #[test]
    fn the_properties_are_read_in_a_stable_alphabetical_order() {
        let document = parse_ok(
            r#"{"openapi":"3.1.0","components":{"schemas":{"S":{"type":"object","properties":{
                 "z":{"type":"string"},"a":{"type":"string"}}}}}}"#,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        let noms: Vec<&str> = properties.iter().map(|(nom, _)| nom.as_str()).collect();
        assert_eq!(noms, ["a", "z"]);
    }

    #[test]
    fn an_operation_carries_its_verb_in_upper_case() {
        let document = parse_ok(
            r#"{"openapi":"3.1.0","paths":{"/a":{"get":{"operationId":"lire","responses":{}},
                               "post":{"operationId":"ecrire","responses":{}}}}}"#,
        );

        let verbes: Vec<&str> = document.paths["/a"]
            .operations
            .iter()
            .map(|(verbe, _)| verbe.as_str())
            .collect();
        assert_eq!(verbes, ["GET", "POST"]);
    }

    #[test]
    fn a_response_without_content_is_read_as_a_status_without_schema() {
        let document = parse_ok(
            r#"{"openapi":"3.1.0","paths":{"/a":{"delete":{"operationId":"d","responses":{
                 "204":{"description":"supprimé"}}}}}}"#,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert_eq!(operation.responses[&204], None);
    }

    #[test]
    fn only_an_application_json_body_is_read() {
        let document = parse_ok(
            r##"{"openapi":"3.1.0","paths":{"/a":{"post":{"operationId":"p","responses":{"200":{
                 "description":"ok","content":{"application/problem+json":{
                   "schema":{"$ref":"#/components/schemas/ProblemDetails"}}}}}}}}}"##,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert_eq!(operation.responses[&200], None);
    }

    #[test]
    fn a_security_requirement_marks_the_operation() {
        let document = parse_ok(
            r#"{"openapi":"3.1.0","paths":{"/a":{"get":{"operationId":"g","responses":{},
                 "security":[{"bearer":[]}]}}}}"#,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert!(operation.secured);
    }

    #[test]
    fn a_payload_that_is_not_json_is_refused() {
        assert!(matches!(parse("pas du json"), Err(Erreur::Json(_))));
    }

    #[test]
    fn a_json_that_is_not_a_document_is_refused() {
        assert!(matches!(
            parse("[]"),
            Err(Erreur::ChampManquant { champ: "openapi" })
        ));
    }
}
