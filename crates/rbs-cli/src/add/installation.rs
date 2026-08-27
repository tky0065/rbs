//! Traduit ce qu'un manifeste de fragment déclare en actions de plan.
//!
//! C'est ici, et nulle part ailleurs, qu'`add` cesse d'être générique : le manifeste est
//! lu, ses sections sont interprétées, et le CLI ne connaît toujours aucune feature par
//! son nom.

use std::path::Path;

use minijinja::Value;

use crate::manifeste::Manifeste;
use crate::plan;
use crate::template::Renderer;
use crate::templates;

/// Le fragment tel que l'installation le voit.
pub(crate) struct Fragment<'a> {
    /// Nom de la feature, pour les messages d'erreur.
    pub nom: &'a str,
    /// Ce que son manifeste déclare.
    pub manifeste: &'a Manifeste,
    /// Ses templates, telles que la source les a lues.
    pub templates: &'a [templates::Fichier],
    /// Contexte de rendu, déduit du projet visé.
    pub contexte: Value,
}

/// Ce qui peut empêcher d'interpréter un manifeste.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Le manifeste désigne une template que le fragment ne porte pas.
    #[error("{feature}/feature.toml déclare `{template}`, absente du fragment")]
    TemplateAbsente {
        /// Feature en cours d'installation.
        feature: String,
        /// Template introuvable, telle que le manifeste la nomme.
        template: String,
    },

    /// Une template ne s'est pas rendue.
    #[error("{fichier} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        fichier: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// L'action n'a pas pu être planifiée.
    #[error("{0}")]
    Plan(#[from] plan::Erreur),
}

/// Ajoute au plan ce que le manifeste déclare, et rend les chemins déposés.
pub(crate) fn actions(
    fragment: &Fragment,
    constructeur: &mut plan::Constructeur,
) -> Result<Vec<String>, Erreur> {
    let renderer = Renderer::new();
    let mut deposes = Vec::new();

    for (destination, source) in a_deposer(fragment)? {
        let contenu = renderer
            .rendre(source, fragment.contexte.clone())
            .map_err(|source| Erreur::Rendu {
                fichier: destination.clone(),
                source,
            })?;

        constructeur.creer(&destination, &contenu)?;
        deposes.push(destination);
    }

    Ok(deposes)
}

/// Les templates à déposer, avec leur chemin dans le projet.
///
/// Sans `[[fichiers]]`, le fragment est copié tel quel : un fragment qui n'apporte pas de
/// code Rust n'a rien à déclarer pour que ses fichiers arrivent où leur arborescence les
/// place déjà.
fn a_deposer<'a>(fragment: &'a Fragment) -> Result<Vec<(String, &'a str)>, Erreur> {
    if fragment.manifeste.fichiers.is_empty() {
        return Ok(fragment
            .templates
            .iter()
            .map(|template| {
                (
                    template.destination.to_string_lossy().into_owned(),
                    template.source.as_str(),
                )
            })
            .collect());
    }

    fragment
        .manifeste
        .fichiers
        .iter()
        .map(|declare| {
            let source = template(fragment, &declare.source)?;
            Ok((declare.cible.clone(), source))
        })
        .collect()
}

/// La source de la template que le manifeste désigne par `nom`.
fn template<'a>(fragment: &'a Fragment, nom: &str) -> Result<&'a str, Erreur> {
    fragment
        .templates
        .iter()
        .find(|template| template.origine == Path::new(nom))
        .map(|template| template.source.as_str())
        .ok_or_else(|| Erreur::TemplateAbsente {
            feature: fragment.nom.to_string(),
            template: nom.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use minijinja::context;
    use tempfile::TempDir;

    use super::*;
    use crate::manifeste;

    /// Une template du fragment, telle que la source la restituerait.
    fn template_de(origine: &str, source: &str) -> templates::Fichier {
        let origine = PathBuf::from(origine);
        let destination = if origine
            .extension()
            .is_some_and(|suffixe| suffixe == "jinja")
        {
            origine.with_extension("")
        } else {
            origine.clone()
        };

        templates::Fichier {
            destination,
            origine,
            source: source.to_string(),
        }
    }

    /// Planifie l'installation d'un fragment décrit par son manifeste et ses templates.
    fn planifier(
        racine: &Path,
        manifeste: &str,
        templates: &[templates::Fichier],
    ) -> Result<(Vec<String>, plan::Plan), Erreur> {
        let manifeste = manifeste::lire(manifeste, "essai/feature.toml")
            .expect("le manifeste du test doit être valide");
        let mut constructeur = plan::Constructeur::nouveau(racine.to_path_buf());

        let deposes = actions(
            &Fragment {
                nom: "essai",
                manifeste: &manifeste,
                templates,
                contexte: context! { nom_projet => "demo-api", nom_crate => "demo_api" },
            },
            &mut constructeur,
        )?;

        Ok((deposes, constructeur.finir()))
    }

    #[test]
    fn sans_section_fichiers_le_fragment_est_copie_tel_quel() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        let templates = [
            template_de("Dockerfile.jinja", "FROM rust\n"),
            template_de(".dockerignore", "target\n"),
        ];

        let (deposes, plan) = planifier(
            projet.path(),
            "[feature]\ndescription = \"docker\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["Dockerfile", ".dockerignore"]);
        assert_eq!(plan.fichiers().len(), 2);
        assert_eq!(plan.fichiers()[0].apres, "FROM rust\n");
    }

    #[test]
    fn une_template_declaree_est_deposee_a_la_cible_qui_l_accompagne() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        let templates = [template_de(
            "model.rs.jinja",
            "// {@ nom_crate @}\npub struct User;\n",
        )];

        let (deposes, plan) = planifier(
            projet.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[fichiers]]\nsource = \"model.rs.jinja\"\ncible = \"src/auth/model.rs\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["src/auth/model.rs"]);
        assert_eq!(plan.fichiers()[0].chemin, "src/auth/model.rs");
        assert_eq!(plan.fichiers()[0].apres, "// demo_api\npub struct User;\n");
    }

    /// Une template déclarée mais absente est une faute du manifeste, pas un silence.
    #[test]
    fn une_template_declaree_et_absente_est_signalee_par_son_nom() {
        let projet = TempDir::new().expect("répertoire temporaire créable");

        let erreur = planifier(
            projet.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[fichiers]]\nsource = \"absente.rs.jinja\"\ncible = \"src/auth/model.rs\"\n",
            &[],
        )
        .expect_err("la template n'existe pas");

        assert!(matches!(erreur, Erreur::TemplateAbsente { .. }), "{erreur}");
        assert!(erreur.to_string().contains("absente.rs.jinja"), "{erreur}");
    }
}
