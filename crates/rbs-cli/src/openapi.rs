//! Le document OpenAPI d'un projet, tel que son binaire `openapi` l'imprime.
//!
//! `generate client`, `routes` et `openapi export` lisent tous trois ce que
//! `ApiDoc::openapi()` rend, sans démarrer de serveur. L'obtenir — et refuser le projet qui
//! ne le peut pas, en disant comment y remédier — ne s'écrit donc qu'ici.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::client::document;
use crate::metadata;

/// Le binaire du projet qui imprime le document.
pub(crate) const BINAIRE: &str = "src/bin/openapi.rs";

/// La bibliothèque sans laquelle ce binaire ne peut pas atteindre `ApiDoc`.
pub(crate) const BIBLIOTHEQUE: &str = "src/lib.rs";

/// Ce qui peut empêcher d'obtenir le document d'un projet.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Obtention {
    /// Le projet n'a pas de bibliothèque, et le binaire ne peut donc pas exister.
    #[error(
        "ce projet n'a pas de {BIBLIOTHEQUE} : `ApiDoc` y vit dans le binaire principal, où \
         un second binaire ne peut pas l'atteindre"
    )]
    SansBibliotheque,

    /// Le projet ne porte pas le binaire qui imprime le document.
    #[error("ce projet n'a pas de {BINAIRE} : rbs n'a aucun document OpenAPI à lire")]
    SansBinaire,

    /// `cargo` n'a pas pu être lancé.
    #[error("cargo n'a pas pu être lancé : {0}")]
    Cargo(#[source] std::io::Error),

    /// Le binaire du projet a échoué.
    #[error("`cargo run --bin openapi` a échoué (code {code}) : le projet ne compile pas")]
    BinaireEnEchec {
        /// Code de sortie du sous-processus.
        code: i32,
    },
}

impl Obtention {
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Un projet créé avant que la template ne porte ce binaire n'a rien à lancer, et cela
    /// se répare en deux gestes plutôt que par une décision : le remède les donne.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Obtention::SansBinaire => Some(format!(
                "créez {BINAIRE} :\n\n\
                 use utoipa::OpenApi;\n\n\
                 fn main() -> Result<(), serde_json::Error> {{\n    \
                 println!(\"{{}}\", <votre_crate>::openapi::ApiDoc::openapi().to_pretty_json()?);\n\n    \
                 Ok(())\n\
                 }}\n\n\
                 puis déclarez-le dans Cargo.toml :\n\n\
                 [[bin]]\nname = \"openapi\"\npath = \"{BINAIRE}\"\n\n\
                 un projet créé par `rbs new` le porte déjà."
            )),
            _ => None,
        }
    }

    /// Code stable de la faute, en snake_case ASCII.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Obtention::SansBibliotheque => "sans_bibliotheque",
            Obtention::SansBinaire => "sans_binaire_openapi",
            Obtention::Cargo(_) => "cargo_introuvable",
            Obtention::BinaireEnEchec { .. } => "projet_ne_compile_pas",
        }
    }
}

/// Lance le binaire `openapi` du projet enraciné en `root` et rend ce qu'il a imprimé.
///
/// Les deux refus précèdent cargo, et dans cet ordre : sans bibliothèque, le binaire ne
/// peut pas exister, et annoncer son absence enverrait le lecteur écrire un fichier qui ne
/// compilerait pas.
///
/// `stderr` est hérité et non capturé : la compilation du projet passe par là, et
/// l'escamoter laisserait la commande muette pendant une minute sur un projet froid. La
/// sortie standard, elle, reste au seul document.
pub(crate) fn imprimer(root: &Path) -> Result<String, Obtention> {
    if !root.join(BIBLIOTHEQUE).exists() {
        return Err(Obtention::SansBibliotheque);
    }

    if !root.join(BINAIRE).exists() {
        return Err(Obtention::SansBinaire);
    }

    let sortie = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "openapi"])
        .current_dir(root)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(Obtention::Cargo)?;

    if !sortie.status.success() {
        return Err(Obtention::BinaireEnEchec {
            code: sortie.status.code().unwrap_or(-1),
        });
    }

    Ok(String::from_utf8_lossy(&sortie.stdout).into_owned())
}

