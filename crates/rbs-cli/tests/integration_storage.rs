//! Le stockage d'un projet réel, joué contre un MinIO en conteneur.
//!
//! `N1` et `N2` prouvent que les deux backends compilent et que le second se construit
//! sans joindre le réseau. Aucun ne montre qu'ils sont **substituables** : c'est ce que
//! fait ici la ronde du trait, rejouée contre S3 sans une ligne de différence.

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;

mod common;

const IMAGE: (&str, &str) = ("minio/minio", "latest");

const BUCKET: &str = "demo";
const CLE: &str = "rbs-test";
const SECRET: &str = "rbs-test-secret";

const TESTS: [&str; 2] = [
    "the_s3_backend_passes_the_same_round_as_the_file_backend",
    "an_object_put_by_the_trait_reads_back_through_the_s3_client",
];

#[test]
#[ignore = "démarre MinIO et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn both_backends_pass_the_same_round_and_the_object_reads_back_outside_the_trait() {
    // Le bucket est créé par le conteneur et non par un test livré à l'utilisateur : un
    // répertoire de premier niveau de `/data` *est* un bucket pour MinIO. Le fragment se
    // contente alors de déposer et de lire, comme face à un bucket de production.
    let minio = GenericImage::new(IMAGE.0, IMAGE.1)
        .with_wait_for(WaitFor::log(LogWaitStrategy::stdout_or_stderr("API:")))
        .with_entrypoint("sh")
        .with_env_var("MINIO_ROOT_USER", CLE)
        .with_env_var("MINIO_ROOT_PASSWORD", SECRET)
        .with_cmd([
            "-c",
            &format!("mkdir -p /data/{BUCKET} && minio server /data"),
        ])
        .start()
        .expect("MinIO doit démarrer — Docker est-il lancé ?");

    let port = minio
        .get_host_port_ipv4(9000.tcp())
        .expect("le port S3 de MinIO doit être publié");

    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "storage"]).assert().success();

    let output = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .env("RBS_STORAGE__BACKEND", "s3")
        .env("RBS_STORAGE__BUCKET", BUCKET)
        .env("RBS_STORAGE__ENDPOINT", format!("http://127.0.0.1:{port}"))
        .env("RBS_STORAGE__ACCESS_KEY_ID", CLE)
        .env("RBS_STORAGE__SECRET_ACCESS_KEY", SECRET)
        // MinIO veut le bucket dans le chemin, non dans le sous-domaine.
        .env("RBS_STORAGE__FORCE_PATH_STYLE", "true")
        .args(["test", "--workspace", "--", "--ignored"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "les tests du stockage ont échoué :\n{journal}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces deux lignes, un fragment qui cesserait de livrer ses tests S3 laisserait
    // celui-ci au vert sans que rien n'ait joint MinIO.
    for test in TESTS {
        assert!(
            journal.contains(&format!("test storage::tests::{test} ... ok")),
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
