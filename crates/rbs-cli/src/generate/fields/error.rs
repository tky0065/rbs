use std::fmt;

use super::FieldType;

/// Toutes les fautes relevées dans une chaîne `--fields`, dans l'ordre des champs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FieldsError {
    pub errors: Vec<FieldError>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FieldError {
    /// Rang du champ dans la chaîne, à partir de 1.
    pub rank: usize,
    /// Le nom du champ, ou la portion brute quand le nom n'a pas pu être lu.
    pub label: String,
    pub kind: ErrorKind,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    InvalidForm,
    NotSnakeCase { suggestion: Option<String> },
    RustKeyword { suggestions: Vec<String> },
    ReservedName,
    MigrationNameCollision,
    DuplicateName { previous_rank: usize },
    UnknownType { name: String },
    UnknownModifier { name: String },
    DuplicateModifier { name: String },
    RedundantIndex,
    MissingTarget,
    DerivedColumnName { suggestion: String },
    NullifyWithoutOptional,
    ConflictingOnDelete,
    RedundantIndexOnReference,
}

impl ErrorKind {
    fn message(&self, label: &str) -> String {
        match self {
            Self::InvalidForm => "forme attendue : « nom:type[:modificateur…] »".to_string(),
            Self::NotSnakeCase { .. } => {
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string()
            }
            Self::RustKeyword { .. } => format!("« {label} » est un mot-clé Rust"),
            Self::ReservedName => format!("« {label} » ne se déclare pas"),
            Self::MigrationNameCollision => format!(
                "« {label} » entrerait en collision avec l'identifiant de la table dans la migration"
            ),
            Self::DuplicateName { previous_rank } => {
                format!("« {label} » est déjà déclaré au champ {previous_rank}")
            }
            Self::UnknownType { name } => format!("type inconnu « {name} »"),
            Self::UnknownModifier { name } => format!("modificateur inconnu « {name} »"),
            Self::DuplicateModifier { name } => {
                format!("modificateur « {name} » en double")
            }
            Self::RedundantIndex => {
                "« index » redondant : « unique » pose déjà un index".to_string()
            }
            Self::MissingTarget => "« references » attend une entité cible".to_string(),
            Self::DerivedColumnName { .. } => {
                format!("la colonne « {label} » est dérivée du nom de la relation")
            }
            Self::NullifyWithoutOptional => "« nullify » sur une colonne non nullable".to_string(),
            Self::ConflictingOnDelete => "« cascade » et « nullify » se contredisent".to_string(),
            Self::RedundantIndexOnReference => {
                "« index » redondant : une clé étrangère est déjà indexée".to_string()
            }
        }
    }

    fn hint(&self, label: &str) -> Option<String> {
        match self {
            Self::InvalidForm => Some("exemple : « email:string:unique »".to_string()),
            Self::NotSnakeCase { suggestion } => suggestion
                .as_ref()
                .map(|value| format!("essayez « {value} »")),
            Self::RustKeyword { suggestions } => {
                let list: Vec<String> = suggestions.iter().map(|s| format!("« {s} »")).collect();
                Some(format!("essayez {}", list.join(" ou ")))
            }
            Self::ReservedName => {
                Some("id, created_at et updated_at sont posés sur toute entité".to_string())
            }
            Self::MigrationNameCollision => Some(format!("essayez « {label}_ »")),
            Self::DuplicateName { .. } => {
                Some("un nom de champ ne peut apparaître qu'une fois".to_string())
            }
            Self::UnknownType { .. } => Some(FieldType::NAMES.join(", ")),
            Self::UnknownModifier { .. } => {
                Some("unique, optional, index — sur une référence : cascade, nullify".to_string())
            }
            Self::DuplicateModifier { .. } => None,
            Self::RedundantIndex => Some("retirez « index »".to_string()),
            Self::MissingTarget => Some("exemple : « author:references:users »".to_string()),
            Self::DerivedColumnName { suggestion } => Some(format!("essayez « {suggestion} »")),
            Self::NullifyWithoutOptional => {
                Some("ajoutez « optional », ou choisissez « cascade »".to_string())
            }
            Self::ConflictingOnDelete => Some("gardez l'un des deux".to_string()),
            Self::RedundantIndexOnReference => Some("retirez « index »".to_string()),
        }
    }
}

