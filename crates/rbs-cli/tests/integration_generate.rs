//! Ce que la génération refuse, éprouvé par la commande telle que l'utilisateur la lance.
//!
//! Le test vit ici et non dans le module du générateur : `CARGO_BIN_EXE_rbs`, dont
//! `assert_cmd` a besoin pour trouver le binaire, n'est défini que pour les tests
//! d'intégration. Dans `src/`, il faisait échouer `cargo test -p rbs-cli --bins`.

use std::path::PathBuf;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Un projet neuf, créé par le binaire livré, dans `parent`.
fn projet(parent: &TempDir) -> PathBuf {
    let noyau = common::noyau();

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    parent.path().join("demo-api")
}

/// Un nom qui entrerait en collision avec le squelette ou avec un mot-clé de Rust est
/// refusé avant toute écriture, et le message nomme le fautif.
#[test]
fn un_nom_en_conflit_est_refuse_en_le_nommant_et_sans_rien_ecrire() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet(&parent);

    for nom in ["state", "match"] {
        let sortie = Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&racine)
            .args(["g", "crud", nom, "--fields", "titre:string"])
            .output()
            .expect("le binaire doit être lançable");

        let stderr = String::from_utf8_lossy(&sortie.stderr);
        assert!(
            !sortie.status.success(),
            "`{nom}` a été accepté :\n{stderr}"
        );
        assert!(
            stderr.contains(nom),
            "le conflit doit être nommé :\n{stderr}"
        );
        assert!(
            !racine.join("src").join(nom).exists(),
            "`{nom}` a laissé un répertoire"
        );
    }
}
