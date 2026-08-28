use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use super::{Storage, StorageError, normaliser};

/// Stockage sur le système de fichiers local, sous une racine unique.
#[derive(Debug, Clone)]
pub struct StockageFichiers {
    racine: PathBuf,
}

impl StockageFichiers {
    /// Ouvre un stockage sous `racine`, créée au premier dépôt.
    pub fn nouveau(racine: PathBuf) -> Self {
        Self { racine }
    }

    /// Le chemin d'un objet, sa clé une fois ramenée sous la racine.
    fn chemin(&self, cle: &str) -> Result<PathBuf, StorageError> {
        Ok(self.racine.join(normaliser(cle)?))
    }
}

fn indisponible(erreur: std::io::Error) -> StorageError {
    StorageError::Indisponible(Box::new(erreur))
}

#[async_trait]
impl Storage for StockageFichiers {
    async fn deposer(&self, cle: &str, contenu: Vec<u8>) -> Result<(), StorageError> {
        let chemin = self.chemin(cle)?;

        if let Some(parent) = chemin.parent() {
            fs::create_dir_all(parent).await.map_err(indisponible)?;
        }

        fs::write(&chemin, contenu).await.map_err(indisponible)
    }

    async fn lire(&self, cle: &str) -> Result<Vec<u8>, StorageError> {
        fs::read(self.chemin(cle)?).await.map_err(|erreur| {
            if erreur.kind() == ErrorKind::NotFound {
                StorageError::Introuvable(cle.to_owned())
            } else {
                indisponible(erreur)
            }
        })
    }

    // Une clé absente n'est pas une erreur : `DeleteObject` réussit sur une clé que S3 ne
    // trouve pas, et deux backends qui divergeraient là-dessus ne seraient pas
    // substituables l'un à l'autre.
    async fn supprimer(&self, cle: &str) -> Result<(), StorageError> {
        match fs::remove_file(self.chemin(cle)?).await {
            Err(erreur) if erreur.kind() != ErrorKind::NotFound => Err(indisponible(erreur)),
            _ => Ok(()),
        }
    }

    async fn existe(&self, cle: &str) -> Result<bool, StorageError> {
        fs::try_exists(self.chemin(cle)?)
            .await
            .map_err(indisponible)
    }
}
