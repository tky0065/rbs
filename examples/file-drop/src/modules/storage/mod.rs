pub mod files;
pub mod s3;

#[cfg(test)]
mod tests;

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::state::AppState;

// region: erreurs
/// Ce qu'un stockage refuse, ne trouve pas, ou ne peut pas faire.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Aucun objet n'est déposé sous cette clé.
    #[error("aucun objet sous `{0}`")]
    NotFound(String),

    /// La clé sortirait de la racine configurée.
    #[error("clé refusée : `{0}` sort de la racine du stockage")]
    RejectedKey(String),

    /// Le backend n'a pas répondu.
    #[error("stockage indisponible : {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
// endregion: erreurs

// region: trait
#[async_trait]
pub trait Storage: std::fmt::Debug + Send + Sync {
    /// Dépose `content` sous `key`, en écrasant l'objet qui s'y trouvait.
    async fn put(&self, key: &str, content: Vec<u8>) -> Result<(), StorageError>;

    /// Rend le contenu déposé sous `key`.
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Retire l'objet déposé sous `key`. Une clé absente n'est pas une erreur.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Un objet est-il déposé sous `key` ?
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    /// Le backend répond-il ? La sonde que `GET /health` interroge.
    ///
    /// Rend un booléen et non un `Result` : un contrôle de santé ne distingue pas les
    /// raisons d'une panne, et le détail d'une erreur n'a rien à faire dans une réponse
    /// souvent exposée sans authentification.
    async fn available(&self) -> bool;
}
// endregion: trait

/// Le stockage du projet répond-il ?
///
/// Une fonction plutôt qu'un appel direct à [`Storage::available`] : la sonde s'inscrit
/// dans `src/health/controller.rs`, où le trait n'est pas importé, et une ligne qui
/// qualifierait le trait au complet dépasserait la largeur que rustfmt admet.
pub async fn probe(storage: &Arc<dyn Storage>) -> bool {
    storage.available().await
}

/// Section `[storage]` de la configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Implémentation retenue.
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Racine du backend `fs`, créée au premier dépôt.
    #[serde(default = "default_root")]
    pub root: PathBuf,

    /// Bucket du backend `s3`.
    #[serde(default)]
    pub bucket: String,

    /// Région du backend `s3`.
    #[serde(default = "default_region")]
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

fn default_backend() -> String {
    "fs".to_owned()
}

fn default_root() -> PathBuf {
    PathBuf::from("./stockage")
}

fn default_region() -> String {
    "us-east-1".to_owned()
}

/// Le stockage que décrit la section `[storage]`.
pub fn from_config() -> anyhow::Result<Arc<dyn Storage>> {
    build(rbs_core::config::section::<StorageConfig>("storage")?)
}

// region: build
/// Le stockage décrit par `config`, sans toucher ni à la configuration ni au réseau.
fn build(config: StorageConfig) -> anyhow::Result<Arc<dyn Storage>> {
    match config.backend.as_str() {
        "fs" => Ok(Arc::new(files::FileStorage::new(config.root))),
        "s3" => Ok(Arc::new(s3::S3Storage::new(&config))),
        inconnu => anyhow::bail!(
            "storage.backend = \"{inconnu}\" : les valeurs admises sont \"fs\" et \"s3\""
        ),
    }
}
// endregion: build

// region: normalize
/// Résout `key` en un chemin relatif sûr, ou la refuse.
///
/// Un nom d'objet vient souvent de l'utilisateur. La clé est donc parcourue composant par
/// composant et refusée dès qu'un `..` passe au-dessus de la racine — une recherche de
/// sous-chaîne, elle, laisserait passer `a/../../b`.
pub fn normalize(key: &str) -> Result<String, StorageError> {
    let rejection = || StorageError::RejectedKey(key.to_owned());
    let mut segments: Vec<&str> = Vec::new();

    for composant in Path::new(key).components() {
        match composant {
            Component::CurDir => {}
            Component::ParentDir => {
                segments.pop().ok_or_else(rejection)?;
            }
            Component::Normal(segment) => segments.push(segment.to_str().ok_or_else(rejection)?),
            Component::RootDir | Component::Prefix(_) => return Err(rejection()),
        }
    }

    if segments.is_empty() {
        return Err(rejection());
    }

    Ok(segments.join("/"))
}
// endregion: normalize

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart
// avec elle.
impl AppState {
    /// Le stockage partagé, tel qu'un handler le lit depuis l'état.
    pub fn storage(&self) -> &Arc<dyn Storage> {
        &self.storage
    }
}
