//! Validation du nom d'une feature, avant qu'un seul fichier ne soit écrit.
//!
//! Une feature occupe `src/<nom>/` : son nom entre donc en concurrence avec les modules
//! que `rbs new` a posés. Le diagnostic suit celui des champs — un message, puis un
//! indice qui donne l'issue.

use std::fmt;

use super::champs::erreur::{en_snake_case, suggestions_mot_cle};
use super::champs::{MOTS_CLES_RUST, est_en_snake_case};

/// Ce qui rend un nom de feature inutilisable.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurNom {
    libelle: String,
    nature: Nature,
}

#[derive(Debug, PartialEq, Eq)]
enum Nature {
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
pub(crate) fn valider(nom: &str) -> Result<(), ErreurNom> {
    let erreur = |nature| {
        Err(ErreurNom {
            libelle: nom.to_string(),
            nature,
        })
    };

    if nom.is_empty() {
        return erreur(Nature::Vide);
    }

    if !est_en_snake_case(nom) {
        let recasse = en_snake_case(nom);
        let suggestion = (recasse != nom && est_en_snake_case(&recasse)).then_some(recasse);

        return erreur(Nature::PasEnSnakeCase { suggestion });
    }

    if MOTS_CLES_RUST.contains(&nom) {
        return erreur(Nature::MotCleRust);
    }

    if MODULES_DU_SQUELETTE.contains(&nom) {
        return erreur(Nature::ModuleDuSquelette);
    }

    Ok(())
}

impl fmt::Display for ErreurNom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let libelle = &self.libelle;

        let (message, indice) = match &self.nature {
            Nature::Vide => (
                "le nom de la feature est vide".to_string(),
                Some("exemple : « rbs generate crud users »".to_string()),
            ),
            Nature::PasEnSnakeCase { suggestion } => (
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string(),
                suggestion
                    .as_ref()
                    .map(|valeur| format!("essayez « {valeur} »")),
            ),
            Nature::MotCleRust => {
                let liste: Vec<String> = suggestions_mot_cle(libelle)
                    .iter()
                    .map(|suggestion| format!("« {suggestion} »"))
                    .collect();

                (
                    format!("« {libelle} » est un mot-clé Rust"),
                    Some(format!("essayez {}", liste.join(" ou "))),
                )
            }
            Nature::ModuleDuSquelette => (
                format!("« {libelle} » est un module du squelette du projet"),
                Some(format!(
                    "src/ porte déjà ce module — essayez « {libelle}s »"
                )),
            ),
        };

        write!(f, "✗ {message}")?;
        if let Some(indice) = indice {
            write!(f, "\n  {indice}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nom_en_snake_case_est_accepte() {
        for nom in ["users", "blog_posts", "categories", "v2_items"] {
            assert!(valider(nom).is_ok(), "« {nom} » doit être accepté");
        }
    }

    #[test]
    fn un_nom_mal_casse_suggere_sa_forme_snake_case() {
        let erreur = valider("BlogPosts").expect_err("un nom en PascalCase doit être refusé");

        let rendu = erreur.to_string();
        assert!(rendu.contains("snake_case"), "{rendu}");
        assert!(
            rendu.contains("blog_posts"),
            "la recasse doit être proposée : {rendu}"
        );
    }

    #[test]
    fn un_mot_cle_rust_est_refuse() {
        let erreur = valider("match").expect_err("un mot-clé Rust doit être refusé");

        assert!(erreur.to_string().contains("mot-clé Rust"), "{erreur}");
    }

    #[test]
    fn un_module_du_squelette_est_refuse_en_le_nommant() {
        for nom in ["main", "router", "openapi", "state", "health"] {
            let Err(erreur) = valider(nom) else {
                panic!("« {nom} » doit être refusé");
            };
            let rendu = erreur.to_string();

            assert!(
                rendu.contains(nom),
                "le message doit nommer le module en cause : {rendu}"
            );
            assert!(
                rendu.contains("squelette"),
                "le message doit dire d'où vient la collision : {rendu}"
            );
        }
    }

    #[test]
    fn un_nom_vide_est_refuse() {
        assert!(valider("").is_err());
    }
}
