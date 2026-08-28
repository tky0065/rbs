pub(crate) mod error;

pub(crate) use error::{ErrorKind, FieldError, FieldsError};
use error::{keyword_suggestions, to_snake_case};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

/// Un des sept types de la grammaire `--fields`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Uuid,
    Datetime,
    Text,
}

impl FieldType {
    pub(crate) const NOMS: [&'static str; 7] =
        ["string", "int", "float", "bool", "uuid", "datetime", "text"];

    pub(crate) fn parse(mot: &str) -> Option<Self> {
        Some(match mot {
            "string" => Self::String,
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "uuid" => Self::Uuid,
            "datetime" => Self::Datetime,
            "text" => Self::Text,
            _ => return None,
        })
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Datetime => "datetime",
            Self::Text => "text",
        }
    }

    pub(crate) fn rust_type(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Int => "i32",
            Self::Float => "f64",
            Self::Bool => "bool",
            Self::Uuid => "Uuid",
            Self::Datetime => "DateTimeWithTimeZone",
        }
    }

    pub(crate) fn migration_method(self) -> &'static str {
        match self {
            Self::String => "string()",
            Self::Int => "integer()",
            Self::Float => "double()",
            Self::Bool => "boolean()",
            Self::Uuid => "uuid()",
            Self::Datetime => "timestamp_with_time_zone()",
            Self::Text => "text()",
        }
    }

    /// SeaORM déduit la colonne du type Rust ; seul `text` doit être forcé, `String`
    /// donnant sinon un `varchar`.
    pub(crate) fn column_type_attribute(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("Text"),
            _ => None,
        }
    }
}

/// Un champ déclaré dans `--fields`, une fois analysé et validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub name: String,
    pub type_: FieldType,
    pub unique: bool,
    pub optionnel: bool,
    pub index: bool,
}

impl Field {
    pub(crate) fn rust_type(&self) -> String {
        if self.optionnel {
            format!("Option<{}>", self.type_.rust_type())
        } else {
            self.type_.rust_type().to_string()
        }
    }

    /// Le champ mérite-t-il une contrainte d'email dans les DTO ?
    ///
    /// La grammaire de `--fields` n'a pas de type `email` et n'en aura pas : sept types
    /// suffisent à décrire une colonne, et un format de chaîne n'est pas un type de
    /// colonne. La contrainte se déduit donc du nom, seule information dont on dispose.
    pub(crate) fn validates_email(&self) -> bool {
        let textual = matches!(self.type_, FieldType::String | FieldType::Text);

        textual && (self.name == "email" || self.name.ends_with("_email"))
    }

    /// Nom en PascalCase, forme qu'exige l'enum `DeriveIden` de la migration.
    pub(crate) fn pascal_name(&self) -> String {
        to_pascal_case(&self.name)
    }
}

/// Sérialisé à la main : minijinja ne voit pas les méthodes Rust, or les templates
/// doivent lire `rust_type` comme elles lisent `name`. Sans cela, chaque générateur
/// reconstruirait sa propre structure de vue.
impl Serialize for Field {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Champ", 11)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("pascal_name", &self.pascal_name())?;
        state.serialize_field("type", self.type_.name())?;
        state.serialize_field("unique", &self.unique)?;
        state.serialize_field("optional", &self.optionnel)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("rust_type", &self.rust_type())?;
        state.serialize_field("bare_rust_type", self.type_.rust_type())?;
        state.serialize_field("migration_method", self.type_.migration_method())?;
        state.serialize_field("column_type_attribute", &self.type_.column_type_attribute())?;
        state.serialize_field("valide_email", &self.validates_email())?;
        state.end()
    }
}

/// Recasse un identifiant snake_case en PascalCase.
pub(crate) fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .map(|mot| {
            let mut caracteres = mot.chars();
            match caracteres.next() {
                Some(premier) => premier.to_uppercase().chain(caracteres).collect(),
                None => String::new(),
            }
        })
        .collect()
}