impl fmt::Display for FieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for error in &self.errors {
            if !first {
                writeln!(f)?;
            }
            first = false;

            let message = error.kind.message(&error.label);
            // Une portion vide n'a pas de libellé à citer : « champ 2 «  » » se lit mal.
            if error.label.is_empty() {
                write!(f, "erreur : champ {} — {message}", error.rank)?;
            } else {
                write!(
                    f,
                    "erreur : champ {} « {} » — {message}",
                    error.rank, error.label
                )?;
            }

            if let Some(hint) = error.kind.hint(&error.label) {
                write!(f, "\n        → {hint}")?;
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
    let characters: Vec<char> = name.chars().collect();
    let mut output = String::with_capacity(name.len() + 4);

    for (rank, &character) in characters.iter().enumerate() {
        if character == '-' || character == ' ' {
            if !output.is_empty() && !output.ends_with('_') {
                output.push('_');
            }
            continue;
        }

        if character.is_uppercase() {
            // Une capitale ouvre un mot quand elle suit une minuscule (`firstName`) ou
            // quand elle termine un acronyme accolé au mot suivant (`HTTPStatus`).
            let follows_a_lowercase = rank > 0 && !characters[rank - 1].is_uppercase();
            let precedes_a_lowercase = characters
                .get(rank + 1)
                .is_some_and(|next| next.is_lowercase());

            if rank > 0 && (follows_a_lowercase || precedes_a_lowercase) && !output.ends_with('_') {
                output.push('_');
            }
            output.extend(character.to_lowercase());
        } else {
            output.push(character);
        }
    }

    output
}

/// Le suffixe `_` marche pour tout mot-clé ; les quatre alias devant lui sont ceux
/// qu'un développeur écrirait de lui-même.
pub(crate) fn keyword_suggestions(word: &str) -> Vec<String> {
    let alias = match word {
        "type" => Some("kind"),
        "ref" => Some("reference"),
        "match" => Some("matching"),
        "move" => Some("movement"),
        _ => None,
    };

    alias
        .map(str::to_string)
        .into_iter()
        .chain(std::iter::once(format!("{word}_")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(kind: ErrorKind, label: &str) -> String {
        FieldsError {
            errors: vec![FieldError {
                rank: 1,
                label: label.to_string(),
                kind,
            }],
        }
        .to_string()
    }

    #[test]
    fn an_invalid_form_shows_the_expected_form() {
        let text = rendered(ErrorKind::InvalidForm, "title");
        assert_eq!(
            text,
            "erreur : champ 1 « title » — forme attendue : « nom:type[:modificateur…] »\n\
             \x20       → exemple : « email:string:unique »"
        );
    }

    #[test]
    fn a_badly_cased_name_suggests_its_snake_case_form() {
        let text = rendered(
            ErrorKind::NotSnakeCase {
                suggestion: Some("title".to_string()),
            },
            "Title",
        );
        assert!(text.contains("le nom doit être en snake_case"), "{text}");
        assert!(text.contains("→ essayez « title »"), "{text}");
    }

    #[test]
    fn a_name_with_no_possible_recasing_gets_no_hint() {
        let text = rendered(ErrorKind::NotSnakeCase { suggestion: None }, "prénom");

        assert!(
            text.contains("minuscules ASCII, chiffres et souligné"),
            "{text}"
        );
        assert!(!text.contains("→"), "{text}");
    }

    #[test]
    fn a_rust_keyword_suggests_its_two_fallbacks() {
        let text = rendered(
            ErrorKind::RustKeyword {
                suggestions: vec!["kind".to_string(), "type_".to_string()],
            },
            "type",
        );
        assert!(text.contains("« type » est un mot-clé Rust"), "{text}");
        assert!(text.contains("→ essayez « kind » ou « type_ »"), "{text}");
    }

    #[test]
    fn a_reserved_name_recalls_the_three_implicit_columns() {
        let text = rendered(ErrorKind::ReservedName, "id");
        assert!(text.contains("« id » ne se déclare pas"), "{text}");
        assert!(
            text.contains("id, created_at et updated_at sont posés sur toute entité"),
            "{text}"
        );
    }

    #[test]
    fn a_table_name_announces_the_collision_in_the_migration() {
        let text = rendered(ErrorKind::MigrationNameCollision, "table");
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
        let text = rendered(ErrorKind::DuplicateName { previous_rank: 1 }, "email");
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
            ErrorKind::UnknownType {
                name: "decimal".to_string(),
            },
            "price",
        );
        assert!(text.contains("type inconnu « decimal »"), "{text}");
        for word in FieldType::NAMES {
            assert!(text.contains(word), "« {word} » absent de : {text}");
        }
    }

    #[test]
    fn an_unknown_modifier_lists_the_allowed_ones() {
        let text = rendered(
            ErrorKind::UnknownModifier {
                name: "uniq".to_string(),
            },
            "name",
        );
        assert!(text.contains("modificateur inconnu « uniq »"), "{text}");
        assert!(
            text.contains("unique, optional, index — sur une référence : cascade, nullify"),
            "{text}"
        );
    }

    #[test]
    fn a_duplicated_modifier_is_named() {
        let text = rendered(
            ErrorKind::DuplicateModifier {
                name: "unique".to_string(),
            },
            "email",
        );
        assert!(text.contains("modificateur « unique » en double"), "{text}");
    }

    #[test]
    fn a_redundant_index_explains_why() {
        let text = rendered(ErrorKind::RedundantIndex, "slug");
        assert!(
            text.contains("« index » redondant : « unique » pose déjà un index"),
            "{text}"
        );
        assert!(text.contains("→ retirez « index »"), "{text}");
    }

    #[test]
    fn several_errors_render_one_block_each_in_order() {
        let text = FieldsError {
            errors: vec![
                FieldError {
                    rank: 1,
                    label: "Title".to_string(),
                    kind: ErrorKind::NotSnakeCase {
                        suggestion: Some("title".to_string()),
                    },
                },
                FieldError {
                    rank: 2,
                    label: "type".to_string(),
                    kind: ErrorKind::RustKeyword {
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

    #[test]
    fn a_missing_target_shows_the_expected_form() {
        let text = rendered(ErrorKind::MissingTarget, "author");
        assert!(
            text.contains("« references » attend une entité cible"),
            "{text}"
        );
        assert!(
            text.contains("→ exemple : « author:references:users »"),
            "{text}"
        );
    }

    #[test]
    fn a_derived_column_name_suggests_the_bare_form() {
        let text = rendered(
            ErrorKind::DerivedColumnName {
                suggestion: "author".to_string(),
            },
            "author_id",
        );
        assert!(
            text.contains("la colonne « author_id » est dérivée"),
            "{text}"
        );
        assert!(text.contains("→ essayez « author »"), "{text}");
    }

    #[test]
    fn nullify_without_optional_explains_the_contradiction() {
        let text = rendered(ErrorKind::NullifyWithoutOptional, "author");
        assert!(
            text.contains("« nullify » sur une colonne non nullable"),
            "{text}"
        );
        assert!(text.contains("→ ajoutez « optional »"), "{text}");
    }

    #[test]
    fn two_on_delete_policies_are_named_together() {
        let text = rendered(ErrorKind::ConflictingOnDelete, "author");
        assert!(
            text.contains("« cascade » et « nullify » se contredisent"),
            "{text}"
        );
    }

    #[test]
    fn a_redundant_index_on_a_reference_explains_why() {
        let text = rendered(ErrorKind::RedundantIndexOnReference, "author");
        assert!(
            text.contains("une clé étrangère est déjà indexée"),
            "{text}"
        );
        assert!(text.contains("→ retirez « index »"), "{text}");
    }
}
