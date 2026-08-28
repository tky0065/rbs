//! `rbs upgrade` joué par le binaire livré, sur un projet réel.
//!
//! Ni conteneur ni compilation du projet engendré : la commande ne lit et n'écrit qu'un
//! `Cargo.toml`, et c'est précisément ce que ces tests vérifient — que rien d'autre du
//! projet n'a bougé.

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::Command as Assert;
use tempfile::TempDir;

mod common;

/// Version du binaire en test, sur laquelle un projet en retard sera aligné.
const CLI: &str = env!("CARGO_PKG_VERSION");

/// Un projet en retard reçoit son plan avant que le manifeste ne change, et les deux
/// numéros suivent ensemble.
#[test]
fn a_project_behind_the_binary_is_shown_its_plan_then_aligned() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    dater(&projet, "0.0.1");
    common::commiter(&projet, "initial");

    let rendu = String::from_utf8(
        rbs(&projet)
            .arg("upgrade")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .expect("la sortie est de l'UTF-8");

    assert!(rendu.contains("0.0.1"), "{rendu}");
    assert!(
        rendu.contains("plan pour"),
        "le plan doit être affiché :\n{rendu}"
    );
    assert!(rendu.contains("Cargo.toml"), "{rendu}");

    let manifeste = manifeste(&projet);
    assert!(
        manifeste.contains(&format!("version = \"{CLI}\"\nfeatures")),
        "la métadonnée n'a pas suivi :\n{manifeste}"
    );
    assert!(
        !manifeste.contains("0.0.1"),
        "un 0.0.1 subsiste :\n{manifeste}"
    );
}

/// Seul le manifeste bouge : le reste du projet appartient au développeur.
#[test]
fn nothing_but_the_manifest_is_touched() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    dater(&projet, "0.0.1");
    common::commiter(&projet, "initial");

    rbs(&projet).arg("upgrade").assert().success();

    assert_eq!(git(&projet, &["status", "--porcelain"]), " M Cargo.toml\n");
    assert_eq!(git(&projet, &["diff", "--name-only"]), "Cargo.toml\n");
}

/// Relancée, la commande n'a plus rien à faire et n'écrit rien.
#[test]
fn a_second_run_finds_nothing_to_do() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    dater(&projet, "0.0.1");
    common::commiter(&projet, "initial");

    rbs(&projet).arg("upgrade").assert().success();
    let apres = manifeste(&projet);

    let rendu = String::from_utf8(
        rbs(&projet)
            .arg("upgrade")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .expect("la sortie est de l'UTF-8");

    assert!(rendu.contains("rien à faire"), "{rendu}");
    assert_eq!(manifeste(&projet), apres, "le manifeste a été réécrit");
}

/// Un projet postérieur au binaire est refusé, les deux numéros nommés.
#[test]
fn a_project_ahead_of_the_binary_is_refused_by_naming_both_versions() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    dater(&projet, "9.9.9");
    common::commiter(&projet, "initial");

    let sortie = rbs(&projet).arg("upgrade").assert().failure();
    let rendu =
        String::from_utf8(sortie.get_output().stderr.clone()).expect("la sortie est de l'UTF-8");

    assert!(rendu.contains("9.9.9"), "{rendu}");
    assert!(rendu.contains(CLI), "{rendu}");
}

/// Réécrit la version que `[package.metadata.rbs]` garde de la génération.
fn dater(projet: &Path, version: &str) {
    let manifeste = manifeste(projet);
    let avant = format!("version = \"{CLI}\"\nfeatures");
    assert!(manifeste.contains(&avant), "{manifeste}");

    fs::write(
        projet.join("Cargo.toml"),
        manifeste.replace(&avant, &format!("version = \"{version}\"\nfeatures")),
    )
    .expect("le manifeste est réécrivable");
}

fn manifeste(projet: &Path) -> String {
    fs::read_to_string(projet.join("Cargo.toml")).expect("le manifeste est lisible")
}

fn rbs(projet: &Path) -> Assert {
    let mut commande = Assert::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(projet);
    commande
}

fn git(projet: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(projet)
        .output()
        .expect("git doit être lançable");

    assert!(output.status.success(), "git {arguments:?} a échoué");

    String::from_utf8_lossy(&output.stdout).into_owned()
}
