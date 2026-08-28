use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use super::{Storage, StorageError, normalize};

/// Stockage sur le système de fichiers local, sous une racine unique.
#[derive(Debug, Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    /// Ouvre un stockage sous `root`, créée au premier dépôt.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Le chemin d'un objet, sa clé une fois ramenée sous la racine.
    fn path(&self, key: &str) -> Result<PathBuf, StorageError> {
        Ok(self.root.join(normalize(key)?))
    }
}

fn unavailable(error: std::io::Error) -> StorageError {
    StorageError::Unavailable(Box::new(error))
}

#[async_trait]
impl Storage for FileStorage {
    async fn put(&self, key: &str, content: Vec<u8>) -> Result<(), StorageError> {
        let path = self.path(key)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(unavailable)?;
        }

        fs::write(&path, content).await.map_err(unavailable)
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        fs::read(self.path(key)?).await.map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                StorageError::NotFound(key.to_owned())
            } else {
                unavailable(error)
            }
        })
    }

    // Une clé absente n'est pas une erreur : `DeleteObject` réussit sur une clé que S3 ne
    // trouve pas, et deux backends qui divergeraient là-dessus ne seraient pas
    // substituables l'un à l'autre.
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        match fs::remove_file(self.path(key)?).await {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(unavailable(error)),
            _ => Ok(()),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        fs::try_exists(self.path(key)?).await.map_err(unavailable)
    }
}
