//! Traduction du document OpenAPI (`client::document`) en client TypeScript.
//!
//! `type_de` rend l'expression de type d'un schéma partout où il apparaît en position
//! de champ ou de paramètre ; `interfaces` rend en plus la déclaration de plus haut
//! niveau associée à chaque composant nommé du document ; `rendre` assemble le fichier
//! entier, méthodes comprises, à partir du document et de la template du client.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::client::document::{Document, Location, Schema};
use crate::template::Renderer;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/client/ts/client.ts.jinja"
));

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
            .map(|valeur| format!("\"{}\"", echappe_litteral(valeur)))
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

/// Échappe `\` et `"` d'une valeur d'énumération OpenAPI avant de l'écrire entre
/// guillemets doubles TypeScript — rien n'interdit à l'un ou l'autre d'y figurer, et
/// sans cette passe le littéral produit casserait la syntaxe, voire se refermerait au
/// milieu du type.
fn echappe_litteral(valeur: &str) -> String {
    valeur.replace('\\', "\\\\").replace('"', "\\\"")
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
            nullable,
            ..
        } if !properties.is_empty() => {
            // `objet_corps` ignore `nullable`, à la différence de `type_de` : sans ce
            // second `avec_nullable`, un composant nullable à propriétés perdait son
            // `| null` en devenant une déclaration multi-lignes.
            avec_nullable(objet_corps(properties, required), *nullable)
        }
        _ => type_de(schema),
    }
}

/// Un commentaire `/** … */` prêt à écrire au-dessus d'une interface, chaque ligne de
/// la description préfixée de ` * ` au-delà de la première.
fn commentaire(description: &str) -> String {
    // Une description OpenAPI n'a aucune raison d'éviter `*/` : sans cette neutralisation,
    // une occurrence y referme le commentaire et déverse le reste en code TypeScript brut.
    let description = description.replace("*/", "*\\/");
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

/// Les erreurs que peut rendre `interfaces` ou `rendre`.
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
    #[error(
        "les opérations `{premiere}` et `{seconde}` donnent la même méthode `{rendu}` : posez un `operation_id` sur l'un des deux handlers"
    )]
    MethodesHomonymes {
        premiere: String,
        seconde: String,
        rendu: String,
    },
    #[error(
        "l'opération `{operation}` n'a pas d'operationId : posez un `operation_id` sur son handler"
    )]
    SansOperationId { operation: String },
    #[error(
        "l'opération `{operation}` déclare un paramètre `{parametre}` en `{emplacement}`, que le générateur ne sait pas poser"
    )]
    ParametreNonSupporte {
        operation: String,
        parametre: String,
        emplacement: String,
    },
}

/// Une méthode prête à écrire dans le corps de la classe `ApiClient` : nom déjà passé
/// par `nom_de_methode`, doc déjà formatée en `/** … */` indenté, signature et corps déjà
/// rendus — la template n'a plus qu'à les concaténer, sans logique conditionnelle.
#[derive(Debug, Serialize)]
pub(crate) struct Methode {
    pub nom: String,
    pub doc: String,
    pub signature: String,
    pub corps: String,
}

/// `articles_list` → `articlesList` : coupe sur `_`, `-` et l'espace, capitalise
/// chaque tronçon suivant `map`pé par `capitalise`, garde le premier tel quel hormis
/// sa casse initiale — un `operationId` déjà en casse mixte n'est pas retouché plus
/// qu'il ne faut.
fn nom_de_methode(operation_id: &str) -> String {
    let mut troncons = operation_id
        .split(['_', '-', ' '])
        .filter(|tronc| !tronc.is_empty());

    let Some(premier) = troncons.next() else {
        return String::new();
    };
    let mut chars = premier.chars();
    let premier_rendu = match chars.next() {
        Some(c) => c.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    };

    premier_rendu + &troncons.map(capitalise).collect::<String>()
}

