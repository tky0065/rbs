//! Le code de sortie, vu depuis la ligne de commande.
//!
//! Trois familles, pour un script qui ne lit pas le message : 1 quand le projet porte une
//! faute, 2 quand l'appel est à corriger, 3 quand l'environnement a manqué. Aucun
//! `#[ignore]` : ni Docker ni compilation du projet engendré.

mod common;

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// Le code rendu par `rbs arguments` lancé depuis `repertoire`, et ce qu'il a écrit.
fn lancer(repertoire: &Path, arguments: &[&str]) -> (i32, String) {
    let rendu = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .args(arguments)
        .current_dir(repertoire)
        .output()
        .expect("le binaire doit s'exécuter");

    let sortie = format!(
        "{}{}",
        String::from_utf8_lossy(&rendu.stdout),
        String::from_utf8_lossy(&rendu.stderr)
    );

    (rendu.status.code().unwrap_or(-1), sortie)
}

/// Un projet neuf dont la base est hors d'atteinte.
///
/// Le port 1 refuse sur-le-champ : `doctor` conclut sans bâtir la crate `migration` pour
/// demander sa version à un serveur qui ne répondrait pas.
fn projet_hors_ligne(parent: &Path) -> PathBuf {
    let noyau = common::noyau();

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent)
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@127.0.0.1:1/demo_api",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    parent.join("demo-api")
}

#[test]
fn doctor_outside_a_project_is_a_call_to_correct() {
    let ailleurs = TempDir::new().expect("répertoire temporaire créable");

    let (code, sortie) = lancer(ailleurs.path(), &["doctor"]);

    assert_eq!(code, 2, "{sortie}");
}

#[test]
fn an_unknown_feature_is_a_call_to_correct() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_hors_ligne(parent.path());

    let (code, sortie) = lancer(&racine, &["add", "inconnue"]);

    assert_eq!(code, 2, "{sortie}");
}

/// Commité d'abord : sans quoi c'est la garde du working tree qui répondrait, et son
/// code n'est pas celui d'une ancre disparue.
#[test]
fn a_missing_anchor_is_a_fault_of_the_project() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_hors_ligne(parent.path());
    let routeur = racine.join("src/router.rs");
    let source = std::fs::read_to_string(&routeur).expect("le routeur est lisible");
    let sans_ancre: Vec<&str> = source
        .lines()
        .filter(|ligne| !ligne.contains("rbs:layers>"))
        .collect();
    std::fs::write(&routeur, sans_ancre.join("\n")).expect("le routeur est réécrivable");
    common::commiter(&racine, "retire l'ancre des layers");

    let (code, sortie) = lancer(&racine, &["add", "cors"]);

    assert_eq!(code, 1, "{sortie}");
    assert!(sortie.contains("<rbs:layers>"), "{sortie}");
}

#[cfg(unix)]
#[test]
fn a_directory_that_refuses_writing_is_the_environment() {
    use std::os::unix::fs::PermissionsExt;

    let parent = TempDir::new().expect("répertoire temporaire créable");
    let fermer = |mode: u32| {
        std::fs::set_permissions(parent.path(), std::fs::Permissions::from_mode(mode))
            .expect("les droits du répertoire temporaire se changent");
    };
    fermer(0o555);

    let (code, sortie) = lancer(
        parent.path(),
        &[
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@127.0.0.1:1/demo_api",
            "--yes",
        ],
    );

    // Rouvert avant l'assertion : `TempDir` doit pouvoir se supprimer même en échec.
    fermer(0o755);
    assert_eq!(code, 3, "{sortie}");
}

/// Un diagnostic qui trouve une faute a tourné jusqu'au bout : son code n'est pas celui
/// d'un diagnostic empêché.
#[test]
fn doctor_on_a_project_whose_base_does_not_answer_is_a_fault_found() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_hors_ligne(parent.path());

    let (code, sortie) = lancer(&racine, &["doctor"]);

    assert_eq!(code, 1, "{sortie}");
    assert!(sortie.contains("127.0.0.1:1"), "{sortie}");
}
