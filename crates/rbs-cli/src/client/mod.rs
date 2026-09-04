//! Génération d'un client TypeScript depuis le document OpenAPI d'un projet.
//!
//! Le CLI ne sait rien du contrat de votre API et ne cherche pas à le deviner : il lance le
//! binaire `openapi` du projet, qui imprime ce que `ApiDoc::openapi()` rend. Le client suit
//! donc le code, et non une lecture approximative des sources.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{git, metadata, plan};

pub(crate) mod document;
pub(crate) mod ts;

/// Le binaire du projet qui imprime le document.
const BINAIRE: &str = "src/bin/openapi.rs";

/// La bibliothèque sans laquelle ce binaire ne peut pas atteindre `ApiDoc`.
const BIBLIOTHEQUE: &str = "src/lib.rs";

/// Le langage du client demandé.
///
/// Sans rapport avec `lang::Lang`, qui est la langue de l'`AGENTS.md` engendré : ici c'est
/// le langage de programmation de la sortie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Lang {
    /// TypeScript.
    Ts,
}

impl Lang {
    /// Le nom du fichier écrit, et le sous-répertoire par défaut qui le porte.
    fn fichier(self) -> &'static str {
        match self {
            Lang::Ts => "client.ts",
        }
    }

    fn repertoire(self) -> &'static str {
        match self {
            Lang::Ts => "clients/ts",
        }
    }
}

/// Ce qu'il faut savoir pour engendrer un client.
pub(crate) struct Options {
    /// Langage demandé.
    pub lang: Lang,
    /// Répertoire de sortie, relatif à la racine du projet.
    pub out: Option<PathBuf>,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Écrit malgré un working tree Git sale.
    pub force: bool,
}

/// Ce que la commande s'apprête à écrire.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Chemin du client, relatif à la racine du projet.
    pub fichier: String,
    /// Nombre de méthodes engendrées, qui dit ce que le contrat porte.
    pub operations: usize,
}

/// Ce qui peut empêcher d'engendrer un client.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,

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

    /// Le document imprimé n'a pas pu être lu.
    #[error("{0}")]
    Document(#[from] document::Erreur),

    /// Le document ne se traduit pas en TypeScript.
    #[error("{0}")]
    Rendu(#[from] ts::Erreur),

    /// Un fichier du projet n'a pas pu être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le working tree Git porte des modifications non commitées.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    /// Le plan n'a pas pu être construit.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pas pu être appliqué.
    #[error("{0}")]
    Application(#[from] plan::application::Error),

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Error {
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Un projet créé avant que la template ne porte ce binaire n'a rien à lancer, et cela
    /// se répare en deux gestes plutôt que par une décision : le remède les donne.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Error::SansBinaire => Some(format!(
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
}

/// Le chemin du client, relatif à la racine du projet.
///
/// `--out` remplace le répertoire, jamais le nom du fichier : c'est le nom que le client
/// porte dans un import, et le laisser varier ferait d'une régénération dans un autre
/// répertoire un second fichier plutôt qu'une mise à jour.
fn sortie(out: Option<&Path>, lang: Lang) -> PathBuf {
    out.map_or_else(|| PathBuf::from(lang.repertoire()), Path::to_path_buf)
        .join(lang.fichier())
}

/// Prépare l'écriture du client du projet qui contient `options.directory`.
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    let metadata::Cible { root, metadonnees } = metadata::cible::<Error>(&options.directory)?;

    if !options.force {
        git::garde(&root)?;
    }

    // Les deux refus précèdent cargo, et dans cet ordre : sans bibliothèque, le binaire ne
    // peut pas exister, et annoncer son absence enverrait le lecteur écrire un fichier qui
    // ne compilerait pas.
    if !root.join(BIBLIOTHEQUE).exists() {
        return Err(Error::SansBibliotheque);
    }

    if !root.join(BINAIRE).exists() {
        return Err(Error::SansBinaire);
    }

    let json = imprime_le_document(&root)?;
    let document = document::parse(&json)?;

    let projet = metadonnees.package_name(&root.join("Cargo.toml"))?;
    let rendu = ts::rendre(&document, &projet)?;
    let operations = document
        .paths
        .values()
        .map(|chemin| chemin.operations.len())
        .sum();

    let fichier = sortie(options.out.as_deref(), options.lang)
        .to_string_lossy()
        .into_owned();

    let mut builder = plan::Builder::new(root);
    builder.create(&fichier, &rendu)?;

    Ok(Planned {
        plan: builder.finir(),
        fichier,
        operations,
    })
}

/// Lance le binaire du projet et rend ce qu'il a imprimé.
///
/// `stderr` est hérité et non capturé : la compilation du projet passe par là, et
/// l'escamoter laisserait la commande muette pendant une minute sur un projet froid.
fn imprime_le_document(root: &Path) -> Result<String, Error> {
    let sortie = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "openapi"])
        .current_dir(root)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(Error::Cargo)?;

    if !sortie.status.success() {
        return Err(Error::BinaireEnEchec {
            code: sortie.status.code().unwrap_or(-1),
        });
    }

    Ok(String::from_utf8_lossy(&sortie.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_project_without_a_library_is_refused_by_naming_it() {
        let (_tmp, root) = fixtures::project();
        std::fs::remove_file(root.join(BIBLIOTHEQUE)).expect("la bibliothèque doit se supprimer");

        let erreur = plan_for(&Options {
            lang: Lang::Ts,
            out: None,
            directory: root,
            force: true,
        })
        .expect_err("le projet sans bibliothèque doit être refusé");

        let message = erreur.to_string();
        assert!(message.contains(BIBLIOTHEQUE), "{message}");
    }

    #[test]
    fn a_project_without_the_openapi_binary_is_refused_with_the_block_to_paste() {
        let (_tmp, root) = fixtures::project();
        std::fs::remove_file(root.join(BINAIRE)).expect("le binaire doit se supprimer");

        let erreur = plan_for(&Options {
            lang: Lang::Ts,
            out: None,
            directory: root,
            force: true,
        })
        .expect_err("le projet sans binaire doit être refusé");

        let remede = erreur.remedy().expect("le refus doit porter un remède");
        assert!(remede.contains("[[bin]]"), "{remede}");
        assert!(remede.contains(BINAIRE), "{remede}");
        assert!(remede.contains("ApiDoc::openapi()"), "{remede}");
    }

    #[test]
    fn the_default_output_is_the_typescript_directory_of_clients() {
        assert_eq!(
            sortie(None, Lang::Ts),
            PathBuf::from("clients/ts/client.ts")
        );
    }

    #[test]
    fn an_explicit_output_replaces_the_directory_but_not_the_file_name() {
        assert_eq!(
            sortie(Some(Path::new("web/src/api")), Lang::Ts),
            PathBuf::from("web/src/api/client.ts")
        );
    }
}
