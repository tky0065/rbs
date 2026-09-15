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

/// Ce que le fragment livre et que `cargo test` joue sans base, nommés avec le sous-module
/// — `signature` ou `target` — où le découpage des tests l'a rangé.
const TESTS_ORDINAIRES: [(&str, &str); 14] = [
    (
        "signature",
        "the_signature_matches_an_independently_computed_vector",
    ),
    ("signature", "the_signature_changes_with_the_timestamp"),
    (
        "signature",
        "the_signature_header_carries_the_timestamp_and_the_v1_digest",
    ),
    ("signature", "an_exact_pattern_matches_only_its_own_event"),
    (
        "signature",
        "a_prefix_pattern_matches_every_event_of_its_family",
    ),
    ("signature", "the_star_pattern_matches_every_event"),
    (
        "target",
        "private_loopback_and_link_local_addresses_are_not_public",
    ),
    ("target", "public_addresses_are_public"),
    (
        "target",
        "outside_development_only_https_to_a_public_host_passes",
    ),
    (
        "target",
        "in_development_http_and_private_hosts_pass_but_not_other_schemes",
    ),
    (
        "target",
        "the_resolver_drops_localhost_outside_development_and_keeps_it_in_development",
    ),
    (
        "target",
        "a_refusal_from_the_resolver_is_found_in_the_client_error",
    ),
    ("target", "a_connection_failure_is_not_a_refusal"),
    ("target", "a_refusal_wrapped_in_an_io_error_is_found"),
];

/// Ce qu'il livre et qui joint la base, nommés avec le sous-module — `emission`, `routes`
/// ou `blocked` — où le découpage des tests l'a rangé.
const TESTS_SOUS_CONTENEUR: [(&str, &str); 13] = [
    (
        "emission",
        "emitting_an_event_enqueues_one_delivery_per_listening_subscription",
    ),
    ("emission", "a_revoked_subscription_is_not_delivered_to"),
    (
        "emission",
        "a_subscription_that_does_not_listen_receives_nothing",
    ),
    (
        "emission",
        "a_delivery_whose_subscription_was_revoked_succeeds_without_a_request",
    ),
    (
        "emission",
        "an_emission_rolled_back_with_its_transaction_enqueues_nothing",
    ),
    ("routes", "a_user_role_is_refused_on_the_three_routes"),
    ("routes", "an_admin_subscribes_then_reads_and_revokes"),
    ("routes", "an_empty_pattern_is_refused_by_validation"),
    ("routes", "revoking_twice_keeps_the_first_date"),
    ("blocked", "an_admin_subscribing_a_private_url_gets_400"),
    ("blocked", "post_refuses_a_blocked_target_before_sending"),
    (
        "blocked",
        "a_delivery_to_a_blocked_target_is_abandoned_not_retried",
    ),
    ("blocked", "in_development_a_local_receiver_is_reached"),
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

    // Les deux flux sont exigés séparément : quatorze des vingt-sept tests livrés n'ont
    // besoin d'aucune base et sortent sous `cargo test` ordinaire, les treize autres sous
    // `--ignored`. Les confondre ferait passer ce test sans qu'un seul des deux groupes
    // soit vraiment joué.
    let (abouti, ordinaires) = cargo_test_brut(&racine, &common::cible(), &[]);
    assert!(abouti, "`cargo test` du projet a échoué :\n{ordinaires}");
    for (sous_module, test) in TESTS_ORDINAIRES {
        assert!(
            ordinaires.contains(&format!(
                "test modules::webhooks::tests::{sous_module}::{test} ... ok"
            )),
            "`{test}` n'a pas été exécuté :\n{ordinaires}"
        );
    }

    let (abouti, sous_conteneur) = cargo_test_brut(&racine, &common::cible(), &["--", "--ignored"]);
    assert!(
        abouti,
        "`cargo test -- --ignored` du projet a échoué :\n{sous_conteneur}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces treize lignes, un fragment qui cesserait de livrer ses tests laisserait celui-ci
    // au vert sans qu'une seule transaction ait été ouverte.
    for (sous_module, test) in TESTS_SOUS_CONTENEUR {
        assert!(
            sous_conteneur.contains(&format!(
                "test modules::webhooks::tests::{sous_module}::{test} ... ok"
            )),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Un projet neuf portant `webhooks`, sa base pointée sur `url`.
///
/// Le fragment déclare `requires = ["jobs", "auth"]`, et `auth` exige à son tour
/// `rate-limit` et `mail` : `rbs add webhooks` sur un projet nu doit poser les quatre
/// **puis** les webhooks. Un utilisateur qui ne s'y attend pas verrait apparaître quatre
/// répertoires qu'il n'a pas demandés, et c'est la documentation qui le lui dit.
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
        "src/modules/jobs/mod.rs",
        "src/auth/mod.rs",
        "src/modules/rate_limit/mod.rs",
        "src/modules/mail/mod.rs",
    ] {
        assert!(
            racine.join(requis).exists(),
            "le fragment requis n'a pas été entraîné : {requis}"
        );
    }
    assert!(racine.join("src/modules/webhooks/mod.rs").exists());

    // Le worker n'exécute que ce que le registre connaît : sans cette ligne, chaque
    // livraison partirait en réessai puis en échec sous « aucun job n'est inscrit », et la
    // compilation du projet ne le dirait pas.
    let registre =
        fs::read_to_string(racine.join("src/modules/jobs/mod.rs")).expect("registre lisible");
    assert!(
        registre.contains(".register::<crate::modules::webhooks::delivery::Delivery>()"),
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
