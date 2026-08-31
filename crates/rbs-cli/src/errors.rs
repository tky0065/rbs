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
