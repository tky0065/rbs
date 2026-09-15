use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use tokio::fs;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use super::{Object, Storage, StorageError, normalize};

/// Stockage sur le système de fichiers local, sous une racine unique.
#[derive(Debug, Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    /// Ouvre un stockage sous `root`, créée ici.
    ///
    /// Au démarrage plutôt qu'au premier dépôt : une racine qui ne se crée pas — droit
    /// absent, chemin sous un fichier — est une erreur de configuration, et elle se voit
    /// avant que la première requête ne la découvre en 500. C'est aussi ce qui laisse à
    /// `available` quelque chose à constater sans rien écrire.
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&root).map_err(|error| {
            unavailable(std::io::Error::new(
                error.kind(),
                format!("la racine {} ne se crée pas : {error}", root.display()),
            ))
        })?;

        Ok(Self { root })
    }

    /// Le chemin d'un objet, sa clé une fois ramenée sous la racine.
    fn path(&self, key: &str) -> Result<PathBuf, StorageError> {
        Ok(self.root.join(normalize(key)?))
    }
}

fn unavailable(error: std::io::Error) -> StorageError {
    StorageError::Unavailable(Box::new(error))
}

/// Écrit `content` dans un temporaire à côté de `path`, puis le renomme sur elle.
///
/// Même répertoire, donc même système de fichiers : le `rename` est atomique, et un
/// lecteur voit l'ancien objet ou le nouveau, jamais un fichier tronqué. Un UUID par
/// dépôt : deux dépôts concurrents sur la même clé n'écrivent jamais dans le même
/// temporaire, et le dernier renommé l'emporte.
fn deposit(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut partial = path.as_os_str().to_owned();
    partial.push(format!(".{}.partiel", Uuid::new_v4()));
    let partial = PathBuf::from(partial);

    let deposited = std::fs::File::create(&partial).and_then(|mut file| {
        file.write_all(content)?;
        // Le renommage peut atteindre le disque avant les données : sans cette
        // synchronisation, une coupure laisserait un fichier vide sous le nom final.
        file.sync_all()?;
        // Windows refuse de renommer un fichier encore ouvert.
        drop(file);
        std::fs::rename(&partial, path)
    });

    if deposited.is_err() {
        let _ = std::fs::remove_file(&partial);
    }

    deposited
}

#[async_trait]
impl Storage for FileStorage {
    async fn put(&self, key: &str, content: Bytes) -> Result<(), StorageError> {
        let path = self.path(key)?;

        // Un seul saut vers le pool bloquant pour toute la séquence : elle n'a aucun
        // point où reprendre autrement que du début, et `tokio::fs` en ferait quatre.
        tokio::task::spawn_blocking(move || deposit(&path, &content))
            .await
            .map_err(|error| unavailable(std::io::Error::other(error)))?
            .map_err(unavailable)
    }

    // L'ouverture tranche `NotFound` avant qu'un octet ne parte ; la lecture suit ensuite
    // le client, morceau par morceau. Un dépôt concurrent renomme un autre fichier sur la
    // clé : celui qui est ouvert reste entier jusqu'au bout du flux.
    async fn get(&self, key: &str) -> Result<Object, StorageError> {
        let file = fs::File::open(self.path(key)?).await.map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                StorageError::NotFound(key.to_owned())
            } else {
                unavailable(error)
            }
        })?;
        let length = file.metadata().await.map_err(unavailable)?.len();

        Ok(Object {
            length: Some(length),
            body: ReaderStream::new(file).boxed(),
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

    // La racine est créée à la construction : la sonde constate qu'elle est toujours là,
    // sans rien écrire — la recréer masquerait précisément la perte qu'elle doit montrer.
    // Un droit d'écriture retiré ou un volume remonté en lecture seule ne se voient pas
    // ici : ils se voient au premier dépôt, en 500 dans le journal.
    async fn available(&self) -> bool {
        fs::metadata(&self.root)
            .await
            .is_ok_and(|metadata| metadata.is_dir())
    }
}
