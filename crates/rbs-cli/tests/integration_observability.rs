//! `rbs add observability` sur un projet neuf, puis sa compilation.
//!
//! Les tests de rendu prouvent que le fragment écrit les bons fichiers ; celui-ci prouve
//! que la dizaine de crates qu'il tire s'accordent — la façade `metrics` et son
//! exportateur côté projet, les quatre crates OpenTelemetry côté noyau, derrière la
//! feature cargo que le manifeste du fragment lève. C'est le seul test qui puisse le
//! dire, et une version d'écart dans l'écosystème OpenTelemetry le fait tomber.
//!
//! Pas de conteneur : la feature ne joint aucun service. Le `#[ignore]` ne tient qu'à la
//! compilation d'un projet Axum + SeaORM complet.

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les deux tests que le fragment livre et qui gardent la cardinalité du collecteur.
const CARDINALITE: [&str; 2] = [
    "a_request_on_a_parameterised_route_counts_under_its_template",
    "an_unmatched_path_counts_under_a_single_constant",
];

#[test]
#[ignore = "compile un projet Axum + SeaORM complet, et la dizaine de crates du fragment : plusieurs minutes"]
fn the_fragment_compiles_and_keeps_the_cardinality_of_the_collector() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet)
        .args(["add", "observability"])
        .assert()
        .success();

    // Filtré sur le module : les tests de santé du squelette exigeraient une base de
    // données, que cette feature-ci n'a aucune raison de faire monter.
    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    let output = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace", "observability"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "les tests de l'observabilité ont échoué :\n{journal}"
    );

    // Un filtre qui ne retient aucun test sort en 0 : sans ces deux lignes, un fragment
    // qui cesserait de livrer ses tests laisserait celui-ci au vert sans que rien n'ait
    // vérifié sous quelle étiquette une requête est comptée.
    for test in CARDINALITE {
        assert!(
            journal.contains(&format!("test observability::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{journal}"
        );
    }
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
