//! Les types qui décrivent un filtre dans le document OpenAPI.
//!
//! Aucun n'est construit ni lu : ils ne servent qu'à `#[schema(value_type = ...)]` dans le
//! `filter.rs` que le CLI engendre. `Comparison<T>` ne peut pas s'y décrire lui-même —
//! utoipa ne sait pas décrire un générique dont le paramètre n'implémente pas `ToSchema`,
//! ce que ni `DateTimeUtc` ni `Uuid` ne font : la feature `chrono` d'utoipa agit dans la
//! macro, qui reconnaît ces types par leur nom dans un champ, et n'implémente aucun trait.
//!
//! Chaque schéma est une énumération `untagged`, dont utoipa tire un `oneOf` à deux
//! membres : la valeur nue d'abord — la forme qu'un client écrit dans le cas courant, et
//! celle que Swagger propose alors en exemple — puis l'objet qui nomme les opérateurs. Le
//! `Deserialize` dérivé n'est là que pour rendre `#[serde(untagged)]` légal : c'est cet
//! attribut que lit la macro d'utoipa, et ces types ne lisent jamais rien.
//!
//! Un schéma par type de colonne, plutôt qu'un seul portant une valeur libre : c'est ce
//! qui fait écrire `"published": true` au document, et non `"published": "string"`.

use sea_orm::prelude::Uuid;
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;

/// Engendre le couple qui documente une colonne comparable.
///
/// Les six opérateurs sont les mêmes pour toute colonne ordonnée ; seul change le type de
/// la valeur, et c'est lui que le document doit nommer.
macro_rules! comparaison_documentee {
    ($schema:ident, $operateurs:ident, $valeur:ty, $conditions:literal, $operateurs_doc:literal) => {
        #[doc = $conditions]
        #[doc = ""]
        #[doc = "Une valeur nue, écrite hors de tout objet, vaut la condition `eq`."]
        #[derive(Deserialize, ToSchema)]
        #[serde(untagged)]
        #[non_exhaustive]
        pub enum $schema {
            /// La valeur seule, hors de tout objet : une égalité stricte.
            Bare($valeur),
            /// L'objet qui nomme les conditions demandées.
            Operators($operateurs),
        }

        #[doc = $operateurs_doc]
        #[doc = ""]
        #[doc = "Les conditions écrites ensemble valent un ET."]
        #[derive(Deserialize, ToSchema)]
        #[non_exhaustive]
        pub struct $operateurs {
            /// Égalité stricte.
            pub eq: Option<$valeur>,
            /// Strictement supérieur.
            pub gt: Option<$valeur>,
            /// Supérieur ou égal.
            pub gte: Option<$valeur>,
            /// Strictement inférieur.
            pub lt: Option<$valeur>,
            /// Inférieur ou égal.
            pub lte: Option<$valeur>,
            /// `true` exige une colonne nulle, `false` une colonne renseignée.
            pub is_null: Option<bool>,
        }
    };
}

comparaison_documentee!(
    BoolComparisonSchema,
    BoolComparisonOperators,
    bool,
    "Conditions acceptées sur une colonne booléenne.",
    "Opérateurs acceptés sur une colonne booléenne."
);
comparaison_documentee!(
    IntComparisonSchema,
    IntComparisonOperators,
    i32,
    "Conditions acceptées sur une colonne entière.",
    "Opérateurs acceptés sur une colonne entière."
);
comparaison_documentee!(
    FloatComparisonSchema,
    FloatComparisonOperators,
    f64,
    "Conditions acceptées sur une colonne décimale.",
    "Opérateurs acceptés sur une colonne décimale."
);
comparaison_documentee!(
    UuidComparisonSchema,
    UuidComparisonOperators,
    Uuid,
    "Conditions acceptées sur une colonne d'identifiants.",
    "Opérateurs acceptés sur une colonne d'identifiants."
);
comparaison_documentee!(
    DateTimeComparisonSchema,
    DateTimeComparisonOperators,
    DateTimeSchema,
    "Conditions acceptées sur une colonne datée.",
    "Opérateurs acceptés sur une colonne datée."
);
comparaison_documentee!(
    DateComparisonSchema,
    DateComparisonOperators,
    DateSchema,
    "Conditions acceptées sur une colonne de date sans heure.",
    "Opérateurs acceptés sur une colonne de date sans heure."
);

/// Un instant, écrit en RFC 3339.
///
/// Ce type n'existe que pour porter le format d'une date nue : une variante
/// d'énumération n'accepte pas `value_type`, et sans lui la forme courte se documenterait
/// en `string` sans format — Swagger n'en proposerait alors aucun exemple datable.
#[derive(Deserialize, ToSchema)]
#[schema(value_type = String, format = DateTime)]
pub struct DateTimeSchema(
    /// L'instant, en RFC 3339.
    pub String,
);

/// Un jour sans heure, écrit en ISO 8601 (`AAAA-MM-JJ`).
///
/// Même raison d'être que [`DateTimeSchema`] : une variante d'énumération n'accepte pas
/// `value_type`, et sans ce type nommé la forme courte se documenterait en `string` sans
/// format.
#[derive(Deserialize, ToSchema)]
#[schema(value_type = String, format = Date)]
pub struct DateSchema(
    /// Le jour, en ISO 8601 (`AAAA-MM-JJ`).
    pub String,
);