/// Analyse la chaîne `--fields`. Les fautes de tous les champs sont collectées en une
/// passe : l'utilisateur corrige sa ligne d'un coup plutôt qu'une faute par exécution.
pub(crate) fn parse(input: &str) -> Result<Vec<Field>, FieldsError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut fields = Vec::new();
    let mut erreurs = Vec::new();
    let mut rangs_par_nom: Vec<(String, usize)> = Vec::new();

    for (rang, portion) in input.split(',').enumerate() {
        let rang = rang + 1;

        // L'homonymie se contrôle après la validation du champ lui-même : un champ
        // fautif par ailleurs signale sa propre faute, pas le doublon.
        match parse_field(rang, portion.trim()) {
            Ok(champ) => match rangs_par_nom.iter().find(|(name, _)| *name == champ.name) {
                Some(&(_, rang_precedent)) => erreurs.push(FieldError {
                    rang,
                    libelle: champ.name,
                    kind: ErrorKind::NomEnDouble { rang_precedent },
                }),
                None => {
                    rangs_par_nom.push((champ.name.clone(), rang));
                    fields.push(champ);
                }
            },
            Err(error) => erreurs.push(error),
        }
    }

    if erreurs.is_empty() {
        Ok(fields)
    } else {
        Err(FieldsError { erreurs })
    }
}

fn parse_field(rang: usize, portion: &str) -> Result<Field, FieldError> {
    let error = |libelle: &str, kind| FieldError {
        rang,
        libelle: libelle.to_string(),
        kind,
    };

    let mut parties = portion.split(':').map(str::trim);
    let name = parties.next().unwrap_or_default();
    let type_brut = parties.next().unwrap_or_default();

    if name.is_empty() || type_brut.is_empty() {
        return Err(error(portion, ErrorKind::FormeInvalide));
    }

    if !is_snake_case(name) {
        // Une recasse qui rendrait le nom inchangé, ou toujours invalide — un nom
        // accentué, par exemple — vaut mieux ne pas être proposée du tout.
        let recasse = to_snake_case(name);
        let suggestion = (recasse != name && is_snake_case(&recasse)).then_some(recasse);

        return Err(error(name, ErrorKind::PasEnSnakeCase { suggestion }));
    }

    if MOTS_CLES_RUST.contains(&name) {
        return Err(error(
            name,
            ErrorKind::MotCleRust {
                suggestions: keyword_suggestions(name),
            },
        ));
    }

    if NOMS_POSES_PAR_RBS.contains(&name) {
        return Err(error(name, ErrorKind::NomReserve));
    }

    // La migration écrit `enum Users { Table, Id, … }` : un champ `table` y ajouterait
    // une seconde variante `Table`.
    if name == NOM_DE_LA_TABLE_EN_MIGRATION {
        return Err(error(name, ErrorKind::NomCollisionMigration));
    }

    let Some(type_) = FieldType::parse(type_brut) else {
        return Err(error(
            name,
            ErrorKind::TypeInconnu {
                name: type_brut.to_string(),
            },
        ));
    };

    let mut champ = Field {
        name: name.to_string(),
        type_,
        unique: false,
        optionnel: false,
        index: false,
    };

    for modificateur in parties {
        // Un séparateur surnuméraire — `email:string:` — est une faute de forme, pas un
        // modificateur dont le nom serait vide.
        if modificateur.is_empty() {
            return Err(error(name, ErrorKind::FormeInvalide));
        }

        let drapeau = match modificateur {
            "unique" => &mut champ.unique,
            "optional" => &mut champ.optionnel,
            "index" => &mut champ.index,
            inconnu => {
                return Err(error(
                    name,
                    ErrorKind::ModificateurInconnu {
                        name: inconnu.to_string(),
                    },
                ));
            }
        };

        if *drapeau {
            return Err(error(
                name,
                ErrorKind::ModificateurEnDouble {
                    name: modificateur.to_string(),
                },
            ));
        }

        *drapeau = true;
    }

    if champ.unique && champ.index {
        return Err(error(name, ErrorKind::IndexRedondant));
    }

    Ok(champ)
}

