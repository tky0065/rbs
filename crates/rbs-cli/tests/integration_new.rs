//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_projet_genere_compile_et_passe_ses_tests() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = depot().join("crates/rbs-core");

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
            .env("CARGO_TARGET_DIR", depot().join("target/rbs-integration"))
            .args([action, "--workspace"])
            .assert()
            .success();
    }
}
