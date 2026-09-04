//! Traduction des schémas OpenAPI (`client::document::Schema`) en types TypeScript.
//!
//! `type_de` rend l'expression de type d'un schéma partout où il apparaît en position
//! de champ ou de paramètre ; `interfaces` rend en plus la déclaration de plus haut
//! niveau associée à chaque composant nommé du document.

use std::collections::{BTreeMap, BTreeSet};

use crate::client::document::{Document, Schema};

/// Le nom TypeScript d'un composant : coupe sur tout ce qui n'est pas alphanumérique,
/// capitalise la première lettre de chaque tronçon et recolle — `Page_PostResponse`
/// devient `PagePostResponse`. Un résultat qui commencerait par un chiffre reçoit un
/// `_` en tête, un chiffre ne pouvant ouvrir un identifiant TypeScript.
pub(crate) fn identifiant(nom: &str) -> String {
    let rendu: String = nom
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|tronc| !tronc.is_empty())
        .map(capitalise)
        .collect();

    match rendu.chars().next() {
        Some(premier) if premier.is_ascii_digit() => format!("_{rendu}"),
        _ => rendu,
    }
}

/// Met en majuscule la première lettre d'un tronçon, laisse le reste inchangé.
fn capitalise(tronc: &str) -> String {
    let mut chars = tronc.chars();
    match chars.next() {
        Some(premier) => premier.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// L'expression de type TypeScript d'un schéma, telle qu'elle apparaît dans une
/// signature ou comme type d'une propriété — un objet s'y rend sur une seule ligne.
pub(crate) fn type_de(schema: &Schema) -> String {
    match schema {
        Schema::Ref(nom) => identifiant(nom),
        Schema::Primitive {
            kind,
            nullable,
            enumeration,
        } => avec_nullable(primitif(kind, enumeration), *nullable),
        Schema::Array { items, nullable } => avec_nullable(tableau(items), *nullable),
        Schema::Object {
            properties,
            required,
            additional,
            nullable,
            ..
        } => avec_nullable(objet_inline(properties, required, additional), *nullable),
        Schema::Union(variantes) => variantes
            .iter()
            .map(type_de)
            .collect::<Vec<_>>()
            .join(" | "),
        Schema::Intersection(variantes) => variantes
            .iter()
            .map(type_de)
            .collect::<Vec<_>>()
            .join(" & "),
        Schema::Inconnu => "unknown".to_string(),
    }
}

/// `nullable` ajoute `| null` à une expression déjà construite.
fn avec_nullable(base: String, nullable: bool) -> String {
    if nullable {
        format!("{base} | null")
    } else {
        base
    }
}

/// Une énumération de chaînes se rend en union de littéraux ; sinon le type porté par
/// `kind` — `integer` et `number` se confondent, TypeScript n'ayant qu'un `number`.
fn primitif(kind: &str, enumeration: &[String]) -> String {
    if !enumeration.is_empty() {
        return enumeration
            .iter()
            .map(|valeur| format!("\"{valeur}\""))
            .collect::<Vec<_>>()
            .join(" | ");
    }
    match kind {
        "string" => "string",
        "integer" | "number" => "number",
        "boolean" => "boolean",
        _ => "unknown",
    }
    .to_string()
}

/// `T[]`, `T` parenthésé quand son rendu contient un espace — sans quoi `A | B[]` se
/// lirait comme un tableau de `B` unioné à `A`, plutôt que comme un tableau de `A | B`.
fn tableau(items: &Schema) -> String {
    let rendu = type_de(items);
    if rendu.contains(' ') {
        format!("({rendu})[]")
    } else {
        format!("{rendu}[]")
    }
}

/// L'expression inline d'un objet : `Record<string, T>` pour une carte,
/// `{ a: A; b?: B }` sur une seule ligne pour des propriétés déclarées, `Record<string,
/// unknown>` quand ni l'un ni l'autre n'est présent.
fn objet_inline(
    properties: &[(String, Schema)],
    required: &BTreeSet<String>,
    additional: &Option<Box<Schema>>,
) -> String {
    if let Some(cle_libre) = additional {
        return format!("Record<string, {}>", type_de(cle_libre));
    }
    if properties.is_empty() {
        return "Record<string, unknown>".to_string();
    }
    let membres: Vec<String> = properties
        .iter()
        .map(|(nom, schema)| membre(nom, schema, required))
        .collect();
    format!("{{ {} }}", membres.join("; "))
}

/// Le corps `{\n  … ;\n}` d'une interface nommée, une propriété par ligne — seul rendu
/// qui diffère de `type_de` : une interface est une déclaration, pas une expression.
fn objet_corps(properties: &[(String, Schema)], required: &BTreeSet<String>) -> String {
    let membres: Vec<String> = properties
        .iter()
        .map(|(nom, schema)| format!("  {};", membre(nom, schema, required)))
        .collect();
    format!("{{\n{}\n}}", membres.join("\n"))
}

/// `nom: Type` ou `nom?: Type` selon que `required` porte ce nom.
fn membre(nom: &str, schema: &Schema, required: &BTreeSet<String>) -> String {
    let marque = if required.contains(nom) { "" } else { "?" };
    format!("{nom}{marque}: {}", type_de(schema))
}

/// Le corps d'une interface pour un schéma de composant : identique à `type_de`, sauf
/// pour un objet à propriétés déclarées, qui se rend alors sur plusieurs lignes — la
/// seule différence entre les deux rendus, l'un étant une expression et l'autre une
/// déclaration.
fn corps_de(schema: &Schema) -> String {
    match schema {
        Schema::Object {
            properties,
            required,
            additional: None,
            ..
        } if !properties.is_empty() => objet_corps(properties, required),
        _ => type_de(schema),
    }
}

/// Un commentaire `/** … */` prêt à écrire au-dessus d'une interface, chaque ligne de
/// la description préfixée de ` * ` au-delà de la première.
fn commentaire(description: &str) -> String {
    let lignes: Vec<&str> = description.lines().collect();
    match lignes.as_slice() {
        [] => format!("/** {description} */"),
        [seule] => format!("/** {seule} */"),
        plusieurs => {
            let corps: Vec<String> = plusieurs
                .iter()
                .map(|ligne| format!(" * {ligne}"))
                .collect();
            format!("/**\n{}\n */", corps.join("\n"))
        }
    }
}

/// Une interface prête à écrire : nom déjà passé par `identifiant`, doc déjà formatée
/// en `/** … */`, corps déjà rendu par `corps_de`.
// Construite par `interfaces`, qu'aucune commande n'appelle encore : tombe avec le lot
// qui assemble le fichier client et y écrit chaque interface.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Interface {
    pub nom: String,
    pub doc: Option<String>,
    pub corps: String,
}

/// Traduit chaque schéma de composant du document en `Interface`.
///
/// Deux vérifications précèdent le rendu, plutôt que de laisser un rendu partiel
/// masquer l'erreur : que tout `$ref` cible un composant déclaré, et qu'aucun couple de
/// composants ne se réduise au même identifiant TypeScript.
// Aucun appelant tant que `generate client` n'existe pas : tombe avec la commande, qui
// assemblera le fichier client à partir de ces interfaces.
#[allow(dead_code)]
pub(crate) fn interfaces(document: &Document) -> Result<Vec<Interface>, Erreur> {
    for (nom, schema) in &document.schemas {
        verifie_references(nom, schema, document)?;
    }

    let mut identifiants_vus: BTreeMap<String, String> = BTreeMap::new();
    let mut rendues = Vec::with_capacity(document.schemas.len());

    for (nom, schema) in &document.schemas {
        let identifiant = identifiant(nom);
        if let Some(premier) = identifiants_vus.get(&identifiant) {
            return Err(Erreur::IdentifiantsHomonymes {
                premier: premier.clone(),
                second: nom.clone(),
                rendu: identifiant,
            });
        }
        identifiants_vus.insert(identifiant.clone(), nom.clone());

        let doc = description_de(schema)
            .filter(|description| !description.trim().is_empty())
            .map(|description| commentaire(&description));

        rendues.push(Interface {
            nom: identifiant,
            doc,
            corps: corps_de(schema),
        });
    }

    Ok(rendues)
}

/// La description d'un composant, quand son schéma en porte une — seul `Object` le
/// fait dans le modèle lu par `document::parse`.
fn description_de(schema: &Schema) -> Option<String> {
    match schema {
        Schema::Object { description, .. } => description.clone(),
        _ => None,
    }
}

/// Parcourt récursivement `schema` à la recherche d'un `$ref` qui ne cible aucun
/// composant déclaré — fait avant tout rendu, pour qu'une référence pendante soit une
/// erreur plutôt qu'un `unknown` muet qui la masquerait.
fn verifie_references(nom: &str, schema: &Schema, document: &Document) -> Result<(), Erreur> {
    match schema {
        Schema::Ref(cible) => {
            if document.schemas.contains_key(cible) {
                Ok(())
            } else {
                Err(Erreur::ReferenceInconnue {
                    nom: nom.to_string(),
                    cible: cible.clone(),
                })
            }
        }
        Schema::Primitive { .. } | Schema::Inconnu => Ok(()),
        Schema::Array { items, .. } => verifie_references(nom, items, document),
        Schema::Object {
            properties,
            additional,
            ..
        } => {
            for (_, propriete) in properties {
                verifie_references(nom, propriete, document)?;
            }
            if let Some(cle_libre) = additional {
                verifie_references(nom, cle_libre, document)?;
            }
            Ok(())
        }
        Schema::Union(variantes) | Schema::Intersection(variantes) => {
            for variante in variantes {
                verifie_references(nom, variante, document)?;
            }
            Ok(())
        }
    }
}

/// Les erreurs que peut rendre `interfaces`.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    #[error(
        "les schémas `{premier}` et `{second}` donnent le même type TypeScript `{rendu}` : renommez l'un des deux"
    )]
    IdentifiantsHomonymes {
        premier: String,
        second: String,
        rendu: String,
    },
    #[error("le schéma `{nom}` référence `{cible}`, que le document ne déclare pas")]
    ReferenceInconnue { nom: String, cible: String },
    // D'autres variantes s'ajoutent avec le rendu des opérations en méthodes du client.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::document;

    fn schemas(json: &str) -> Document {
        document::parse(json).expect("document valide")
    }

    /// Le schéma d'une propriété d'un composant `S`, raccourci de tous les tests de type.
    fn type_du_champ(json_du_champ: &str) -> String {
        let document = schemas(&format!(
            r#"{{"openapi":"3.1.0","components":{{"schemas":{{"S":{{"type":"object",
               "properties":{{"a":{json_du_champ}}}}}}}}}}}"#
        ));
        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        type_de(&properties[0].1)
    }

    #[test]
    fn a_uuid_is_still_a_string() {
        assert_eq!(
            type_du_champ(r#"{"type":"string","format":"uuid"}"#),
            "string"
        );
    }

    #[test]
    fn an_integer_is_a_number() {
        assert_eq!(
            type_du_champ(r#"{"type":"integer","format":"int64"}"#),
            "number"
        );
    }

    #[test]
    fn a_nullable_string_is_a_union_with_null() {
        assert_eq!(
            type_du_champ(r#"{"type":["string","null"]}"#),
            "string | null"
        );
    }

    #[test]
    fn an_array_takes_the_suffix_of_its_items() {
        assert_eq!(
            type_du_champ(r#"{"type":"array","items":{"type":"string"}}"#),
            "string[]"
        );
    }

    #[test]
    fn an_array_of_nullable_items_is_parenthesised() {
        assert_eq!(
            type_du_champ(r#"{"type":"array","items":{"type":["string","null"]}}"#),
            "(string | null)[]"
        );
    }

    #[test]
    fn a_map_becomes_a_record() {
        assert_eq!(
            type_du_champ(
                r#"{"type":"object","additionalProperties":{"type":"array","items":{"type":"string"}}}"#
            ),
            "Record<string, string[]>"
        );
    }

    #[test]
    fn an_object_without_anything_is_a_record_of_unknown() {
        assert_eq!(
            type_du_champ(r#"{"type":"object"}"#),
            "Record<string, unknown>"
        );
    }

    #[test]
    fn a_string_enum_becomes_a_union_of_literals() {
        assert_eq!(
            type_du_champ(r#"{"type":"string","enum":["admin","user"]}"#),
            "\"admin\" | \"user\""
        );
    }

    #[test]
    fn an_inline_object_is_rendered_inline() {
        assert_eq!(
            type_du_champ(
                r#"{"type":"object","required":["a"],"properties":{"a":{"type":"string"},"b":{"type":"boolean"}}}"#
            ),
            "{ a: string; b?: boolean }"
        );
    }

    #[test]
    fn a_schema_without_a_type_is_unknown() {
        assert_eq!(type_du_champ("{}"), "unknown");
    }

    #[test]
    fn a_component_name_loses_what_is_not_an_identifier() {
        assert_eq!(identifiant("Page_PostResponse"), "PagePostResponse");
        assert_eq!(identifiant("ProblemDetails"), "ProblemDetails");
    }

    #[test]
    fn two_components_that_render_the_same_identifier_are_refused() {
        let document = schemas(
            r#"{"openapi":"3.1.0","components":{"schemas":{"Page_A":{"type":"object"},"PageA":{"type":"object"}}}}"#,
        );

        let erreur = interfaces(&document).expect_err("la collision doit être refusée");

        let message = erreur.to_string();
        assert!(message.contains("Page_A"), "{message}");
        assert!(message.contains("PageA"), "{message}");
    }

    #[test]
    fn a_reference_to_an_absent_component_is_refused() {
        let document = schemas(
            r##"{"openapi":"3.1.0","components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"$ref":"#/components/schemas/Fantome"}}}}}}"##,
        );

        let erreur = interfaces(&document).expect_err("la référence pendante doit être refusée");

        assert!(erreur.to_string().contains("Fantome"), "{erreur}");
    }

    #[test]
    fn an_interface_renders_its_required_and_optional_properties() {
        let document = schemas(
            r#"{"openapi":"3.1.0","components":{"schemas":{"CreatePost":{"type":"object",
                 "required":["title"],"properties":{
                   "title":{"type":"string"},"draft":{"type":["boolean","null"]}}}}}}"#,
        );

        let rendues = interfaces(&document).expect("rendu");

        assert_eq!(rendues.len(), 1);
        assert_eq!(rendues[0].nom, "CreatePost");
        // `draft` avant `title` : les propriétés sortent dans l'ordre de
        // `serde_json`, alphabétique faute de la feature `preserve_order`, quel que
        // soit l'ordre du JSON ci-dessus — et le document que sert le projet est déjà
        // trié ainsi, donc le client n'y introduit aucun écart.
        assert_eq!(
            rendues[0].corps,
            "{\n  draft?: boolean | null;\n  title: string;\n}"
        );
    }
}
