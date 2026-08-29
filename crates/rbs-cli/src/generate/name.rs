//! Validation du nom d'une feature, avant qu'un seul fichier ne soit écrit.
//!
//! Une feature occupe `src/<name>/` : son nom entre donc en concurrence avec les modules
//! que `rbs new` a posés. Le diagnostic suit celui des champs — un message, puis un
//! indice qui donne l'issue.

use std::fmt;

use super::fields::error::{keyword_suggestions, to_snake_case};
use super::fields::{RUST_KEYWORDS, is_snake_case};

/// Ce qui rend un nom de feature inutilisable.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NameError {
    libelle: String,
    kind: Kind,
}

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Vide,
    PasEnSnakeCase { suggestion: Option<String> },
    MotCleRust,
    ModuleDuSquelette,
}

/// Modules que `rbs new` pose à la racine de `src/` : une feature qui en porte le nom
/// écraserait le module existant. `health` est un répertoire, les quatre autres des
/// fichiers — la collision est la même.
const MODULES_DU_SQUELETTE: [&str; 5] = ["main", "router", "openapi", "state", "health"];

/// Vérifie qu'une feature peut porter ce nom sans casser le projet.
pub(crate) fn validate(name: &str) -> Result<(), NameError> {
    let error = |kind| {
        Err(NameError {
            libelle: name.to_string(),
            kind,
        })
    };

    if name.is_empty() {
        return error(Kind::Vide);
    }

    if !is_snake_case(name) {
        let recasse = to_snake_case(name);
        let suggestion = (recasse != name && is_snake_case(&recasse)).then_some(recasse);

        return error(Kind::PasEnSnakeCase { suggestion });
    }

    if RUST_KEYWORDS.contains(&name) {
        return error(Kind::MotCleRust);
    }

    if MODULES_DU_SQUELETTE.contains(&name) {
        return error(Kind::ModuleDuSquelette);
    }

    Ok(())
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let libelle = &self.libelle;

        let (message, index) = match &self.kind {
            Kind::Vide => (
                "le nom de la feature est vide".to_string(),
                Some("exemple : « rbs generate crud users »".to_string()),
            ),
            Kind::PasEnSnakeCase { suggestion } => (
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string(),
                suggestion
                    .as_ref()
                    .map(|value| format!("essayez « {value} »")),
            ),
            Kind::MotCleRust => {
                let liste: Vec<String> = keyword_suggestions(libelle)
                    .iter()
                    .map(|suggestion| format!("« {suggestion} »"))
                    .collect();

                (
                    format!("« {libelle} » est un mot-clé Rust"),
                    Some(format!("essayez {}", liste.join(" ou "))),
                )
            }
            Kind::ModuleDuSquelette => (
                format!("« {libelle} » est un module du squelette du projet"),
                Some(format!(
                    "src/ porte déjà ce module — essayez « {libelle}s »"
                )),
            ),
        };

        write!(f, "✗ {message}")?;
        if let Some(index) = index {
            write!(f, "\n  {index}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snake_case_name_is_accepted() {
        for name in ["users", "blog_posts", "categories", "v2_items"] {
            assert!(validate(name).is_ok(), "« {name} » doit être accepté");
        }
    }

    #[test]
    fn a_badly_cased_name_suggests_its_snake_case_form() {
        let error = validate("BlogPosts").expect_err("un nom en PascalCase doit être refusé");

        let rendered = error.to_string();
        assert!(rendered.contains("snake_case"), "{rendered}");
        assert!(
            rendered.contains("blog_posts"),
            "la recasse doit être proposée : {rendered}"
        );
    }

    #[test]
    fn a_rust_keyword_is_rejected() {
        let error = validate("match").expect_err("un mot-clé Rust doit être refusé");

        assert!(error.to_string().contains("mot-clé Rust"), "{error}");
    }

    #[test]
    fn a_skeleton_module_is_rejected_by_naming_it() {
        for name in ["main", "router", "openapi", "state", "health"] {
            let Err(error) = validate(name) else {
                panic!("« {name} » doit être refusé");
            };
            let rendered = error.to_string();

            assert!(
                rendered.contains(name),
                "le message doit nommer le module en cause : {rendered}"
            );
            assert!(
                rendered.contains("squelette"),
                "le message doit dire d'où vient la collision : {rendered}"
            );
        }
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(validate("").is_err());
    }
}
