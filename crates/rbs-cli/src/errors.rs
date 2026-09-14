//! Les fautes que plusieurs commandes rendent au même mot près.
//!
//! Rust ne partage pas une variante entre deux énumérations : chaque commande garde donc
//! la sienne, et n'en porte plus le texte ni le constructeur. Ce qui diffère d'une
//! commande à l'autre — le message qui nomme `rbs add` ou `rbs generate` — reste chez
//! elle : deux textes voisins restent deux textes.

use std::io;
use std::path::Path;

/// Un fichier du projet ou d'une template n'a pu être lu ou écrit.
#[derive(Debug, thiserror::Error)]
#[error("{path} est inaccessible : {source}")]
pub(crate) struct Acces {
    /// Chemin fautif.
    pub path: String,
    /// Cause système.
    pub source: io::Error,
}

impl Acces {
    /// La faute, le chemin rendu tel qu'il s'affiche.
    pub(crate) fn new(path: &Path, source: io::Error) -> Self {
        Self {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Le projet porte des modifications non commitées, qu'une commande rendrait
/// indiscernables des siennes.
#[derive(Debug, thiserror::Error)]
#[error("le working tree n'est pas propre : {files} — commitez, ou relancez avec --force")]
pub(crate) struct WorkingTreeSale {
    /// Fichiers suivis modifiés, énumérés.
    pub files: String,
}

/// Le message des commandes qui ne nomment pas la commande fautive.
pub(crate) const PAS_UN_PROJET: &str = "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici";

/// Déclare, pour une énumération portant `PasUnProjet` et `Metadata`, la conversion
/// depuis la faute de remontée : une faute du manifeste se nomme, seule son absence vaut
/// « pas un projet rbs ».
macro_rules! depuis_la_racine {
    ($erreur:ty) => {
        impl From<$crate::metadata::RootError> for $erreur {
            fn from(faute: $crate::metadata::RootError) -> Self {
                match faute {
                    $crate::metadata::RootError::Absent => Self::PasUnProjet,
                    $crate::metadata::RootError::Illisible(faute) => Self::Metadata(faute),
                }
            }
        }
    };
}

pub(crate) use depuis_la_racine;

/// Ce que `--json` rend d'une erreur, en plus de son message.
///
/// Le `code` ne bouge pas d'une reformulation du message à l'autre : un script qui décide
/// sur un texte français casse à la prochaine relecture, décider sur `code()` ne casse
/// qu'au retrait de la variante elle-même — et le `match` exhaustif de chaque
/// implémentation s'en assure au moment de la compiler.
pub(crate) trait Codee {
    /// Code stable, en snake_case ASCII.
    fn code(&self) -> &'static str;
    /// Le remède, quand la panne se répare en un texte. Il peut dire plus que l'affichage
    /// humain : toute erreur qui porte un bloc à coller dit aussi où le coller, même quand
    /// le rendu texte de la commande ne le montrait pas.
    fn remede(&self) -> Option<String>;
    /// Le bloc à coller, quand le remède est une ancre disparue, mal placée, ou une zone
    /// absente d'`AGENTS.md`.
    fn bloc(&self) -> Option<String>;
}

/// Ce que le code de sortie dit d'un échec, pour un script qui ne lit pas le message.
///
/// Trois familles plutôt qu'un code par erreur : un script ne branche que sur ce qu'il
/// peut faire — corriger le projet, corriger l'appel, ou réessayer quand l'environnement
/// le permettra. `rbs doctor` en a besoin pour distinguer une faute trouvée d'un
/// diagnostic qui n'a pas pu tourner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sortie {
    /// Le projet porte une faute que la commande a trouvée, ou qui l'empêche.
    Faute,
    /// La commande est mal appelée, ou pas au bon endroit — le 2 des erreurs d'usage de clap.
    Usage,
    /// L'environnement a manqué : fichier illisible, outil introuvable, service injoignable.
    Environnement,
}

impl Sortie {
    /// Le code que le processus rend.
    pub(crate) fn code(self) -> i32 {
        match self {
            Self::Faute => 1,
            Self::Usage => 2,
            Self::Environnement => 3,
        }
    }
}

/// Une erreur qui sait à quelle famille de sortie elle appartient.
///
/// Chaque implémentation est un `match` exhaustif, sans `_` : une variante ajoutée ne
/// compile pas tant que sa famille n'a pas été décidée.
pub(crate) trait Classee {
    /// La famille de l'échec.
    fn sortie(&self) -> Sortie;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::job;
    use crate::metadata;
    use crate::prompts::PromptError;
    use crate::{
        add, anchors, client, dev, doctor, generate::command, migrate, new, openapi, plan, seed,
        upgrade,
    };

    #[test]
    fn each_family_renders_its_own_exit_code() {
        assert_eq!(Sortie::Faute.code(), 1);
        assert_eq!(Sortie::Usage.code(), 2);
        assert_eq!(Sortie::Environnement.code(), 3);
    }

    fn acces() -> Acces {
        Acces::new(Path::new("src/router.rs"), io::Error::other("refusé"))
    }

    fn ancre() -> plan::Error {
        plan::Error::Anchor(anchors::Missing {
            anchor: anchors::ROUTES,
        })
    }

    #[test]
    fn a_command_run_outside_a_project_is_a_call_to_correct() {
        let sorties = [
            add::Error::PasUnProjet.sortie(),
            command::Error::PasUnProjet.sortie(),
            job::Error::PasUnProjet.sortie(),
            migrate::Error::PasUnProjet.sortie(),
            seed::Error::PasUnProjet.sortie(),
            dev::Error::PasUnProjet.sortie(),
            openapi::Error::PasUnProjet.sortie(),
            upgrade::Error::PasUnProjet.sortie(),
            client::Error::PasUnProjet.sortie(),
            doctor::Error::PasUnProjet.sortie(),
            new::Error::NomInvalide {
                name: "4chan".to_string(),
            }
            .sortie(),
            new::Error::Prompt(PromptError::NomRequis).sortie(),
        ];

        for sortie in sorties {
            assert_eq!(sortie, Sortie::Usage);
        }
    }

    #[test]
    fn a_file_or_a_tool_out_of_reach_is_the_environment() {
        let sorties = [
            add::Error::Acces(acces()).sortie(),
            command::Error::Acces(acces()).sortie(),
            job::Error::Acces(acces()).sortie(),
            seed::Error::Acces(acces()).sortie(),
            openapi::Error::Acces(acces()).sortie(),
            upgrade::Error::Acces(acces()).sortie(),
            client::Error::Acces(acces()).sortie(),
            new::Error::Ecriture {
                path: "demo".to_string(),
                source: io::Error::other("refusé"),
            }
            .sortie(),
            migrate::Error::Cargo(io::Error::other("introuvable")).sortie(),
            dev::Error::Injoignable {
                host: "127.0.0.1".to_string(),
                port: 1,
            }
            .sortie(),
            doctor::Error::Cwd(io::Error::other("supprimé")).sortie(),
        ];

        for sortie in sorties {
            assert_eq!(sortie, Sortie::Environnement);
        }
    }

    #[test]
    fn a_plan_that_the_project_stops_is_a_fault() {
        let sorties = [
            add::Error::Plan(ancre()).sortie(),
            command::Error::Plan(ancre()).sortie(),
            job::Error::Plan(ancre()).sortie(),
            upgrade::Error::Plan(ancre()).sortie(),
            client::Error::Plan(ancre()).sortie(),
            migrate::Error::SansUrl.sortie(),
        ];

        for sortie in sorties {
            assert_eq!(sortie, Sortie::Faute);
        }
    }

    #[test]
    fn a_wrapped_error_keeps_the_family_of_its_cause() {
        assert_eq!(
            add::Error::Env(migrate::Error::SansUrl).sortie(),
            Sortie::Faute
        );
        assert_eq!(
            add::Error::Env(migrate::Error::Cargo(io::Error::other("introuvable"))).sortie(),
            Sortie::Environnement
        );
        assert_eq!(
            dev::Error::Env(migrate::Error::PasUnProjet).sortie(),
            Sortie::Usage
        );
        assert_eq!(
            new::Error::Installation {
                features: "cors".to_string(),
                source: Box::new(add::Error::Acces(acces())),
            }
            .sortie(),
            Sortie::Environnement
        );
    }

    /// Un fichier que le plan n'a pas pu lire reste une panne d'environnement, même
    /// enveloppé dans l'erreur de la commande.
    #[test]
    fn a_plan_that_cannot_read_a_file_is_the_environment() {
        assert_eq!(
            add::Error::Plan(plan::Error::Acces(acces())).sortie(),
            Sortie::Environnement
        );
    }

    #[test]
    fn a_manifest_that_is_not_an_rbs_project_is_a_call_to_correct() {
        assert_eq!(
            add::Error::Metadata(metadata::Error::PasUnProjet {
                path: "Cargo.toml".to_string(),
            })
            .sortie(),
            Sortie::Usage
        );
    }

    #[test]
    fn cargo_that_cannot_be_launched_is_the_environment() {
        assert_eq!(
            openapi::Error::Obtention(openapi::Obtention::Cargo(io::Error::other("introuvable")))
                .sortie(),
            Sortie::Environnement
        );
    }

    #[test]
    fn a_project_that_does_not_compile_is_a_fault() {
        assert_eq!(
            client::Error::Openapi(openapi::Obtention::BinaireEnEchec { code: 101 }).sortie(),
            Sortie::Faute
        );
    }

    /// Le remède est de modifier l'échéance du projet à la main, pas l'appel.
    #[test]
    fn a_schedule_already_set_is_a_fault_of_the_project() {
        assert_eq!(
            job::Error::EcheanceExistante {
                nom: "purge".to_string(),
                existante: "0 4 * * *".to_string(),
                demandee: "0 5 * * *".to_string(),
            }
            .sortie(),
            Sortie::Faute
        );
    }

    /// `--force` lève un conflit : c'est l'appel qui change, pas l'environnement.
    #[test]
    fn a_conflict_is_lifted_by_the_call_and_a_failed_write_by_the_environment() {
        let conflit = || plan::application::Error::Conflit {
            chemins: "Dockerfile".to_string(),
        };

        assert_eq!(add::Error::Application(conflit()).sortie(), Sortie::Usage);
        assert_eq!(
            upgrade::Error::Application(plan::application::Error::Ecriture {
                path: "Dockerfile".to_string(),
                source: io::Error::other("disque plein"),
            })
            .sortie(),
            Sortie::Environnement
        );
    }

    /// Un test rouge garde le code que `cargo test` a rendu : une CI le distingue d'une
    /// commande qui n'a pas pu démarrer.
    #[test]
    fn a_red_test_keeps_the_code_cargo_returned() {
        assert_eq!(dev::Error::Tests { code: 101 }.exit_code(), 101);
        assert_eq!(dev::Error::PasUnProjet.exit_code(), 2);
    }

    /// Un code n'est jamais lu par un humain : un accent, une majuscule ou un tiret y
    /// signalerait un message recopié plutôt qu'un code choisi pour durer.
    fn est_snake_case_ascii(code: &str) -> bool {
        !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_')
    }

    #[test]
    fn a_sample_of_errors_from_every_command_renders_a_snake_case_ascii_code() {
        let codes: Vec<&str> = vec![
            add::Error::PasUnProjet.code(),
            add::Error::UrlIndecomposable {
                url: "postgres://x".to_string(),
            }
            .code(),
            add::Error::WorkingTreeSale(WorkingTreeSale {
                files: "a".to_string(),
            })
            .code(),
            command::Error::PasUnProjet.code(),
            command::Error::DejaPresente {
                path: "src/articles".to_string(),
                feature: "articles".to_string(),
            }
            .code(),
            command::Error::UploadSansStorage.code(),
            command::Error::EnfantSansCle {
                child: "commentaires".to_string(),
                table: "articles".to_string(),
            }
            .code(),
            command::Error::Plan(plan::Error::Anchor(anchors::Missing {
                anchor: anchors::ROUTES,
            }))
            .code(),
            client::Error::PasUnProjet.code(),
            client::Error::Openapi(openapi::Obtention::SansBibliotheque).code(),
            client::Error::Openapi(openapi::Obtention::BinaireEnEchec { code: 1 }).code(),
            upgrade::Error::PasUnProjet.code(),
            upgrade::Error::CliAnterieur {
                projet: "1.5.0".to_string(),
                cli: "1.4.0".to_string(),
            }
            .code(),
            plan::application::Error::Conflit {
                chemins: "src.rs".to_string(),
            }
            .code(),
        ];

        for code in codes {
            assert!(est_snake_case_ascii(code), "{code:?}");
        }
    }
}
