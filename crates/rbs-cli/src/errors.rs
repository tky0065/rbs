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
// Tombe avec le branchement de `--json` sur `add`, `generate` et `upgrade` : sans lui,
// rien n'appelle encore ce trait hors des tests.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) trait Codee {
    /// Code stable, en snake_case ASCII.
    fn code(&self) -> &'static str;
    /// Le remède déjà rendu par la commande, quand la panne se répare en un texte —
    /// jamais un nouveau texte : `--json` réutilise celui que l'affichage humain porte.
    fn remede(&self) -> Option<String>;
    /// Le bloc à coller, quand le remède est une ancre disparue, mal placée, ou une zone
    /// absente d'`AGENTS.md`.
    fn bloc(&self) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add, anchors, client, generate::command, openapi, plan, upgrade};

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