/// Conditions acceptées sur une colonne textuelle.
///
/// Une chaîne nue, écrite hors de tout objet, vaut la condition `eq`.
#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum TextMatchSchema {
    /// La chaîne seule, hors de tout objet : une égalité stricte.
    Bare(String),
    /// L'objet qui nomme les conditions demandées.
    Operators(TextMatchOperators),
}

/// Opérateurs acceptés sur une colonne textuelle.
///
/// Un texte se cherche par sous-chaîne et ne s'ordonne pas : ses opérateurs ne sont donc
/// pas ceux d'une colonne comparable.
#[derive(Deserialize, ToSchema)]
#[non_exhaustive]
pub struct TextMatchOperators {
    /// Égalité stricte.
    pub eq: Option<String>,
    /// Sous-chaîne, cherchée par `LIKE '%…%'`.
    pub contains: Option<String>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions acceptées sur une colonne à valeurs énumérées.
///
/// Une valeur nue, écrite hors de tout objet, vaut la condition `eq`.
///
/// Les valeurs elles-mêmes ne sont pas nommées ici : elles viennent de `--fields` et
/// changent d'une colonne à l'autre. C'est l'énumération que le modèle engendré déclare
/// qui les porte dans le document, là où le corps de la ressource les cite.
#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OneOfSchema {
    /// La valeur seule, hors de tout objet : une égalité stricte.
    Bare(String),
    /// L'objet qui nomme les conditions demandées.
    Operators(OneOfOperators),
}

/// Opérateurs acceptés sur une colonne à valeurs énumérées.
///
/// Une énumération ne s'ordonne pas : l'appartenance à une liste y remplace les
/// comparaisons d'une colonne ordonnée.
#[derive(Deserialize, ToSchema)]
#[non_exhaustive]
pub struct OneOfOperators {
    /// Égalité stricte.
    pub eq: Option<String>,
    /// Appartenance à l'une des valeurs citées. Une liste vide n'en accepte aucune.
    pub r#in: Option<Vec<String>>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions acceptées sur une colonne comparable, sans que son type soit nommé.
///
/// Les filtres engendrés citent depuis la 1.3.1 le schéma du type de leur colonne, qui
/// nomme aussi la forme courte. Celui-ci reste ce que citent les filtres engendrés avant :
/// le retirer les empêcherait de compiler contre une 1.x ultérieure. Il ne gagne pas le
/// `oneOf` des autres — un membre acceptant toute valeur en rendrait deux vrais à la fois,
/// et un validateur strict refuserait alors la forme longue.
#[derive(ToSchema)]
#[non_exhaustive]
pub struct ComparisonSchema {
    /// Égalité stricte. Une valeur nue, hors de tout objet, vaut cette condition.
    pub eq: Option<Value>,
    /// Strictement supérieur.
    pub gt: Option<Value>,
    /// Supérieur ou égal.
    pub gte: Option<Value>,
    /// Strictement inférieur.
    pub lt: Option<Value>,
    /// Inférieur ou égal.
    pub lte: Option<Value>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use utoipa::{PartialSchema, ToSchema};

    /// Rend le schéma d'un type, tel qu'il entre dans le document.
    fn schema<T: PartialSchema>() -> Value {
        serde_json::to_value(T::schema()).expect("schéma sérialisable")
    }

    /// Le premier membre du `oneOf` est la valeur nue : c'est lui que Swagger propose en
    /// exemple, et c'est la forme que le client écrit dans le cas courant.
    #[test]
    fn a_bare_value_is_the_first_form_offered() {
        for (schema, type_, format) in [
            (schema::<BoolComparisonSchema>(), "boolean", None),
            (schema::<IntComparisonSchema>(), "integer", Some("int32")),
            (schema::<FloatComparisonSchema>(), "number", Some("double")),
            (schema::<UuidComparisonSchema>(), "string", Some("uuid")),
            (schema::<TextMatchSchema>(), "string", None),
        ] {
            let formes = schema["oneOf"]
                .as_array()
                .unwrap_or_else(|| panic!("un oneOf attendu : {schema}"));

            assert_eq!(formes.len(), 2, "{schema}");
            assert_eq!(formes[0]["type"], json!(type_), "{schema}");
            assert_eq!(
                formes[0].get("format").cloned(),
                format.map(Value::from),
                "{schema}"
            );
            assert!(formes[1]["$ref"].is_string(), "{schema}");
        }
    }

    /// Une date nue passe par un type nommé : utoipa ne décrit `DateTimeUtc` que dans un
    /// champ, et une variante d'énumération n'accepte pas `value_type`. Sans lui, la forme
    /// courte se documenterait en `string` sans format, et Swagger n'en proposerait aucun
    /// exemple datable.
    #[test]
    fn a_bare_date_keeps_its_format() {
        let schema = schema::<DateTimeSchema>();

        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "date-time");
    }

