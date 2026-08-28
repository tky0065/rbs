//! `rbs seed` vu du dehors : le binaire livré, lancé sur un projet créé par lui.
//!
//! Ces tests ne compilent rien. C'est le point : chacun vérifie que la commande s'arrête
//! **avant** cargo, et l'absence de `target/` dans le projet en est la preuve matérielle.

use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Le critère du squelette : un projet vierge sort vert, et ne compile rien.
#[test]
fn on_a_fresh_project_the_command_says_there_is_nothing_to_insert() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let succes = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&projet)
        .arg("seed")
        .assert()
        .success();

    let sortie = String::from_utf8_lossy(&succes.get_output().stdout).into_owned();
    assert!(
        sortie.contains("rien à insérer"),
        "le message doit dire qu'il n'y a rien à insérer :\n{sortie}"
    );
    assert!(
        !projet.join("target").exists(),
        "cargo a tourné là où il n'y avait rien à insérer"
    );
}

/// Le premier critère du lot : la production refuse, et le binaire du projet ne part pas.
#[test]
fn under_production_the_command_refuses_without_launching_the_project_binary() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let echec = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&projet)
        .env("RBS_ENV", "production")
        .arg("seed")
        .assert()
        .failure();

    let sortie = String::from_utf8_lossy(&echec.get_output().stderr).into_owned();
    assert!(
        sortie.contains("--force"),
        "le refus doit nommer l'échappatoire :\n{sortie}"
    );
    assert!(
        sortie.contains("production"),
        "le refus doit nommer l'environnement :\n{sortie}"
    );
    assert!(
        !projet.join("target").exists(),
        "cargo a tourné : le binaire du projet a été lancé malgré le refus"
    );
}

/// Le second critère : dire comment créer le binaire, non buter sur cargo.
#[test]
fn a_project_without_seeds_says_how_to_create_one() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    let _ = fs::remove_dir_all(projet.join("src/seeds"));

    let echec = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&projet)
        .arg("seed")
        .assert()
        .failure();

    let output = echec.get_output();
    let sortie = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        sortie.contains("src/seeds/main.rs"),
        "le message doit nommer ce qui manque :\n{sortie}"
    );
    assert!(
        sortie.contains("[[bin]]"),
        "le message doit dire comment créer le binaire :\n{sortie}"
    );
    assert!(
        !sortie.contains("no bin target") && !sortie.contains("error: "),
        "une erreur de cargo est passée jusqu'à l'utilisateur :\n{sortie}"
    );
    assert!(
        !projet.join("target").exists(),
        "cargo a tourné là où il n'y avait rien à lancer"
    );
}
