use std::fs;
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::TryStreamExt;

use super::{Storage, StorageError};

mod files;
mod s3;

/// Un répertoire vide, propre à un test, sous la racine temporaire du système.
fn root(nom: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("storage-{}-{nom}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("le répertoire du test doit se créer");

    path
}

/// Recolle un objet lu, et rend la taille annoncée avec lui.
async fn read(storage: &dyn Storage, key: &str) -> Result<(Option<u64>, Vec<u8>), StorageError> {
    let object = storage.get(key).await?;
    let morceaux: Vec<Bytes> = object
        .body
        .try_collect()
        .await
        .expect("le flux doit se lire");

    Ok((object.length, morceaux.concat()))
}

/// Ce que le trait promet, sans rien connaître du backend qui l'honore.
///
/// Écrite contre `&dyn Storage` pour être rejouable telle quelle contre S3 : deux
/// backends qui ne passeraient pas la même ronde n'abstrairaient rien.
async fn round(storage: &dyn Storage) {
    let key = "factures/2026/janvier.pdf";

    assert!(!storage.exists(key).await.expect("l'existence se consulte"));

    storage
        .put(key, Bytes::from_static(b"%PDF-1.7"))
        .await
        .expect("le dépôt doit aboutir");

    assert!(storage.exists(key).await.expect("l'existence se consulte"));
    assert_eq!(
        read(storage, key).await.expect("la lecture doit aboutir"),
        (Some(8), b"%PDF-1.7".to_vec())
    );

    storage
        .delete(key)
        .await
        .expect("la suppression doit aboutir");

    assert!(!storage.exists(key).await.expect("l'existence se consulte"));
    assert!(
        matches!(storage.get(key).await, Err(StorageError::NotFound(_))),
        "lire un objet supprimé doit rendre `NotFound`"
    );
}

/// Les fichiers réguliers sous `dir`, à toute profondeur.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).expect("le répertoire doit se lire") {
        let path = entry.expect("l'entrée doit se lire").path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}