/// Le chemin d'une requête : un littéral de gabarit TypeScript quand un paramètre de
/// chemin y figure — `encodeURIComponent` protège une valeur qui contiendrait une barre
/// ou un caractère spécial — une chaîne simple sinon.
fn chemin_gabarit(chemin: &str) -> String {
    if !chemin.contains('{') {
        return format!("\"{chemin}\"");
    }

    let mut rendu = String::from("`");
    let mut reste = chemin;
    while let Some(debut) = reste.find('{') {
        rendu.push_str(&reste[..debut]);
        let apres = &reste[debut + 1..];
        let fin = apres
            .find('}')
            .expect("un chemin OpenAPI ferme toute accolade qu'il ouvre");
        let nom = &apres[..fin];
        rendu.push_str(&format!("${{encodeURIComponent(String({nom}))}}"));
        reste = &apres[fin + 1..];
    }
    rendu.push_str(reste);
    rendu.push('`');
    rendu
}

/// Le corps `return this.request<…>(…);` d'une méthode. Les clés `body` et `query` de
/// l'objet d'options ne figurent que si l'opération en porte l'un ou l'autre ; sans les
/// deux, l'objet d'options est omis plutôt que passé vide.
fn corps_de_methode(
    verbe: &str,
    chemin_rendu: &str,
    return_type: &str,
    a_body: bool,
    a_query: bool,
) -> String {
    let mut options = String::new();
    if a_body || a_query {
        let mut cles = Vec::new();
        if a_body {
            cles.push("body");
        }
        if a_query {
            cles.push("query");
        }
        let lignes: String = cles.iter().map(|cle| format!("      {cle},\n")).collect();
        options = format!(", {{\n{lignes}    }}");
    }

    format!("    return this.request<{return_type}>(\"{verbe}\", {chemin_rendu}{options});")
}

/// Le doc-commentaire `/** … */` d'une méthode, indenté de deux espaces pour s'aligner
/// dans le corps de la classe — le pendant de `commentaire` pour une déclaration membre
/// plutôt que de plus haut niveau, et vide s'il n'y a rien à dire.
fn commentaire_methode(lignes: &[String]) -> String {
    match lignes {
        [] => String::new(),
        [seule] => format!("  /** {seule} */\n"),
        plusieurs => {
            let corps: Vec<String> = plusieurs
                .iter()
                .map(|ligne| format!("   * {ligne}"))
                .collect();
            format!("  /**\n{}\n   */\n", corps.join("\n"))
        }
    }
}

/// Le contexte passé à la template du client — tout y arrive déjà sous une forme que
/// minijinja n'a plus qu'à écrire, sans logique conditionnelle côté template.
#[derive(Serialize)]
struct Contexte<'a> {
    projet: &'a str,
    interfaces: Vec<InterfaceVue>,
    methodes: Vec<Methode>,
    problem_details_manquant: bool,
}

/// `Interface`, mais avec `doc` converti en chaîne prête à écrire : `UndefinedBehavior::
/// Strict` de minijinja lève sur un `Option::None`, et la template place `interface.doc`
/// directement devant `export interface`, sans `{% if %}` pour s'en garder.
#[derive(Serialize)]
struct InterfaceVue {
    nom: String,
    doc: String,
    corps: String,
}

impl From<Interface> for InterfaceVue {
    fn from(interface: Interface) -> Self {
        Self {
            nom: interface.nom,
            doc: interface
                .doc
                .map_or_else(String::new, |doc| format!("{doc}\n")),
            corps: interface.corps,
        }
    }
}

