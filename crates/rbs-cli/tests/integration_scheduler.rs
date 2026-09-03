//! Le déclenchement calendaire d'un projet réel, joué contre de vraies bases.
//!
//! Deux choses s'y prouvent que rien d'autre ne prouve. Que les tests livrés au projet
//! tournent, d'abord : le fragment n'est compilé nulle part ailleurs, et sa réconciliation
//! comme sa réservation ne disent rien tant qu'aucune transaction n'a été ouverte. Que la
//! réservation **désigne un seul gagnant sur les trois moteurs**, ensuite : c'est la
//! garantie qui justifie d'avoir mis le calendrier en base, et c'est aussi ce qui tranche
//! le choix d'une clé primaire textuelle, que MySQL est seul à contraindre.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Ce que le fragment livre et que `cargo test` joue sans base.
const TESTS_ORDINAIRES: [&str; 3] = [
    "a_five_field_expression_means_the_same_as_its_six_field_form",
    "an_expression_of_any_other_length_is_refused_by_name",
    "an_unparsable_expression_is_refused_even_with_the_right_field_count",
];

/// Ce qu'il livre et qui joint la base.
const TESTS_SOUS_CONTENEUR: [&str; 7] = [
    "a_newly_declared_schedule_is_inserted_with_its_next_occurrence",
    "a_schedule_removed_from_the_code_is_removed_from_the_table",
    "a_redeploy_does_not_move_the_next_occurrence_of_a_known_schedule",
    "an_unparsable_expression_stops_the_reconciliation",
    "a_due_schedule_enqueues_its_job_and_moves_on",
    "a_schedule_that_is_not_due_is_left_alone",
    "concurrent_tickers_trigger_a_due_schedule_exactly_once",
];

/// Le test de concurrence, exigé nommément sur chacun des trois moteurs.
const CONCURRENCE: &str = "concurrent_tickers_trigger_a_due_schedule_exactly_once";

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_scheduler_on("postgres", &common::url_of(&postgres), &parent);

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    migrate_dans(&racine, &common::cible());

    // Les deux flux sont exigés séparément : trois des dix tests livrés n'ont besoin
    // d'aucune base et sortent sous `cargo test` ordinaire, les sept autres sous
    // `--ignored`. Les confondre ferait passer ce test sans qu'un seul des deux groupes
    // soit vraiment joué.
    let (abouti, ordinaires) = cargo_test_brut(&racine, &common::cible(), &[]);
    assert!(abouti, "`cargo test` du projet a échoué :\n{ordinaires}");
    for test in TESTS_ORDINAIRES {
        assert!(
            ordinaires.contains(&format!("test scheduler::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{ordinaires}"
        );
    }

    let (abouti, sous_conteneur) = cargo_test_brut(&racine, &common::cible(), &["--", "--ignored"]);
    assert!(
        abouti,
        "`cargo test -- --ignored` du projet a échoué :\n{sous_conteneur}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces sept lignes, un fragment qui cesserait de livrer ses tests laisserait celui-ci
    // au vert sans qu'une seule transaction ait été ouverte.
    for test in TESTS_SOUS_CONTENEUR {
        assert!(
            sous_conteneur.contains(&format!("test scheduler::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Le critère qui justifie la table : une échéance due n'est gagnée qu'une fois, quel que
/// soit le moteur.
///
/// C'est aussi ce qui tranche la clé primaire textuelle de `schedules` : MySQL refuse un
/// `TEXT` en clé primaire sans longueur d'index, et la migration ne le dit qu'ici.
///
/// Une cible de compilation par moteur : les trois activent des features `sea-orm`
/// différentes, et une cible commune ferait recompiler `sea-orm` et `sqlx` à chaque
/// bascule — y compris pour les tests qui n'ont rien demandé.
#[test]
#[ignore = "démarre PostgreSQL et MySQL et compile un projet par moteur : plusieurs minutes"]
fn a_due_schedule_is_triggered_once_on_the_three_engines() {
    let parent_sqlite = TempDir::new().expect("répertoire temporaire créable");
    let fichier = parent_sqlite.path().join("demo.db");
    let url_sqlite = format!(
        "sqlite://{}?mode=rwc",
        fichier.to_str().expect("chemin représentable")
    );

    let postgres = common::start_postgres();
    let mysql = common::start_mysql();

    for (moteur, url, parent) in [
        ("postgres", common::url_of(&postgres), None),
        ("mysql", common::url_of_mysql(&mysql), None),
        ("sqlite", url_sqlite, Some(&parent_sqlite)),
    ] {
        let propre;
        let parent = match parent {
            Some(parent) => parent,
            None => {
                propre = TempDir::new().expect("répertoire temporaire créable");
                &propre
            }
        };

        eprintln!("── moteur : {moteur} ──");

        let cible = common::cible_pour(moteur);
        // Une cible par moteur, donc un verrou par moteur : les trois branches restent
        // libres de tourner de front avec celles d'un autre binaire de test.
        let _verrou = common::verrou(&cible);
        let racine = project_with_scheduler_on(moteur, &url, parent);

        migrate_dans(&racine, &cible);

        let (abouti, joues) = cargo_test_brut(&racine, &cible, &["--", "--ignored"]);
        assert!(
            abouti,
            "les tests du projet ont échoué sur {moteur} :\n{joues}"
        );
        assert!(
            joues.contains(&format!("test scheduler::tests::{CONCURRENCE} ... ok")),
            "le test de concurrence n'a pas été joué sur {moteur} :\n{joues}"
        );
    }
}

/// Un projet neuf portant `scheduler`, sa base pointée sur `url`.
///
/// Le fragment déclare `requires = ["jobs"]` : `rbs add scheduler` sur un projet nu doit
/// poser la file **puis** le calendrier. Un utilisateur qui ne s'y attend pas verrait
/// apparaître `src/jobs/` sans l'avoir demandé, et c'est la documentation qui le lui dit.
fn project_with_scheduler_on(moteur: &str, url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            moteur,
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

    rbs(&racine).args(["add", "scheduler"]).assert().success();

    assert!(
        racine.join("src/jobs/mod.rs").exists(),
        "le fragment requis n'a pas été entraîné"
    );
    assert!(racine.join("src/scheduler/mod.rs").exists());

    racine
}

fn migrate_dans(racine: &Path, cible: &Path) {
    rbs(racine)
        .env("CARGO_TARGET_DIR", cible)
        .args(["migrate", "up"])
        .assert()
        .success();
}

/// Joue `cargo test` dans le projet et rend son issue et ses deux flux réunis.
fn cargo_test_brut(racine: &Path, cible: &Path, arguments: &[&str]) -> (bool, String) {
    let output = std::process::Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", cible)
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

    (output.status.success(), journal)
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Le fragment ne dépose aucun fichier que le projet ne monterait pas.
///
/// Ce test-ci ne demande ni Docker ni compilation : il tourne sur chaque PR, là où les
/// deux autres attendent qu'on les réclame.
#[test]
fn every_file_the_fragment_ships_is_declared_in_its_manifest() {
    let racine = common::depot().join("crates/rbs-cli/templates/features/scheduler");
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
