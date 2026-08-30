//! La chaîne complète, du projet vide aux tests d'une feature CRUD qui passent.
//!
//! C8 prouve qu'un projet neuf compile ; celui-ci va jusqu'à la base : une feature
//! générée, sa migration appliquée, et les tests générés exécutés contre un vrai
//! PostgreSQL. Aucune étape n'est simulée — le binaire `rbs` est invoqué comme un
//! utilisateur l'invoquerait.

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{GenericImage, ImageExt};

mod common;

/// PostgreSQL **17** et non 18 : c'est ce qui prouve que l'exigence de la 18 est tombée
/// avec le défaut `uuidv7()`, désormais posé par le modèle.
const IMAGE: (&str, &str) = ("postgres", "17");

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
    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace"])
        .assert()
        .success()
        .get_output()
        .clone();

    let joues = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    // Le critère de l'identifiant v7 vit dans le projet, et il s'exige nommément : un
    // gabarit qui cesserait de livrer ce test laisserait celui-ci au vert, `cargo test`
    // sortant en 0 sur une suite amputée.
    assert!(
        joues.contains("test articles::tests::two_creations_in_a_row_carry_increasing_ids ... ok"),
        "le test des identifiants croissants n'a pas été joué :\n{joues}"
    );

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["doctor"])
        .assert()
        .success();
}

/// L'ordre d'application ne s'éprouve que contre une vraie base : un `cargo build` ne dit
/// rien d'une clé étrangère qui référencerait une table pas encore créée. `users` est
/// générée avant `posts`, comme l'inverse écrit dans son modèle l'exige — et c'est cet
/// ordre-là, celui des migrations, que ce test met à l'épreuve.
#[test]
#[ignore = "démarre PostgreSQL et compile la crate migration d'un projet Axum + SeaORM complet"]
fn a_relation_migrates_its_foreign_key_in_the_right_order() {
    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);

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
            "users",
            "--fields",
            "email:string:unique",
        ])
        .assert()
        .success();

    rbs(&projet)
        .args([
            "generate",
            "crud",
            "posts",
            "--fields",
            "title:string,author:references:users",
        ])
        .assert()
        .success();

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // Le nom de la contrainte est celui que le gabarit de migration lui donne,
    // déterministe (`fk_<table>_<colonne>`, et la colonne d'une référence est son nom
    // suffixé de `_id`) : sa seule présence en base prouve à la fois que la migration de
    // `posts` s'est appliquée et que celle de `users`, qu'elle référence, l'a précédée —
    // une base qui l'aurait refusée n'aurait laissé aucune contrainte à trouver.
    let mut resultat = postgres
        .exec(ExecCommand::new([
            "psql",
            "-U",
            common::UTILISATEUR,
            "-d",
            common::BASE,
            "-tAc",
            "select 1 from pg_constraint where conname = 'fk_posts_author_id'",
        ]))
        .expect("psql doit pouvoir s'exécuter dans le conteneur");
    let sortie = String::from_utf8(resultat.stdout_to_vec().expect("la sortie de psql se lit"))
        .expect("psql rend de l'utf-8");

    assert_eq!(
        sortie.trim(),
        "1",
        "la contrainte fk_posts_author est absente de la base :\n{sortie}"
    );
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
