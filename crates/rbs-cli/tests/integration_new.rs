//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_projet_genere_compile_et_passe_ses_tests() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
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

    let projet = parent.path().join("demo-api");
    assert!(projet.join("Cargo.toml").is_file(), "projet non créé");

    for action in ["build", "test"] {
        Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([action, "--workspace"])
            .assert()
            .success();
    }
}
