//! Ce que la génération refuse, éprouvé par la commande telle que l'utilisateur la lance.
//!
//! Le test vit ici et non dans le module du générateur : `CARGO_BIN_EXE_rbs`, dont
//! `assert_cmd` a besoin pour trouver le binaire, n'est défini que pour les tests
//! d'intégration. Dans `src/`, il faisait échouer `cargo test -p rbs-cli --bins`.

use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Un nom qui entrerait en collision avec le squelette ou avec un mot-clé de Rust est
/// refusé avant toute écriture, et le message nomme le fautif.
#[test]
fn a_clashing_name_is_rejected_by_naming_it_and_without_writing_anything() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    for nom in ["state", "match"] {
        let output = Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&racine)
            .args(["g", "crud", nom, "--fields", "titre:string"])
            .output()
            .expect("le binaire doit être lançable");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
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

/// Une référence requise rend l'entité non semable : aucun seed n'est écrit, le montage
/// ne l'inscrit pas non plus, et la commande le dit dans sa sortie — sans quoi l'absence
/// du fichier se découvrirait en le cherchant en vain.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet"]
fn a_required_reference_leaves_no_seed_and_names_it_in_the_output() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args(["g", "crud", "users", "--fields", "email:string:unique"])
        .assert()
        .success();

    let output = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args([
            "g",
            "crud",
            "posts",
            "--fields",
            "title:string,author:references:users",
        ])
        .output()
        .expect("le binaire doit être lançable");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "la génération doit aboutir malgré la référence requise :\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("aucun seed pour posts") && stdout.contains("« author »"),
        "la sortie doit nommer la relation en cause :\n{stdout}"
    );

    assert!(
        !racine.join("src/seeds/posts.rs").exists(),
        "un seed a été écrit malgré la référence requise"
    );

    let binaire_des_seeds =
        fs::read_to_string(racine.join("src/seeds/main.rs")).expect("le binaire des seeds se lit");
    assert!(
        !binaire_des_seeds.contains("posts,"),
        "le seed écarté ne doit pas être monté :\n{binaire_des_seeds}"
    );

    // Le seul juge qui compte : un `mod` vers un fichier absent ne compilerait pas.
    let compilation = std::process::Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["build", "--workspace"])
        .output()
        .expect("cargo doit être lançable");
    assert!(
        compilation.status.success(),
        "le projet ne compile pas :\n{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
}
