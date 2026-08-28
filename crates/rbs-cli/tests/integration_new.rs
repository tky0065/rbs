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

/// Le second critère : une valeur inconnue est refusée en nommant les trois admises.
///
/// Le contrôle appartient à clap, qui énumère déjà les valeurs d'un `ValueEnum` : ce test
/// constate que rbs ne s'est pas mis en travers, non qu'un message maison existe.
#[test]
fn an_unknown_engine_is_refused_naming_the_three_admitted() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args(["new", "demo-api", "--database", "oracle", "--yes"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let message = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    for admise in ["postgres", "mysql", "sqlite"] {
        assert!(
            message.contains(admise),
            "le refus ne nomme pas `{admise}` :\n{message}"
        );
    }
    assert!(
        !parent.path().join("demo-api").exists(),
        "un projet a été créé malgré le refus"
    );
}

/// Le premier critère : les trois valeurs produisent un projet qui compile.
///
/// PostgreSQL garde la vérification complète du test ci-dessus — build, test, clippy et
/// rustfmt. MySQL et SQLite s'arrêtent à `cargo build`, qui est exactement ce que le
/// critère demande, et chacun compile dans sa propre cible : leurs features `sea-orm`
/// diffèrent, et un arbre commun les ferait se recompiler l'un l'autre.
#[test]
#[ignore = "compile un projet Axum + SeaORM par moteur : plusieurs minutes"]
fn each_engine_produces_a_project_that_compiles() {
    let noyau = common::noyau();

    for (moteur, url) in [
        ("mysql", "mysql://root:root@localhost:3306/demo_api"),
        ("sqlite", "sqlite://demo_api.db?mode=rwc"),
    ] {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(parent.path())
            .args([
                "new",
                "demo-api",
                "--database",
                moteur,
                "--database-url",
                url,
                "--core-path",
                noyau.to_str().expect("chemin du noyau représentable"),
                "--yes",
            ])
            .assert()
            .success();

        let projet = parent.path().join("demo-api");

        Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", common::cible_pour(moteur))
            .args(["build", "--workspace"])
            .assert()
            .success();
    }
}
