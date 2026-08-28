//! La chaîne complète, du projet vide aux tests d'une feature CRUD qui passent.
//!
//! C8 prouve qu'un projet neuf compile ; celui-ci va jusqu'à la base : une feature
//! générée, sa migration appliquée, et les tests générés exécutés contre un vrai
//! PostgreSQL. Aucune étape n'est simulée — le binaire `rbs` est invoqué comme un
//! utilisateur l'invoquerait.

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{GenericImage, ImageExt};

mod common;

/// `uuidv7()`, que les migrations générées posent en défaut de clé primaire, n'existe
/// qu'à partir de PostgreSQL 18.
const IMAGE: (&str, &str) = ("postgres", "18");

const UTILISATEUR: &str = "rbs";
const MOT_DE_PASSE: &str = "rbs";
const BASE: &str = "demo";

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_generated_crud_migrates_and_passes_its_tests_against_postgresql() {
    // Le conteneur d'abord : son port détermine l'URL que le projet portera dans son
    // `.env`. Créer le projet avant obligerait à réécrire ce fichier après coup.
    let postgres = GenericImage::new(IMAGE.0, IMAGE.1)
        .with_wait_for(WaitFor::log(
            // PostgreSQL annonce une première fois qu'il accepte les connexions pendant
            // son initialisation, où il n'écoute que sur son socket local : attendre la
            // seconde annonce évite un test qui échoue une fois sur trois. Les deux flux
            // sont suivis ensemble, Docker ne les attribuant pas de la même façon.
            LogWaitStrategy::stdout_or_stderr("database system is ready to accept connections")
                .with_times(2),
        ))
        .with_env_var("POSTGRES_USER", UTILISATEUR)
        .with_env_var("POSTGRES_PASSWORD", MOT_DE_PASSE)
        .with_env_var("POSTGRES_DB", BASE)
        .start()
        .expect("PostgreSQL doit démarrer — Docker est-il lancé ?");

    let port = postgres
        .get_host_port_ipv4(5432.tcp())
        .expect("le port de PostgreSQL doit être publié");
    let url = format!("postgres://{UTILISATEUR}:{MOT_DE_PASSE}@127.0.0.1:{port}/{BASE}");

    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            &url,
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let projet = parent.path().join("demo-api");

    rbs(&projet)
        .args([
            "generate",
            "crud",
            "articles",
            "--fields",
            "titre:string,vues:int,publie:bool",
        ])
        .assert()
        .success();

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // Les tests générés montent l'application sur la base décrite par le `.env` : ils ne
    // passent que si la migration a bien été appliquée juste avant.
    Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace"])
        .assert()
        .success();

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["doctor"])
        .assert()
        .success();
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
