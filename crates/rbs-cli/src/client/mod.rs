//! Génération d'un client TypeScript depuis le document OpenAPI d'un projet.
//!
//! Le CLI ne sait rien du contrat de votre API et ne cherche pas à le deviner : il lance le
//! binaire `openapi` du projet, qui imprime ce que `ApiDoc::openapi()` rend. Le client suit
//! donc le code, et non une lecture approximative des sources.

use std::path::{Path, PathBuf};

use crate::errors::Codee;
use crate::{git, metadata, openapi, plan};

pub(crate) mod document;
pub(crate) mod ts;

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

    /// Le document OpenAPI du projet n'a pas pu être obtenu.
    #[error(transparent)]
    Openapi(#[from] openapi::Obtention),

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
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Error::Openapi(obtention) => obtention.remedy(),
            _ => None,
        }
    }
}

impl Codee for Error {
    fn code(&self) -> &'static str {
        match self {
            Error::PasUnProjet => "pas_un_projet",
            Error::Openapi(obtention) => obtention.code(),
            Error::Document(_) => "document_illisible",
            Error::Rendu(_) => "client_irrendable",
            Error::Acces(_) => "fichier_inaccessible",
            Error::WorkingTreeSale(_) => "arbre_sale",
            Error::Plan(erreur) => erreur.code(),
            Error::Application(erreur) => erreur.code(),
            Error::Metadata(_) => "manifeste_illisible",
        }
    }

    /// `--json` va plus loin que l'affichage humain de `remedy`, qui ne couvre que
    /// `Openapi` : un `Plan` porte son remède dès qu'il en a un, comme son `bloc()`.
    fn remede(&self) -> Option<String> {
        match self {
            Error::Plan(erreur) => erreur.remede(),
            _ => self.remedy(),
        }
    }

    fn bloc(&self) -> Option<String> {
        match self {
            Error::Plan(erreur) => erreur.bloc(),
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

    let json = openapi::imprimer(&root)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::openapi::{BIBLIOTHEQUE, BINAIRE};

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
    fn a_missing_openapi_binary_carries_a_stable_code_and_the_same_remedy() {
        let error = Error::Openapi(crate::openapi::Obtention::SansBinaire);

        assert_eq!(error.code(), "sans_binaire_openapi");
        let remede = error.remede().expect("le refus doit porter un remède");
        assert!(remede.contains("[[bin]]"), "{remede}");
    }

    /// `remedy()` (l'affichage humain) ne couvre que `Openapi`, jamais `Plan` : `Codee::
    /// remede` va plus loin, comme pour `generate`.
    #[test]
    fn a_vanished_anchor_has_a_remede_though_remedy_does_not_cover_plan() {
        let error = Error::Plan(crate::plan::Error::Anchor(crate::anchors::Missing {
            anchor: crate::anchors::ROUTES,
        }));

        assert_eq!(error.remedy(), None);
        assert_eq!(error.code(), "ancre_absente");
        assert!(error.bloc().is_some());
        assert!(error.remede().is_some());
    }

    /// Les trois pannes à bloc d'un plan de client : un bloc sans son remède, ou
    /// l'inverse, laisserait un agent deviner où coller ce qu'on lui montre.
    #[test]
    fn remede_is_some_exactly_when_bloc_is_some_for_a_plan_error() {
        let erreurs = vec![
            crate::plan::Error::Anchor(crate::anchors::Missing {
                anchor: crate::anchors::ROUTES,
            }),
            crate::plan::Error::MalPlacee(Box::new(crate::anchors::Misplaced {
                anchor: crate::anchors::STATE_INIT,
                before: "core: CoreState::new(".to_string(),
                block: "// <rbs:state_init>\n// </rbs:state_init>".to_string(),
            })),
            crate::plan::Error::ZoneAbsente {
                path: "AGENTS.md".to_string(),
                zone: crate::agents::MissingZone {
                    zone: "inventory".to_string(),
                },
            },
        ];

        for erreur in erreurs {
            let error = Error::Plan(erreur);
            assert_eq!(
                error.remede().is_some(),
                error.bloc().is_some(),
                "{error:?}"
            );
            assert!(error.remede().is_some(), "{error:?}");
        }
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