/// Rend le fichier client TypeScript entier : les interfaces des composants, une
/// interface de query par opération qui en a, une méthode par opération, assemblées par
/// la template `client.ts.jinja`.
///
/// Les opérations sont collectées dans l'ordre déterministe du document — `paths` est
/// une `BTreeMap`, `operations` un `Vec` dans l'ordre du texte — puis, pour chacune :
/// l'absence d'`operationId` et un paramètre d'un emplacement non supporté sont refusés
/// avant qu'une méthode n'en soit tirée, pour qu'une opération mal formée ne produise
/// jamais un client partiel.
pub(crate) fn rendre(document: &Document, projet: &str) -> Result<String, Erreur> {
    let interfaces_composants = interfaces(document)?;

    // Repris de `document.schemas` plutôt que des `Interface` déjà rendues, qui ne
    // portent plus le nom d'origine du composant : `interfaces` a déjà refusé toute
    // collision à ce stade, donc cette reconstruction ne peut pas elle-même en trouver.
    let mut identifiants_vus: BTreeMap<String, String> = document
        .schemas
        .keys()
        .map(|nom| (identifiant(nom), nom.clone()))
        .collect();

    let mut interfaces_vue: Vec<InterfaceVue> =
        interfaces_composants.into_iter().map(Into::into).collect();
    let mut methodes_vues: BTreeMap<String, String> = BTreeMap::new();
    let mut methodes = Vec::new();

    for (chemin, path_item) in &document.paths {
        for (verbe, operation) in &path_item.operations {
            let descripteur = format!("{verbe} {chemin}");

            let Some(operation_id) = &operation.operation_id else {
                return Err(Erreur::SansOperationId {
                    operation: descripteur,
                });
            };

            for parametre in &operation.parameters {
                if let Location::Autre(emplacement) = &parametre.location {
                    return Err(Erreur::ParametreNonSupporte {
                        operation: descripteur,
                        parametre: parametre.name.clone(),
                        emplacement: emplacement.clone(),
                    });
                }
            }

            let nom_methode = nom_de_methode(operation_id);
            if let Some(premiere) = methodes_vues.get(&nom_methode) {
                return Err(Erreur::MethodesHomonymes {
                    premiere: premiere.clone(),
                    seconde: operation_id.clone(),
                    rendu: nom_methode,
                });
            }
            methodes_vues.insert(nom_methode.clone(), operation_id.clone());

            let params_chemin = operation
                .parameters
                .iter()
                .filter(|parametre| parametre.location == Location::Path);
            let params_query: Vec<_> = operation
                .parameters
                .iter()
                .filter(|parametre| parametre.location == Location::Query)
                .collect();

            let mut arguments = Vec::new();
            for parametre in params_chemin {
                arguments.push(format!(
                    "{}: {}",
                    parametre.name,
                    type_de(&parametre.schema)
                ));
            }

            let a_body = operation.request_body.is_some();
            if let Some(schema_corps) = &operation.request_body {
                arguments.push(format!("body: {}", type_de(schema_corps)));
            }

            let a_query = !params_query.is_empty();
            if a_query {
                let nom_query = format!("{}Query", capitalise(&nom_methode));
                if let Some(origine) = identifiants_vus.get(&nom_query) {
                    return Err(Erreur::IdentifiantsHomonymes {
                        premier: origine.clone(),
                        second: operation_id.clone(),
                        rendu: nom_query,
                    });
                }
                identifiants_vus.insert(nom_query.clone(), operation_id.clone());

                let properties: Vec<(String, Schema)> = params_query
                    .iter()
                    .map(|parametre| (parametre.name.clone(), parametre.schema.clone()))
                    .collect();
                let required: BTreeSet<String> = params_query
                    .iter()
                    .filter(|parametre| parametre.required)
                    .map(|parametre| parametre.name.clone())
                    .collect();
                let toutes_optionnelles = required.is_empty();

                interfaces_vue.push(InterfaceVue {
                    nom: nom_query.clone(),
                    doc: String::new(),
                    corps: objet_corps(&properties, &required),
                });

                let defaut = if toutes_optionnelles { " = {}" } else { "" };
                arguments.push(format!("query: {nom_query}{defaut}"));
            }

            let mut retours = Vec::new();
            for (statut, schema) in &operation.responses {
                if (200..300).contains(statut)
                    && let Some(schema) = schema
                {
                    retours.push(type_de(schema));
                }
            }
            let retour = if retours.is_empty() {
                "void".to_string()
            } else {
                retours.join(" | ")
            };

            let signature = format!("{nom_methode}({}): Promise<{retour}>", arguments.join(", "));
            let corps = corps_de_methode(verbe, &chemin_gabarit(chemin), &retour, a_body, a_query);

            let mut lignes_doc = Vec::new();
            if let Some(summary) = operation
                .summary
                .as_deref()
                .filter(|s| !s.trim().is_empty())
            {
                lignes_doc.push(summary.to_string());
            }
            if let Some(description) = operation
                .description
                .as_deref()
                .filter(|d| !d.trim().is_empty())
            {
                lignes_doc.push(description.to_string());
            }
            lignes_doc.push(descripteur);
            if operation.secured {
                lignes_doc.push("requiert un jeton".to_string());
            }

            methodes.push(Methode {
                nom: nom_methode,
                doc: commentaire_methode(&lignes_doc),
                signature,
                corps,
            });
        }
    }

    let contexte = Contexte {
        projet,
        interfaces: interfaces_vue,
        methodes,
        problem_details_manquant: !document.schemas.contains_key("ProblemDetails"),
    };

    // Le contexte ci-dessus fournit systématiquement chaque variable que la template
    // lit : un échec de rendu signalerait une faute dans la template elle-même, jamais
    // une entrée utilisateur, d'où l'`expect` plutôt qu'une variante d'erreur de plus.
    let rendu = Renderer::new()
        .render(TEMPLATE, contexte)
        .expect("la template du client ne doit jamais manquer de variable");

    Ok(rendu)
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

    #[test]
    fn a_nullable_object_with_properties_keeps_its_null_union() {
        let document = schemas(
            r#"{"openapi":"3.1.0","components":{"schemas":{"S":{"type":["object","null"],
                 "properties":{"a":{"type":"string"}}}}}}"#,
        );

        let rendues = interfaces(&document).expect("rendu");

        assert_eq!(rendues[0].corps, "{\n  a?: string;\n} | null");
    }

    #[test]
    fn an_enum_value_containing_a_quote_is_escaped() {
        assert_eq!(
            type_du_champ(r#"{"type":"string","enum":["a\"b"]}"#),
            "\"a\\\"b\""
        );
    }

    /// Un document minimal portant une seule opération, pour les tests de méthode.
    fn une_operation(chemin: &str, verbe: &str, corps_json: &str) -> Document {
        document::parse(&format!(
            r#"{{"openapi":"3.1.0","paths":{{"{chemin}":{{"{verbe}":{corps_json}}}}}}}"#
        ))
        .expect("document valide")
    }

    #[test]
    fn an_operation_id_becomes_a_camel_case_method() {
        let rendu = rendre(
            &une_operation(
                "/articles",
                "get",
                r#"{"operationId":"articles_list","responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("articlesList("), "{rendu}");
    }

    #[test]
    fn a_path_parameter_becomes_a_positional_argument() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "get",
                r#"{"operationId":"articles_find","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string","format":"uuid"}}],
                   "responses":{"200":{"description":"ok","content":{"application/json":{
                     "schema":{"type":"string"}}}}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains("articlesFind(id: string): Promise<string>"),
            "{rendu}"
        );
        assert!(
            rendu.contains("${encodeURIComponent(String(id))}"),
            "{rendu}"
        );
    }

    #[test]
    fn the_query_parameters_are_gathered_in_an_exported_interface() {
        let rendu = rendre(
            &une_operation(
                "/articles",
                "get",
                r#"{"operationId":"articles_list","parameters":[
                     {"name":"page","in":"query","required":false,"schema":{"type":"integer"}},
                     {"name":"per_page","in":"query","required":false,"schema":{"type":"integer"}}],
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains("export interface ArticlesListQuery {"),
            "{rendu}"
        );
        assert!(rendu.contains("page?: number;"), "{rendu}");
        assert!(
            rendu.contains("articlesList(query: ArticlesListQuery = {})"),
            "{rendu}"
        );
    }

    #[test]
    fn a_required_query_parameter_makes_the_argument_required() {
        let rendu = rendre(
            &une_operation(
                "/recherche",
                "get",
                r#"{"operationId":"recherche","parameters":[
                     {"name":"q","in":"query","required":true,"schema":{"type":"string"}}],
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains("recherche(query: RechercheQuery)"),
            "{rendu}"
        );
        assert!(!rendu.contains("RechercheQuery = {}"), "{rendu}");
    }

    #[test]
    fn the_arguments_run_path_then_body_then_query() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "patch",
                r#"{"operationId":"articles_update","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string"}},
                     {"name":"dry","in":"query","required":false,"schema":{"type":"boolean"}}],
                   "requestBody":{"required":true,"content":{"application/json":{
                     "schema":{"type":"string"}}}},
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains(
                "articlesUpdate(id: string, body: string, query: ArticlesUpdateQuery = {})"
            ),
            "{rendu}"
        );
    }

    #[test]
    fn a_204_alone_returns_void() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "delete",
                r#"{"operationId":"articles_delete","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string"}}],
                   "responses":{"204":{"description":"supprimé"},
                                "404":{"description":"absent"}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains("articlesDelete(id: string): Promise<void>"),
            "{rendu}"
        );
    }

    #[test]
    fn several_successful_responses_are_unioned() {
        let rendu = rendre(
            &une_operation(
                "/a",
                "post",
                r#"{"operationId":"a_create","responses":{
                     "200":{"description":"ok","content":{"application/json":{"schema":{"type":"string"}}}},
                     "202":{"description":"accepté","content":{"application/json":{"schema":{"type":"boolean"}}}}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("Promise<string | boolean>"), "{rendu}");
    }

    #[test]
    fn a_secured_operation_says_so_in_its_doc_comment() {
        let rendu = rendre(
            &une_operation(
                "/moi",
                "get",
                r#"{"operationId":"moi","security":[{"bearer":[]}],"responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("requiert un jeton"), "{rendu}");
    }

    #[test]
    fn two_operations_of_the_same_name_are_refused() {
        let document = document::parse(
            r#"{"openapi":"3.1.0","paths":{
                 "/a":{"get":{"operationId":"list","responses":{}}},
                 "/b":{"get":{"operationId":"list","responses":{}}}}}"#,
        )
        .expect("document valide");

        let erreur = rendre(&document, "demo").expect_err("la collision doit être refusée");

        let message = erreur.to_string();
        assert!(message.contains("list"), "{message}");
        assert!(message.contains("operation_id"), "{message}");
    }

    #[test]
    fn an_operation_without_an_operation_id_is_refused() {
        let document = une_operation("/a", "get", r#"{"responses":{}}"#);

        let erreur = rendre(&document, "demo").expect_err("l'absence doit être refusée");

        assert!(erreur.to_string().contains("operation_id"), "{erreur}");
    }

    #[test]
    fn a_header_parameter_is_refused_rather_than_ignored() {
        let document = une_operation(
            "/a",
            "get",
            r#"{"operationId":"a","parameters":[
                 {"name":"X-Tenant","in":"header","required":true,"schema":{"type":"string"}}],
               "responses":{}}"#,
        );

        let erreur = rendre(&document, "demo").expect_err("le paramètre doit être refusé");

        let message = erreur.to_string();
        assert!(message.contains("X-Tenant"), "{message}");
        assert!(message.contains("header"), "{message}");
    }

    #[test]
    fn the_rendered_client_carries_the_project_name_and_the_error_class() {
        let rendu = rendre(
            &une_operation("/a", "get", r#"{"operationId":"a","responses":{}}"#),
            "demo-api",
        )
        .expect("rendu");

        assert!(rendu.contains("demo-api"), "{rendu}");
        assert!(
            rendu.contains("export class ApiError extends Error"),
            "{rendu}"
        );
        assert!(rendu.contains("export class ApiClient"), "{rendu}");
    }

    #[test]
    fn a_problem_details_interface_is_emitted_even_when_the_document_has_none() {
        let rendu = rendre(
            &une_operation("/a", "get", r#"{"operationId":"a","responses":{}}"#),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("interface ProblemDetails"), "{rendu}");
    }
}
