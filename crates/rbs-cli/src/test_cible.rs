//! La cible de compilation que se partagent les projets d'essai, et le verrou qui la garde.
//!
//! Les tests du dépôt bâtissent des projets entiers dans une même `target/rbs-integration`
//! pour n'avoir pas à recompiler Axum et SeaORM à chacun. Un `Mutex` y suffisait tant que
//! les écrivains vivaient dans un seul processus ; `crates/rbs-cli/tests/*` sont des
//! binaires séparés, que `cargo test` lance de front et dont aucun ne voit le verrou de
//! l'autre. D'où, sur une suite `--ignored` complète, des `could not parse/generate dep
//! info` et des `failed to write .fingerprint/…` levés par des tests sans rapport entre
//! eux, tous verts au rejeu isolé.
//!
//! Le fichier se partage par `#[path]` et non par un item public, comme `test_postgres` :
//! `generate::bench` est un module `#[cfg(test)]` de la bibliothèque, `tests/common`
//! appartient à un autre crate, et aucune visibilité ne relie ces deux mondes de
//! compilation sans élargir l'API publique de `rbs-cli` au bénéfice des seuls tests.

use std::fs::{File, OpenOptions};
use std::path::Path;

/// Prend `cible` pour soi jusqu'à ce que le garde rendu soit lâché.
///
/// Le verrou est rendu plutôt que posé et relâché sur place : sa portée appartient à
/// l'appelant. Le temps d'un `cargo` suffit à qui ne fait que compiler ; un test qui lance
/// ensuite le binaire bâti doit le tenir jusqu'à l'arrêt de celui-ci, cargo relâchant le
/// sien avant l'exécution — un autre projet écrirait alors `debug/demo-api` sous les pieds
/// du processus en cours.
pub fn verrou(cible: &Path) -> File {
    std::fs::create_dir_all(cible).expect("la cible de compilation doit être créable");

    let fichier = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(cible.join(".rbs-verrou"))
        .expect("le verrou de la cible doit s'ouvrir");

    // `File::lock` est stable depuis Rust 1.89, en deçà du plancher du dépôt : le verrou
    // inter-processus ne demande aucune dépendance de plus, et il est relâché à la
    // fermeture du fichier, donc à la chute du garde.
    fichier
        .lock()
        .expect("le verrou de la cible doit se prendre");

    fichier
}
