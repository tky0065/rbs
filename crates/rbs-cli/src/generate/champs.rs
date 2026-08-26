/// Un des sept types de la grammaire `--fields`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeChamp {
    String,
    Int,
    Float,
    Bool,
    Uuid,
    Datetime,
    Text,
}

impl TypeChamp {
    pub(crate) const NOMS: [&'static str; 7] =
        ["string", "int", "float", "bool", "uuid", "datetime", "text"];

    pub(crate) fn analyser(mot: &str) -> Option<Self> {
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

    pub(crate) fn nom(self) -> &'static str {
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

    pub(crate) fn type_rust(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Int => "i32",
            Self::Float => "f64",
            Self::Bool => "bool",
            Self::Uuid => "Uuid",
            Self::Datetime => "DateTimeWithTimeZone",
        }
    }

    pub(crate) fn methode_migration(self) -> &'static str {
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
    pub(crate) fn attribut_column_type(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("Text"),
            _ => None,
        }
    }
}

/// Un champ déclaré dans `--fields`, une fois analysé et validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Champ {
    pub nom: String,
    pub type_: TypeChamp,
    pub unique: bool,
    pub optionnel: bool,
    pub index: bool,
}

impl Champ {
    pub(crate) fn type_rust(&self) -> String {
        if self.optionnel {
            format!("Option<{}>", self.type_.type_rust())
        } else {
            self.type_.type_rust().to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_type_de_la_grammaire_est_reconnu() {
        let cas = [
            ("string", TypeChamp::String),
            ("int", TypeChamp::Int),
            ("float", TypeChamp::Float),
            ("bool", TypeChamp::Bool),
            ("uuid", TypeChamp::Uuid),
            ("datetime", TypeChamp::Datetime),
            ("text", TypeChamp::Text),
        ];

        for (mot, attendu) in cas {
            assert_eq!(TypeChamp::analyser(mot), Some(attendu), "type « {mot} »");
        }
    }

    #[test]
    fn un_type_hors_grammaire_n_est_pas_reconnu() {
        assert_eq!(TypeChamp::analyser("decimal"), None);
        assert_eq!(TypeChamp::analyser("String"), None);
        assert_eq!(TypeChamp::analyser(""), None);
    }

    #[test]
    fn nom_est_l_inverse_de_analyser() {
        for mot in TypeChamp::NOMS {
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.nom(), mot);
        }
    }

    #[test]
    fn chaque_type_se_projette_vers_rust() {
        assert_eq!(TypeChamp::String.type_rust(), "String");
        assert_eq!(TypeChamp::Text.type_rust(), "String");
        assert_eq!(TypeChamp::Int.type_rust(), "i32");
        assert_eq!(TypeChamp::Float.type_rust(), "f64");
        assert_eq!(TypeChamp::Bool.type_rust(), "bool");
        assert_eq!(TypeChamp::Uuid.type_rust(), "Uuid");
        assert_eq!(TypeChamp::Datetime.type_rust(), "DateTimeWithTimeZone");
    }

    #[test]
    fn chaque_type_se_projette_vers_une_methode_de_migration() {
        assert_eq!(TypeChamp::String.methode_migration(), "string()");
        assert_eq!(TypeChamp::Text.methode_migration(), "text()");
        assert_eq!(TypeChamp::Int.methode_migration(), "integer()");
        assert_eq!(TypeChamp::Float.methode_migration(), "double()");
        assert_eq!(TypeChamp::Bool.methode_migration(), "boolean()");
        assert_eq!(TypeChamp::Uuid.methode_migration(), "uuid()");
        assert_eq!(
            TypeChamp::Datetime.methode_migration(),
            "timestamp_with_time_zone()"
        );
    }

    #[test]
    fn seul_text_porte_un_attribut_column_type() {
        assert_eq!(TypeChamp::Text.attribut_column_type(), Some("Text"));
        for mot in TypeChamp::NOMS {
            if mot == "text" {
                continue;
            }
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.attribut_column_type(), None, "type « {mot} »");
        }
    }

    #[test]
    fn un_champ_optionnel_enveloppe_son_type_rust() {
        let obligatoire = Champ {
            nom: "titre".to_string(),
            type_: TypeChamp::String,
            unique: false,
            optionnel: false,
            index: false,
        };
        let optionnel = Champ {
            optionnel: true,
            ..obligatoire.clone()
        };

        assert_eq!(obligatoire.type_rust(), "String");
        assert_eq!(optionnel.type_rust(), "Option<String>");
    }
}
