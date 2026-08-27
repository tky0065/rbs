pub mod fichiers;
pub mod s3;

#[cfg(test)]
mod tests;

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

/// Ce qu'un stockage refuse, ne trouve pas, ou ne peut pas faire.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Aucun objet n'est déposé sous cette clé.
    #[error("aucun objet sous `{0}`")]
    Introuvable(String),

    /// La clé sortirait de la racine configurée.
    #[error("clé refusée : `{0}` sort de la racine du stockage")]
    CleRefusee(String),

    /// Le backend n'a pas répondu.
    #[error("stockage indisponible : {0}")]
    Indisponible(#[source] Box<dyn std::error::Error + Send + Sync>),
}

// Aucun handler du squelette n'appelle encore ces méthodes : la permission tombe avec la
// première route qui dépose ou sert un fichier.
#[async_trait]
#[allow(dead_code)]
pub trait Storage: std::fmt::Debug + Send + Sync {
    /// Dépose `contenu` sous `cle`, en écrasant l'objet qui s'y trouvait.
    async fn deposer(&self, cle: &str, contenu: Vec<u8>) -> Result<(), StorageError>;

    /// Rend le contenu déposé sous `cle`.
    async fn lire(&self, cle: &str) -> Result<Vec<u8>, StorageError>;

    /// Retire l'objet déposé sous `cle`. Une clé absente n'est pas une erreur.
    async fn supprimer(&self, cle: &str) -> Result<(), StorageError>;

    /// Un objet est-il déposé sous `cle` ?
    async fn existe(&self, cle: &str) -> Result<bool, StorageError>;
}

/// Section `[storage]` de la configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Implémentation retenue.
    #[serde(default = "backend_par_defaut")]
    pub backend: String,

    /// Racine du backend `fs`, créée au premier dépôt.
    #[serde(default = "racine_par_defaut")]
    pub racine: PathBuf,

    /// Bucket du backend `s3`.
    #[serde(default)]
    pub bucket: String,

    /// Région du backend `s3`.
    #[serde(default = "region_par_defaut")]
    pub region: String,

    /// URL du service, à renseigner pour toute API compatible S3 autre qu'AWS.
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Identifiant d'accès du backend `s3`.
    #[serde(default)]
    pub access_key_id: String,

    /// Clé secrète du backend `s3`.
    #[serde(default)]
    pub secret_access_key: String,

    /// Bucket dans le chemin plutôt que dans le sous-domaine, ce qu'exige MinIO.
    #[serde(default)]
    pub force_path_style: bool,
}

fn backend_par_defaut() -> String {
    "fs".to_owned()
}

fn racine_par_defaut() -> PathBuf {
    PathBuf::from("./stockage")
}

fn region_par_defaut() -> String {
    "us-east-1".to_owned()
}

/// Le stockage que décrit la section `[storage]`.
pub fn depuis_config() -> anyhow::Result<Arc<dyn Storage>> {
    construire(rbs_core::config::section::<StorageConfig>("storage")?)
}

/// Le stockage décrit par `config`, sans toucher ni à la configuration ni au réseau.
fn construire(config: StorageConfig) -> anyhow::Result<Arc<dyn Storage>> {
    match config.backend.as_str() {
        "fs" => Ok(Arc::new(fichiers::StockageFichiers::nouveau(config.racine))),
        "s3" => Ok(Arc::new(s3::StockageS3::nouveau(&config))),
        inconnu => anyhow::bail!(
            "storage.backend = \"{inconnu}\" : les valeurs admises sont \"fs\" et \"s3\""
        ),
    }
}

/// Résout `cle` en un chemin relatif sûr, ou la refuse.
///
/// Un nom d'objet vient souvent de l'utilisateur. La clé est donc parcourue composant par
/// composant et refusée dès qu'un `..` passe au-dessus de la racine — une recherche de
/// sous-chaîne, elle, laisserait passer `a/../../b`.
pub fn normaliser(cle: &str) -> Result<String, StorageError> {
    let refus = || StorageError::CleRefusee(cle.to_owned());
    let mut segments: Vec<&str> = Vec::new();

    for composant in Path::new(cle).components() {
        match composant {
            Component::CurDir => {}
            Component::ParentDir => {
                segments.pop().ok_or_else(refus)?;
            }
            Component::Normal(segment) => segments.push(segment.to_str().ok_or_else(refus)?),
            Component::RootDir | Component::Prefix(_) => return Err(refus()),
        }
    }

    if segments.is_empty() {
        return Err(refus());
    }

    Ok(segments.join("/"))
}