/// Mots-clés stricts et réservés des éditions 2015 à 2024. Un champ ainsi nommé
/// produirait une entité que rustc refuse, quarante secondes plus tard.
pub(crate) const MOTS_CLES_RUST: [&str; 51] = [
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Posées par rbs sur toute entité : les redéclarer donnerait deux fois la colonne.
const NOMS_POSES_PAR_RBS: [&str; 3] = ["id", "created_at", "updated_at"];

/// Variante que `#[derive(DeriveIden)]` réserve au nom de la table dans la migration.
const NOM_DE_LA_TABLE_EN_MIGRATION: &str = "table";

pub(crate) fn is_snake_case(name: &str) -> bool {
    let Some(premier) = name.chars().next() else {
        return false;
    };

    premier.is_ascii_lowercase()
        && !name.ends_with('_')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_of_the_grammar_is_recognised() {
        let cas = [
            ("string", FieldType::String),
            ("int", FieldType::Int),
            ("float", FieldType::Float),
            ("bool", FieldType::Bool),
            ("uuid", FieldType::Uuid),
            ("datetime", FieldType::Datetime),
            ("text", FieldType::Text),
        ];

        for (mot, expected) in cas {
            assert_eq!(FieldType::parse(mot), Some(expected), "type « {mot} »");
        }
    }

    #[test]
    fn a_field_named_email_calls_for_a_validation_constraint() {
        let fields = parse("email:string,contact_email:text,nom:string,email_verifie:bool")
            .expect("champs valides");

        let valident: Vec<_> = fields
            .iter()
            .filter(|champ| champ.validates_email())
            .map(|champ| champ.name.as_str())
            .collect();

        assert_eq!(valident, ["email", "contact_email"]);
    }

    #[test]
    fn a_type_outside_the_grammar_is_not_recognised() {
        assert_eq!(FieldType::parse("decimal"), None);
        assert_eq!(FieldType::parse("String"), None);
        assert_eq!(FieldType::parse(""), None);
    }

    #[test]
    fn name_of_is_the_inverse_of_parse() {
        for mot in FieldType::NOMS {
            let type_ = FieldType::parse(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.name(), mot);
        }
    }

    #[test]
    fn each_type_projects_to_rust() {
        assert_eq!(FieldType::String.rust_type(), "String");
        assert_eq!(FieldType::Text.rust_type(), "String");
        assert_eq!(FieldType::Int.rust_type(), "i32");
        assert_eq!(FieldType::Float.rust_type(), "f64");
        assert_eq!(FieldType::Bool.rust_type(), "bool");
        assert_eq!(FieldType::Uuid.rust_type(), "Uuid");
        assert_eq!(FieldType::Datetime.rust_type(), "DateTimeWithTimeZone");
    }

    #[test]
    fn each_type_projects_to_a_migration_method() {
        assert_eq!(FieldType::String.migration_method(), "string()");
        assert_eq!(FieldType::Text.migration_method(), "text()");
        assert_eq!(FieldType::Int.migration_method(), "integer()");
        assert_eq!(FieldType::Float.migration_method(), "double()");
        assert_eq!(FieldType::Bool.migration_method(), "boolean()");
        assert_eq!(FieldType::Uuid.migration_method(), "uuid()");
        assert_eq!(
            FieldType::Datetime.migration_method(),
            "timestamp_with_time_zone()"
        );
    }

    #[test]
    fn only_text_carries_a_column_type_attribute() {
        assert_eq!(FieldType::Text.column_type_attribute(), Some("Text"));
        for mot in FieldType::NOMS {
            if mot == "text" {
                continue;
            }
            let type_ = FieldType::parse(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.column_type_attribute(), None, "type « {mot} »");
        }
    }

    #[test]
    fn an_optional_field_wraps_its_rust_type() {
        let required = Field {
            name: "title".to_string(),
            type_: FieldType::String,
            unique: false,
            optionnel: false,
            index: false,
        };
        let optionnel = Field {
            optionnel: true,
            ..required.clone()
        };

        assert_eq!(required.rust_type(), "String");
        assert_eq!(optionnel.rust_type(), "Option<String>");
    }

    fn fields(input: &str) -> Vec<Field> {
        parse(input).expect("la chaîne doit être valide")
    }

    #[test]
    fn an_empty_string_declares_no_field() {
        assert_eq!(parse(""), Ok(Vec::new()));
        assert_eq!(parse("   "), Ok(Vec::new()));
    }

    #[test]
    fn a_field_without_a_modifier_has_its_three_flags_down() {
        let fields = fields("title:string");

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "title");
        assert_eq!(fields[0].type_, FieldType::String);
        assert!(!fields[0].unique);
        assert!(!fields[0].optionnel);
        assert!(!fields[0].index);
    }

    #[test]
    fn each_modifier_raises_its_flag() {
        assert!(fields("email:string:unique")[0].unique);
        assert!(fields("bio:text:optional")[0].optionnel);
        assert!(fields("slug:string:index")[0].index);
    }

    #[test]
    fn the_order_of_modifiers_is_free() {
        assert_eq!(
            fields("email:string:unique:optional"),
            fields("email:string:optional:unique")
        );
    }

    #[test]
    fn spaces_around_the_separators_are_ignored() {
        assert_eq!(
            fields(" title : string , email : string : unique "),
            fields("title:string,email:string:unique")
        );
    }

    #[test]
    fn the_fields_keep_their_declaration_order() {
        let fields = fields("un:string,deux:int,trois:bool");
        let names: Vec<&str> = fields.iter().map(|champ| champ.name.as_str()).collect();

        assert_eq!(names, ["un", "deux", "trois"]);
    }

    #[test]
    fn a_field_without_a_type_is_an_invalid_form() {
        let error = parse("title").expect_err("un champ sans type est refusé");

        assert_eq!(error.erreurs.len(), 1);
        assert_eq!(error.erreurs[0].rang, 1);
        assert_eq!(error.erreurs[0].libelle, "title");
        assert_eq!(error.erreurs[0].kind, ErrorKind::FormeInvalide);
    }

    #[test]
    fn a_trailing_comma_is_an_invalid_form() {
        let error = parse("title:string,").expect_err("la virgule finale est refusée");

        assert_eq!(error.erreurs.len(), 1);
        assert_eq!(error.erreurs[0].rang, 2);
        assert_eq!(error.erreurs[0].kind, ErrorKind::FormeInvalide);
        assert_eq!(
            error.to_string(),
            "erreur : champ 2 — forme attendue : « nom:type[:modificateur…] »\n\
             \x20       → exemple : « email:string:unique »"
        );
    }

    #[test]
    fn an_extra_separator_is_an_invalid_form() {
        let error = parse("email:string:").expect_err("le séparateur final est refusé");

        assert_eq!(error.erreurs.len(), 1);
        assert_eq!(error.erreurs[0].libelle, "email");
        assert_eq!(error.erreurs[0].kind, ErrorKind::FormeInvalide);
        assert_eq!(kind("email:string::unique"), ErrorKind::FormeInvalide);
    }

    #[test]
    fn a_type_outside_the_grammar_is_reported_on_its_field() {
        let error = parse("price:decimal").expect_err("decimal n'est pas dans la grammaire");

        assert_eq!(error.erreurs[0].libelle, "price");
        assert_eq!(
            error.erreurs[0].kind,
            ErrorKind::TypeInconnu {
                name: "decimal".to_string()
            }
        );
    }

    fn kind(input: &str) -> ErrorKind {
        let mut error = parse(input).expect_err("la chaîne doit être refusée");
        assert_eq!(error.erreurs.len(), 1, "une seule faute attendue");
        error.erreurs.remove(0).kind
    }

    #[test]
    fn a_non_snake_case_name_is_rejected_with_its_recasing() {
        assert_eq!(
            kind("Title:string"),
            ErrorKind::PasEnSnakeCase {
                suggestion: Some("title".to_string())
            }
        );
        assert_eq!(
            kind("firstName:string"),
            ErrorKind::PasEnSnakeCase {
                suggestion: Some("first_name".to_string())
            }
        );
    }

    #[test]
    fn an_accented_name_is_rejected_without_a_misleading_suggestion() {
        assert_eq!(
            kind("prénom:string"),
            ErrorKind::PasEnSnakeCase { suggestion: None }
        );
    }

    #[test]
    fn a_name_with_a_trailing_underscore_or_a_leading_digit_is_rejected() {
        assert!(matches!(
            kind("titre_:string"),
            ErrorKind::PasEnSnakeCase { .. }
        ));
        assert!(matches!(
            kind("1titre:string"),
            ErrorKind::PasEnSnakeCase { .. }
        ));
    }

    #[test]
    fn a_name_with_an_inner_digit_or_underscore_is_accepted() {
        let fields = fields("adresse_ligne_2:string");
        assert_eq!(fields[0].name, "adresse_ligne_2");
    }

    #[test]
    fn a_rust_keyword_is_rejected_before_compilation() {
        assert_eq!(
            kind("type:string"),
            ErrorKind::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()]
            }
        );
        assert!(matches!(kind("match:string"), ErrorKind::MotCleRust { .. }));
        assert!(matches!(kind("async:bool"), ErrorKind::MotCleRust { .. }));
        assert!(matches!(kind("box:string"), ErrorKind::MotCleRust { .. }));
        assert!(matches!(kind("yield:string"), ErrorKind::MotCleRust { .. }));
    }

    #[test]
    fn the_three_columns_rbs_sets_are_rejected() {
        for name in ["id", "created_at", "updated_at"] {
            assert_eq!(
                kind(&format!("{name}:string")),
                ErrorKind::NomReserve,
                "nom « {name} »"
            );
        }
    }

    #[test]
    fn a_field_named_table_is_rejected_for_the_migration() {
        assert_eq!(kind("table:string"), ErrorKind::NomCollisionMigration);
    }

    #[test]
    fn two_fields_with_the_same_name_are_rejected() {
        let error = parse("email:string,email:int").expect_err("l'homonyme est refusé");

        assert_eq!(error.erreurs.len(), 1);
        assert_eq!(error.erreurs[0].rang, 2);
        assert_eq!(error.erreurs[0].libelle, "email");
        assert_eq!(
            error.erreurs[0].kind,
            ErrorKind::NomEnDouble { rang_precedent: 1 }
        );
    }

    #[test]
    fn only_the_second_duplicate_is_reported() {
        let error =
            parse("email:string,name:string,email:string").expect_err("l'homonyme est refusé");

        assert_eq!(error.erreurs.len(), 1);
        assert_eq!(error.erreurs[0].rang, 3);
        assert_eq!(
            error.erreurs[0].kind,
            ErrorKind::NomEnDouble { rang_precedent: 1 }
        );
    }

    #[test]
    fn a_faulty_field_does_not_hide_the_rank_of_the_first_duplicate() {
        let error = parse("Title:string,email:string,email:string")
            .expect_err("deux fautes sont attendues");

        assert_eq!(error.erreurs.len(), 2);
        assert!(matches!(
            error.erreurs[0].kind,
            ErrorKind::PasEnSnakeCase { .. }
        ));
        assert_eq!(
            error.erreurs[1].kind,
            ErrorKind::NomEnDouble { rang_precedent: 2 }
        );
    }

    #[test]
    fn a_duplicated_modifier_is_rejected() {
        assert_eq!(
            kind("email:string:unique:unique"),
            ErrorKind::ModificateurEnDouble {
                name: "unique".to_string()
            }
        );
    }

    #[test]
    fn unique_with_index_is_rejected_as_redundant() {
        assert_eq!(kind("slug:string:unique:index"), ErrorKind::IndexRedondant);
        assert_eq!(kind("slug:string:index:unique"), ErrorKind::IndexRedondant);
    }

    #[test]
    fn a_unique_on_text_passes_without_comment() {
        assert!(fields("bio:text:unique")[0].unique);
        assert!(fields("active:bool:index")[0].index);
    }

    #[test]
    fn every_fault_in_the_string_surfaces_in_order() {
        let error =
            parse("Title:string,type:text,price:decimal").expect_err("trois fautes attendues");

        assert_eq!(error.erreurs.len(), 3);
        assert_eq!(error.erreurs[0].rang, 1);
        assert!(matches!(
            error.erreurs[0].kind,
            ErrorKind::PasEnSnakeCase { .. }
        ));
        assert_eq!(error.erreurs[1].rang, 2);
        assert!(matches!(
            error.erreurs[1].kind,
            ErrorKind::MotCleRust { .. }
        ));
        assert_eq!(error.erreurs[2].rang, 3);
        assert!(matches!(
            error.erreurs[2].kind,
            ErrorKind::TypeInconnu { .. }
        ));
    }

    #[test]
    fn a_field_carrying_two_faults_surfaces_only_the_first() {
        let error = parse("Type:decimal").expect_err("deux fautes, une seule remontée");

        assert_eq!(error.erreurs.len(), 1);
        assert!(matches!(
            error.erreurs[0].kind,
            ErrorKind::PasEnSnakeCase { .. }
        ));
    }

    #[test]
    fn a_field_serialises_with_its_projections() {
        let fields = fields("bio:text:optional");
        let json = serde_json::to_value(&fields[0]).expect("Champ est sérialisable");

        assert_eq!(json["name"], "bio");
        assert_eq!(json["pascal_name"], "Bio");
        assert_eq!(json["type"], "text");
        assert_eq!(json["unique"], false);
        assert_eq!(json["optional"], true);
        assert_eq!(json["index"], false);
        assert_eq!(json["rust_type"], "Option<String>");
        assert_eq!(json["bare_rust_type"], "String");
        assert_eq!(json["migration_method"], "text()");
        assert_eq!(json["column_type_attribute"], "Text");
    }

    #[test]
    fn the_name_projects_to_pascal_case() {
        assert_eq!(fields("title:string")[0].pascal_name(), "Title");
        assert_eq!(
            fields("adresse_ligne_2:string")[0].pascal_name(),
            "AdresseLigne2"
        );
        assert_eq!(
            fields("date_de_naissance:datetime")[0].pascal_name(),
            "DateDeNaissance"
        );
    }

    #[test]
    fn a_type_without_a_column_attribute_serialises_null() {
        let fields = fields("title:string");
        let json = serde_json::to_value(&fields[0]).expect("Champ est sérialisable");

        assert_eq!(json["rust_type"], "String");
        assert!(json["column_type_attribute"].is_null());
    }
}
