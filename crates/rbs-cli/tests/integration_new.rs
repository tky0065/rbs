//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_generated_project_compiles_and_passes_its_tests() {
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

    // Le niveau qu'exige la CI que `rbs add ci` pose dans le projet : un squelette qui
    // laisse un warning derrière lui rendrait rouge, dès le premier push, du code que
    // l'utilisateur n'a pas écrit.
    Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .assert()
        .success();

    // `rustfmt` sur les racines de modules et non `cargo fmt` : lancé sous `cargo test`,
    // celui-ci retrouve le workspace de rbs lui-même et signalerait ses fichiers.
    for racine_de_modules in ["src/main.rs", "src/seeds/main.rs", "migration/src/lib.rs"] {
        Command::new("rustfmt")
            .args(["--edition", "2024", "--check"])
            .arg(projet.join(racine_de_modules))
            .assert()
            .success();
    }
}
