//! `rbs add frontend` sur un projet neuf, puis sa compilation.
//!
//! Le fragment ne dépose encore aucune ligne de client : il pose le module qui servira le
//! build, et la page d'amorçage que le projet rend tant que ce build n'existe pas. Ce test
//! est le seul endroit où cette page est réellement servie — par le routeur du projet,
//! monté comme `main.rs` le monte, avec la documentation OpenAPI et la sonde de santé à
//! côté d'elle.
//!
//! Il vaut aussi pour ce qu'il ne fait pas : `cargo` ne lance ni `npm`, ni `node`, ni
//! aucun outil du client. Un projet qui a installé le frontend compile sur une machine qui
//! n'a pas Node — c'est la contrepartie du mécanisme purement déclaratif des fragments.
//!
//! Pas de conteneur : le repli ne joint aucun service, et l'état se monte sur une
//! connexion non établie. Le `#[ignore]` ne tient qu'à la compilation d'un projet
//! Axum + SeaORM complet.

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les tests que le fragment livre et qui portent ses deux promesses : la page d'amorçage
/// servie par le routeur réel, et le service qui ne masque rien.
const PROMESSES: [&str; 4] = [
    "the_project_router_serves_the_bootstrap_page_at_its_root",
    "the_project_router_still_answers_its_own_health_probe",
    "a_mounted_route_is_never_reached_by_the_fallback",
    "the_build_erases_the_bootstrap_page_as_soon_as_it_exists",
];

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_fragment_compiles_and_serves_its_bootstrap_page() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "frontend"]).assert().success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    let output = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        // Filtré sur le module : les tests de santé du squelette exigeraient une base de
        // données, que ce fragment-ci n'a aucune raison de faire monter.
        .args(["test", "--lib", "modules::frontend"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "les tests du frontend ont échoué :\n{journal}"
    );

    // Un filtre qui ne retient aucun test sort en 0 : sans ces lignes, un fragment qui
    // cesserait de livrer ses tests laisserait celui-ci au vert sans que rien n'ait servi
    // la moindre page.
    for test in PROMESSES {
        assert!(
            journal.contains(&format!("test modules::frontend::tests::{test} ... ok")),
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
