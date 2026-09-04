//! Le journal d'un projet réel, joué contre un PostgreSQL en conteneur.
//!
//! Ce qui s'y prouve et que rien d'autre ne prouve : que la trace disparaît avec la
//! transaction annulée qui la motivait. C'est la garantie qui justifie d'avoir mis le
//! journal en base, et elle ne veut rien dire tant qu'aucune vraie transaction n'a été
//! ouverte.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les tests que le fragment livre au projet et qui joignent la base.
const TESTS: [&str; 4] = [
    "an_entry_written_in_a_rolled_back_transaction_does_not_exist",
    "an_entry_reads_back_with_every_field_it_was_given",
    "an_entry_without_an_actor_stores_null_rather_than_an_empty_string",
    "the_entries_of_one_row_read_back_in_order",
];

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_audit(&common::url_of(&postgres), &parent);

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    migrate(&racine);

    let ordinaires = cargo_test(&racine, &[]);
    assert!(
        ordinaires.contains("test result: ok"),
        "`cargo test` du projet a échoué :\n{ordinaires}"
    );

    let sous_conteneur = cargo_test(&racine, &["--", "--ignored"]);

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces quatre lignes, un fragment qui cesserait de livrer ses tests laisserait
    // celui-ci au vert sans qu'une seule transaction ait été ouverte.
    for test in TESTS {
        assert!(
            sous_conteneur.contains(&format!("test audit::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Un projet neuf portant `audit`, sa base pointée sur `url`.
fn project_with_audit(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            "postgres",
            "--database-url",
            url,
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "audit"]).assert().success();

    racine
}

fn migrate(racine: &Path) {
    rbs(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();
}

/// Joue `cargo test` dans le projet et rend ses deux flux réunis.
fn cargo_test(racine: &Path, arguments: &[&str]) -> String {
    let output = std::process::Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("test")
        .arg("--workspace")
        .args(arguments)
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "les tests du projet ont échoué :\n{journal}"
    );

    journal
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Le fragment ne dépose aucun fichier que le projet ne monterait pas.
///
/// Ce test-ci ne demande ni Docker ni compilation : il tourne sur chaque PR, là où
/// l'autre attend qu'on le réclame.
#[test]
fn every_file_the_fragment_ships_is_declared_in_its_manifest() {
    let racine = common::depot().join("crates/rbs-cli/templates/features/audit");
    let manifeste = fs::read_to_string(racine.join("feature.toml")).expect("manifeste lisible");

    for entree in fs::read_dir(&racine).expect("le fragment se lit") {
        let nom = entree
            .expect("entrée lisible")
            .file_name()
            .to_string_lossy()
            .into_owned();

        if nom == "feature.toml" {
            continue;
        }

        assert!(
            manifeste.contains(&nom),
            "`{nom}` est livrée sans être déclarée dans feature.toml"
        );
    }
}