    /// Une date sans heure a son propre format : la confondre avec `DateTimeSchema`
    /// documenterait `due` comme un instant, que Swagger daterait d'un exemple horodaté.
    #[test]
    fn a_bare_date_without_time_keeps_its_own_format() {
        let schema = schema::<DateSchema>();

        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "date");
    }

    /// Le second membre nomme les opérateurs, un à un : c'est ce que la forme longue
    /// apporte, et un `$ref` qui ne serait pas exposé pendrait dans le vide.
    #[test]
    fn the_second_form_names_the_operators() {
        for (schema, operateurs) in [
            (
                schema::<BoolComparisonOperators>(),
                vec!["eq", "gt", "gte", "lt", "lte", "is_null"],
            ),
            (
                schema::<DateTimeComparisonOperators>(),
                vec!["eq", "gt", "gte", "lt", "lte", "is_null"],
            ),
            (
                schema::<DateComparisonOperators>(),
                vec!["eq", "gt", "gte", "lt", "lte", "is_null"],
            ),
            (
                schema::<TextMatchOperators>(),
                vec!["eq", "contains", "is_null"],
            ),
        ] {
            let proprietes = schema["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("des propriétés attendues : {schema}"));

            assert_eq!(proprietes.len(), operateurs.len(), "{schema}");
            for operateur in operateurs {
                assert!(
                    proprietes.contains_key(operateur),
                    "« {operateur} » absent : {schema}"
                );
            }
        }
    }

    /// Un `$ref` que le document n'expose pas est un lien mort : chaque schéma doit
    /// déclarer les types que son `oneOf` cite, faute de quoi Swagger n'affiche rien de la
    /// forme longue.
    #[test]
    fn each_schema_exposes_what_its_oneof_cites() {
        for (nom, cites) in [
            (
                BoolComparisonSchema::name(),
                vec!["BoolComparisonOperators"],
            ),
            (
                DateTimeComparisonSchema::name(),
                vec!["DateTimeSchema", "DateTimeComparisonOperators"],
            ),
            (
                DateComparisonSchema::name(),
                vec!["DateSchema", "DateComparisonOperators"],
            ),
            (TextMatchSchema::name(), vec!["TextMatchOperators"]),
        ] {
            let mut exposes = Vec::new();
            match nom.as_ref() {
                "BoolComparisonSchema" => BoolComparisonSchema::schemas(&mut exposes),
                "DateTimeComparisonSchema" => DateTimeComparisonSchema::schemas(&mut exposes),
                "DateComparisonSchema" => DateComparisonSchema::schemas(&mut exposes),
                _ => TextMatchSchema::schemas(&mut exposes),
            }

            let noms: Vec<String> = exposes.into_iter().map(|(nom, _)| nom).collect();
            for cite in cites {
                assert!(
                    noms.iter().any(|nom| nom == cite),
                    "« {cite} » absent : {noms:?}"
                );
            }
        }
    }

    /// Une colonne à valeurs énumérées offre les deux mêmes formes que les autres : la
    /// valeur nue d'abord, puis l'objet qui nomme ses opérateurs.
    #[test]
    fn an_enumerated_column_offers_the_bare_value_first() {
        let schema = schema::<OneOfSchema>();
        let formes = schema["oneOf"]
            .as_array()
            .unwrap_or_else(|| panic!("un oneOf attendu : {schema}"));

        assert_eq!(formes.len(), 2, "{schema}");
        assert_eq!(formes[0]["type"], json!("string"), "{schema}");
        assert!(formes[1]["$ref"].is_string(), "{schema}");
    }

    /// `in` est le seul opérateur que `Comparison` n'a pas : une énumération ne s'ordonne
    /// pas, et l'appartenance à une liste remplace la comparaison.
    #[test]
    fn the_enumerated_operators_are_eq_in_and_is_null() {
        let schema = schema::<OneOfOperators>();
        let proprietes = schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("des propriétés attendues : {schema}"));

        assert_eq!(proprietes.len(), 3, "{schema}");
        for operateur in ["eq", "in", "is_null"] {
            assert!(
                proprietes.contains_key(operateur),
                "« {operateur} » absent : {schema}"
            );
        }
    }

    /// Un `$ref` que le document n'expose pas est un lien mort, ici comme ailleurs.
    #[test]
    fn the_enumerated_schema_exposes_what_its_oneof_cites() {
        let mut exposes = Vec::new();
        OneOfSchema::schemas(&mut exposes);
        let noms: Vec<String> = exposes.into_iter().map(|(nom, _)| nom).collect();

        assert!(
            noms.iter().any(|nom| nom == "OneOfOperators"),
            "« OneOfOperators » absent : {noms:?}"
        );
    }

    /// Aucune condition n'est exigée : un filtre qui ne porte que `gte` est valide, et un
    /// schéma qui les réclamerait toutes ferait refuser le cas courant par un validateur.
    #[test]
    fn no_condition_is_required_of_a_client() {
        for schema in [
            schema::<BoolComparisonOperators>(),
            schema::<TextMatchOperators>(),
        ] {
            assert!(schema.get("required").is_none(), "{schema}");
        }
    }
}
