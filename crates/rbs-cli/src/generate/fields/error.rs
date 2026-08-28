use std::fmt;

use super::FieldType;

/// Toutes les fautes relevées dans une chaîne `--fields`, dans l'ordre des champs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FieldsError {
    pub erreurs: Vec<FieldError>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FieldError {
    /// Rang du champ dans la chaîne, à partir de 1.
    pub rang: usize,
    /// Le nom du champ, ou la portion brute quand le nom n'a pas pu être lu.
    pub libelle: String,
    pub kind: ErrorKind,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    FormeInvalide,
    PasEnSnakeCase { suggestion: Option<String> },
    MotCleRust { suggestions: Vec<String> },
    NomReserve,
    NomCollisionMigration,
    NomEnDouble { rang_precedent: usize },
    TypeInconnu { name: String },
    ModificateurInconnu { name: String },
    ModificateurEnDouble { name: String },
    IndexRedondant,
}

impl ErrorKind {
    fn message(&self, libelle: &str) -> String {
        match self {
            Self::FormeInvalide => "forme attendue : « nom:type[:modificateur…] »".to_string(),
            Self::PasEnSnakeCase { .. } => {
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string()
            }
            Self::MotCleRust { .. } => format!("« {libelle} » est un mot-clé Rust"),
            Self::NomReserve => format!("« {libelle} » ne se déclare pas"),
            Self::NomCollisionMigration => format!(
                "« {libelle} » entrerait en collision avec l'identifiant de la table dans la migration"
            ),
            Self::NomEnDouble { rang_precedent } => {
                format!("« {libelle} » est déjà déclaré au champ {rang_precedent}")
            }
            Self::TypeInconnu { name } => format!("type inconnu « {name} »"),
            Self::ModificateurInconnu { name } => format!("modificateur inconnu « {name} »"),
            Self::ModificateurEnDouble { name } => {
                format!("modificateur « {name} » en double")
            }
            Self::IndexRedondant => {
                "« index » redondant : « unique » pose déjà un index".to_string()
            }
        }
    }

    fn index(&self, libelle: &str) -> Option<String> {
        match self {
            Self::FormeInvalide => Some("exemple : « email:string:unique »".to_string()),
            Self::PasEnSnakeCase { suggestion } => suggestion
                .as_ref()
                .map(|value| format!("essayez « {value} »")),
            Self::MotCleRust { suggestions } => {
                let liste: Vec<String> = suggestions.iter().map(|s| format!("« {s} »")).collect();
                Some(format!("essayez {}", liste.join(" ou ")))
            }
            Self::NomReserve => {
                Some("id, created_at et updated_at sont posés sur toute entité".to_string())
            }
            Self::NomCollisionMigration => Some(format!("essayez « {libelle}_ »")),
            Self::NomEnDouble { .. } => {
                Some("un nom de champ ne peut apparaître qu'une fois".to_string())
            }
            Self::TypeInconnu { .. } => Some(FieldType::NOMS.join(", ")),
            Self::ModificateurInconnu { .. } => Some("unique, optional, index".to_string()),
            Self::ModificateurEnDouble { .. } => None,
            Self::IndexRedondant => Some("retirez « index »".to_string()),
        }
    }
}