/// Ce qui peut empêcher `rbs routes` ou `rbs openapi export` d'aboutir.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Le document n'a pas pu être obtenu.
    #[error(transparent)]
    Obtention(#[from] Obtention),

    /// Ce que le binaire a imprimé n'est pas un document OpenAPI.
    #[error("{0}")]
    Document(#[from] document::Erreur),

    /// Le fichier de sortie n'a pas pu être écrit.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Error {
    /// Ce qu'il y a à faire, quand il y a quelque chose à faire.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Error::Obtention(obtention) => obtention.remedy(),
            _ => None,
        }
    }
}

/// Le document du projet qui contient `directory`, tel que son binaire l'imprime.
pub(crate) fn document(directory: &Path) -> Result<String, Error> {
    let root = metadata::project_root(directory)?;

    Ok(imprimer(&root)?)
}

/// Écrit le document dans `out`, relatif à `directory`, ou le rend quand `out` manque.
///
/// Le texte est analysé avant d'être écrit : un binaire `openapi` retouché pour imprimer
/// autre chose laisserait sinon un fichier que le premier outil venu refuserait, sous un
/// nom qui promet un contrat.
pub(crate) fn exporter(directory: &Path, out: Option<&Path>) -> Result<Option<String>, Error> {
    let texte = document(directory)?;
    document::parse(&texte)?;

    let Some(out) = out else {
        return Ok(Some(texte));
    };

    let fichier = directory.join(out);
    fs::write(&fichier, &texte).map_err(|source| crate::errors::Acces::new(&fichier, source))?;

    Ok(None)
}

impl crate::errors::Classee for Error {
    fn sortie(&self) -> crate::errors::Sortie {
        use crate::errors::Sortie;

        match self {
            Self::PasUnProjet => Sortie::Usage,
            Self::Acces(_) => Sortie::Environnement,
            Self::Metadata(_) | Self::Obtention(_) | Self::Document(_) => Sortie::Faute,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_project_without_a_library_is_refused_before_its_binary_is_looked_for() {
        let (_tmp, root) = fixtures::project();
        std::fs::remove_file(root.join(BIBLIOTHEQUE)).expect("la bibliothèque doit se supprimer");
        std::fs::remove_file(root.join(BINAIRE)).expect("le binaire doit se supprimer");

        let erreur = imprimer(&root).expect_err("le projet sans bibliothèque doit être refusé");

        assert!(matches!(erreur, Obtention::SansBibliotheque), "{erreur}");
    }

    #[test]
    fn a_project_without_the_binary_is_refused_with_the_block_to_paste() {
        let (_tmp, root) = fixtures::project();
        std::fs::remove_file(root.join(BINAIRE)).expect("le binaire doit se supprimer");

        let erreur = imprimer(&root).expect_err("le projet sans binaire doit être refusé");

        assert!(matches!(erreur, Obtention::SansBinaire), "{erreur}");
        let remede = erreur.remedy().expect("le refus doit porter un remède");
        assert!(remede.contains("[[bin]]"), "{remede}");
    }

    /// Le refus remonte jusqu'à l'erreur des commandes avec son remède : sans cette
    /// délégation, `rbs routes` refuserait sans dire quoi coller.
    #[test]
    fn the_commands_carry_the_remedy_of_a_missing_binary() {
        let (_tmp, root) = fixtures::project();
        std::fs::remove_file(root.join(BINAIRE)).expect("le binaire doit se supprimer");

        let erreur = exporter(&root, None).expect_err("le projet sans binaire doit être refusé");

        assert!(erreur.to_string().contains(BINAIRE), "{erreur}");
        assert!(
            erreur
                .remedy()
                .is_some_and(|remede| remede.contains("[[bin]]")),
            "{erreur}"
        );
    }

    #[test]
    fn outside_an_rbs_project_no_document_is_read() {
        let ailleurs = tempfile::TempDir::new().expect("répertoire temporaire créable");

        let erreur = document(ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(erreur, Error::PasUnProjet), "{erreur}");
    }
}
