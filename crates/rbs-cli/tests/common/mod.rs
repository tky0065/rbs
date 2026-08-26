//! Ce que les tests d'intégration partagent : où trouver le dépôt, le noyau et la cible.
//!
//! Ces tests compilent des projets Axum + SeaORM complets. La cible commune n'est pas un
//! détail de confort : sans elle, chaque test recompile toute l'arborescence de
//! dépendances pour son compte.

use std::path::{Path, PathBuf};

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
pub fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

/// Le noyau du dépôt, dont dépendra le projet créé.
pub fn noyau() -> PathBuf {
    depot().join("crates/rbs-core")
}

/// Répertoire de compilation partagé par les projets créés en test.
pub fn cible() -> PathBuf {
    depot().join("target/rbs-integration")
}