impl fmt::Display for FieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut premier = true;
        for error in &self.erreurs {
            if !premier {
                writeln!(f)?;
            }
            premier = false;

            let message = error.kind.message(&error.libelle);
            // Une portion vide n'a pas de libellé à citer : « champ 2 «  » » se lit mal.
            if error.libelle.is_empty() {
                write!(f, "erreur : champ {} — {message}", error.rang)?;
            } else {
                write!(
                    f,
                    "erreur : champ {} « {} » — {message}",
                    error.rang, error.libelle
                )?;
            }

            if let Some(index) = error.kind.index(&error.libelle) {
                write!(f, "\n        → {index}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for FieldsError {}

/// Recasse un nom en snake_case, en repliant une suite de capitales sur un seul mot :
/// `HTTPStatus` donne `http_status` et `EMAIL` donne `email`. Découper à chaque capitale
/// produirait `h_t_t_p_status`, une suggestion que personne n'accepterait.
pub(crate) fn to_snake_case(name: &str) -> String {
    let caracteres: Vec<char> = name.chars().collect();
    let mut output = String::with_capacity(name.len() + 4);

    for (rang, &caractere) in caracteres.iter().enumerate() {
        if caractere == '-' || caractere == ' ' {
            if !output.is_empty() && !output.ends_with('_') {
                output.push('_');
            }
            continue;
        }

        if caractere.is_uppercase() {
            // Une capitale ouvre un mot quand elle suit une minuscule (`firstName`) ou
            // quand elle termine un acronyme accolé au mot suivant (`HTTPStatus`).
            let follows_a_lowercase = rang > 0 && !caracteres[rang - 1].is_uppercase();
            let precedes_a_lowercase = caracteres
                .get(rang + 1)
                .is_some_and(|suivant| suivant.is_lowercase());

            if rang > 0 && (follows_a_lowercase || precedes_a_lowercase) && !output.ends_with('_') {
                output.push('_');
            }
            output.extend(caractere.to_lowercase());
        } else {
            output.push(caractere);
        }
    }

    output
}

/// Le suffixe `_` marche pour tout mot-clé ; les quatre alias devant lui sont ceux
/// qu'un développeur écrirait de lui-même.
pub(crate) fn keyword_suggestions(mot: &str) -> Vec<String> {
    let alias = match mot {
        "type" => Some("kind"),
        "ref" => Some("reference"),
        "match" => Some("matching"),
        "move" => Some("movement"),
        _ => None,
    };

    alias
        .map(str::to_string)
        .into_iter()
        .chain(std::iter::once(format!("{mot}_")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(kind: ErrorKind, libelle: &str) -> String {
        FieldsError {
            erreurs: vec![FieldError {
                rang: 1,
                libelle: libelle.to_string(),
                kind,
            }],
        }
        .to_string()
    }

    #[test]
    fn an_invalid_form_shows_the_expected_form() {
        let text = rendered(ErrorKind::FormeInvalide, "title");
        assert_eq!(
            text,
            "erreur : champ 1 « title » — forme attendue : « nom:type[:modificateur…] »\n\
             \x20       → exemple : « email:string:unique »"
        );
    }

    #[test]
    fn a_badly_cased_name_suggests_its_snake_case_form() {
        let text = rendered(
            ErrorKind::PasEnSnakeCase {
                suggestion: Some("title".to_string()),
            },
            "Title",
        );
        assert!(text.contains("le nom doit être en snake_case"), "{text}");
        assert!(text.contains("→ essayez « title »"), "{text}");
    }

    #[test]
    fn a_name_with_no_possible_recasing_gets_no_hint() {
        let text = rendered(ErrorKind::PasEnSnakeCase { suggestion: None }, "prénom");

        assert!(
            text.contains("minuscules ASCII, chiffres et souligné"),
            "{text}"
        );
        assert!(!text.contains("→"), "{text}");
    }

    #[test]
    fn a_rust_keyword_suggests_its_two_fallbacks() {
        let text = rendered(
            ErrorKind::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()],
            },
            "type",
        );
        assert!(text.contains("« type » est un mot-clé Rust"), "{text}");
        assert!(text.contains("→ essayez « kind » ou « type_ »"), "{text}");
    }

    #[test]
    fn a_reserved_name_recalls_the_three_implicit_columns() {
        let text = rendered(ErrorKind::NomReserve, "id");
        assert!(text.contains("« id » ne se déclare pas"), "{text}");
        assert!(
            text.contains("id, created_at et updated_at sont posés sur toute entité"),
            "{text}"
        );
    }

    #[test]
    fn a_table_name_announces_the_collision_in_the_migration() {
        let text = rendered(ErrorKind::NomCollisionMigration, "table");
        assert!(
            text.contains(
                "« table » entrerait en collision avec l'identifiant de la table dans la migration"
            ),
            "{text}"
        );
        assert!(text.contains("→ essayez « table_ »"), "{text}");
    }

    #[test]
    fn a_duplicated_name_points_back_to_the_previous_field() {
        let text = rendered(ErrorKind::NomEnDouble { rang_precedent: 1 }, "email");
        assert!(
            text.contains("« email » est déjà déclaré au champ 1"),
            "{text}"
        );
        assert!(
            text.contains("→ un nom de champ ne peut apparaître qu'une fois"),
            "{text}"
        );
    }

    #[test]
    fn an_unknown_type_lists_the_allowed_types() {
        let text = rendered(
            ErrorKind::TypeInconnu {
                name: "decimal".to_string(),
            },
            "price",
        );
        assert!(text.contains("type inconnu « decimal »"), "{text}");
        for mot in FieldType::NOMS {
            assert!(text.contains(mot), "« {mot} » absent de : {text}");
        }
    }

    #[test]
    fn an_unknown_modifier_lists_the_three_allowed_ones() {
        let text = rendered(
            ErrorKind::ModificateurInconnu {
                name: "uniq".to_string(),
            },
            "name",
        );
        assert!(text.contains("modificateur inconnu « uniq »"), "{text}");
        assert!(text.contains("unique, optional, index"), "{text}");
    }

    #[test]
    fn a_duplicated_modifier_is_named() {
        let text = rendered(
            ErrorKind::ModificateurEnDouble {
                name: "unique".to_string(),
            },
            "email",
        );
        assert!(text.contains("modificateur « unique » en double"), "{text}");
    }

    #[test]
    fn a_redundant_index_explains_why() {
        let text = rendered(ErrorKind::IndexRedondant, "slug");
        assert!(
            text.contains("« index » redondant : « unique » pose déjà un index"),
            "{text}"
        );
        assert!(text.contains("→ retirez « index »"), "{text}");
    }

    #[test]
    fn several_errors_render_one_block_each_in_order() {
        let text = FieldsError {
            erreurs: vec![
                FieldError {
                    rang: 1,
                    libelle: "Title".to_string(),
                    kind: ErrorKind::PasEnSnakeCase {
                        suggestion: Some("title".to_string()),
                    },
                },
                FieldError {
                    rang: 2,
                    libelle: "type".to_string(),
                    kind: ErrorKind::MotCleRust {
                        suggestions: vec!["kind".to_string(), "type_".to_string()],
                    },
                },
            ],
        }
        .to_string();

        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "{text}");
        assert!(lines[0].starts_with("erreur : champ 1 « Title »"), "{text}");
        assert!(lines[2].starts_with("erreur : champ 2 « type »"), "{text}");
    }

    #[test]
    fn to_snake_case_recases_the_usual_forms() {
        assert_eq!(to_snake_case("Title"), "title");
        assert_eq!(to_snake_case("firstName"), "first_name");
        assert_eq!(to_snake_case("HTTPStatus"), "http_status");
        assert_eq!(to_snake_case("EMAIL"), "email");
        assert_eq!(to_snake_case("mon-champ"), "mon_champ");
        assert_eq!(to_snake_case("déjà_ok"), "déjà_ok");
    }

    #[test]
    fn a_common_keyword_has_an_alias_before_its_fallback() {
        assert_eq!(keyword_suggestions("type"), vec!["kind", "type_"]);
        assert_eq!(keyword_suggestions("ref"), vec!["reference", "ref_"]);
        assert_eq!(keyword_suggestions("loop"), vec!["loop_"]);
    }
}
