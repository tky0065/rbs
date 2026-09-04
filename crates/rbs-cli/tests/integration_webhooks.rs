//! Les webhooks sortants d'un projet réel, joués contre une vraie base.
//!
//! C'est la seule chose qui prouve que le fragment fonctionne : il n'est compilé nulle part
//! ailleurs, et ni sa signature, ni sa correspondance de motifs, ni son émission dans la
//! transaction du métier ne disent quoi que ce soit tant qu'aucun projet ne les a joués.
//!
//! **Pas de test sur les trois moteurs**, et ce n'est pas un oubli : ce que webhooks ajoute
//! au schéma est une colonne JSON et une date nullable, or
//! `integration_jobs::the_dequeue_never_hands_the_same_job_twice_on_the_three_engines`
//! éprouve déjà la colonne JSON de la file sur PostgreSQL, MySQL et SQLite. Un troisième
//! projet compilé par moteur coûterait plusieurs minutes pour reprouver la même chose.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Ce que le fragment livre et que `cargo test` joue sans base.
const TESTS_ORDINAIRES: [&str; 6] = [
    "the_signature_matches_an_independently_computed_vector",
    "the_signature_changes_with_the_timestamp",
    "the_signature_header_carries_the_timestamp_and_the_v1_digest",
    "an_exact_pattern_matches_only_its_own_event",
    "a_prefix_pattern_matches_every_event_of_its_family",
    "the_star_pattern_matches_every_event",
];

/// Ce qu'il livre et qui joint la base.
const TESTS_SOUS_CONTENEUR: [&str; 5] = [
    "emitting_an_event_enqueues_one_delivery_per_listening_subscription",
    "a_revoked_subscription_is_not_delivered_to",
    "a_subscription_that_does_not_listen_receives_nothing",
    "a_delivery_whose_subscription_was_revoked_succeeds_without_a_request",
    "an_emission_rolled_back_with_its_transaction_enqueues_nothing",
];

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_webhooks_on("postgres", &common::url_of(&postgres), &parent);

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    migrate_dans(&racine, &common::cible());

    // Les deux flux sont exigés séparément : six des onze tests livrés n'ont besoin
    // d'aucune base et sortent sous `cargo test` ordinaire, les cinq autres sous
    // `--ignored`. Les confondre ferait passer ce test sans qu'un seul des deux groupes
    // soit vraiment joué.
    let (abouti, ordinaires) = cargo_test_brut(&racine, &common::cible(), &[]);
    assert!(abouti, "`cargo test` du projet a échoué :\n{ordinaires}");
    for test in TESTS_ORDINAIRES {
        assert!(
            ordinaires.contains(&format!("test webhooks::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{ordinaires}"
        );
    }

    let (abouti, sous_conteneur) = cargo_test_brut(&racine, &common::cible(), &["--", "--ignored"]);
    assert!(
        abouti,
        "`cargo test -- --ignored` du projet a échoué :\n{sous_conteneur}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces cinq lignes, un fragment qui cesserait de livrer ses tests laisserait celui-ci
    // au vert sans qu'une seule transaction ait été ouverte.
    for test in TESTS_SOUS_CONTENEUR {
        assert!(
            sous_conteneur.contains(&format!("test webhooks::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Un projet neuf portant `webhooks`, sa base pointée sur `url`.
///
/// Le fragment déclare `requires = ["jobs", "auth"]`, et `auth` exige à son tour
/// `rate-limit` : `rbs add webhooks` sur un projet nu doit poser les trois **puis** les
/// webhooks. Un utilisateur qui ne s'y attend pas verrait apparaître trois répertoires
/// qu'il n'a pas demandés, et c'est la documentation qui le lui dit.
fn project_with_webhooks_on(moteur: &str, url: &str, parent: &TempDir) -> PathBuf {
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

    rbs(&racine).args(["add", "webhooks"]).assert().success();

    for requis in [
        "src/jobs/mod.rs",
        "src/auth/mod.rs",
        "src/rate_limit/mod.rs",
    ] {
        assert!(
            racine.join(requis).exists(),
            "le fragment requis n'a pas été entraîné : {requis}"
        );
    }
    assert!(racine.join("src/webhooks/mod.rs").exists());

    // Le worker n'exécute que ce que le registre connaît : sans cette ligne, chaque
    // livraison partirait en réessai puis en échec sous « aucun job n'est inscrit », et la
    // compilation du projet ne le dirait pas.
    let registre = fs::read_to_string(racine.join("src/jobs/mod.rs")).expect("registre lisible");
    assert!(
        registre.contains(".register::<crate::webhooks::delivery::Delivery>()"),
        "le job de livraison n'est pas inscrit au registre :\n{registre}"
    );

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
/// Ce test-ci ne demande ni Docker ni compilation : il tourne sur chaque PR, là où l'autre
/// attend qu'on le réclame.
#[test]
fn every_file_the_fragment_ships_is_declared_in_its_manifest() {
    let racine = common::depot().join("crates/rbs-cli/templates/features/webhooks");
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
