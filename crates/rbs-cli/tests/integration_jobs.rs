//! La file de jobs d'un projet réel, jouée contre un PostgreSQL en conteneur.
//!
//! Deux choses s'y prouvent que rien d'autre ne prouve. Que les tests livrés au projet
//! tournent, d'abord : ils portent l'atomicité avec le métier et la concurrence entre
//! workers, et une file qui n'est pas jouée contre une vraie base ne dit rien de l'une ni
//! de l'autre. Que le job **survit à la mort du processus**, ensuite — c'est la seule
//! chose qui distingue cette file d'un `tokio::spawn` détaché, et elle se joue plutôt
//! qu'elle ne s'affirme.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, GenericImage, ImageExt};

mod common;

/// PostgreSQL **17** et non 18 : c'est ce qui prouve que l'exigence de la 18 est tombée
/// avec le défaut `uuidv7()`, désormais posé par le modèle.
const IMAGE: (&str, &str) = ("postgres", "17");

const UTILISATEUR: &str = "rbs";
const MOT_DE_PASSE: &str = "rbs";
const BASE: &str = "demo";

/// Les tests que le fragment livre au projet et qui joignent la base.
const TESTS: [&str; 4] = [
    "a_job_enqueued_in_a_rolled_back_transaction_does_not_exist",
    "a_job_enqueued_in_a_committed_transaction_is_visible_to_the_worker",
    "two_concurrent_workers_never_reserve_the_same_job",
    "a_failing_job_is_retried_then_marked_failed_after_the_last_attempt",
];

/// Le message du job d'exemple, par lequel la ligne se retrouve dans la table.
const MESSAGE: &str = "survivant";

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_jobs(&url_of(&postgres), &parent);

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
            sous_conteneur.contains(&format!("test jobs::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Le critère du jalon : un job enfilé pendant qu'un processus tourne s'exécute après que
/// ce processus a été tué et relancé.
///
/// L'intervalle de scrutation sert de pince : très long pour le premier processus, qui ne
/// verra donc jamais le job de son vivant ; d'une seconde pour le second, qui le dépile.
/// Sans lui, le premier processus exécuterait le job aussitôt et le test ne prouverait
/// rien de sa survie.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_job_enqueued_before_the_process_is_killed_runs_after_the_restart() {
    let postgres = start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_jobs(&url_of(&postgres), &parent);

    migrate(&racine);
    compile(&racine);

    let premier = Serveur::lancer(&racine, "demo-api", 3600);
    enqueue(&postgres);
    assert_eq!(
        status(&postgres),
        "pending",
        "le job a été dépilé avant d'avoir pu être laissé en attente"
    );

    let journal = premier.tuer();
    assert_eq!(
        status(&postgres),
        "pending",
        "le premier processus a exécuté le job — le test ne prouve plus sa survie :\n{journal}"
    );

    let second = Serveur::lancer(&racine, "demo-api", 1);
    let atteint = wait_for_status(&postgres, "done", Duration::from_secs(60));
    let journal = second.tuer();

    assert!(
        atteint,
        "le job n'a pas survécu au redémarrage — statut « {} » :\n{journal}",
        status(&postgres)
    );
    assert_eq!(
        attempts(&postgres),
        "1",
        "le job a été tenté plus d'une fois :\n{journal}"
    );
}

/// Un PostgreSQL neuf, prêt à recevoir le schéma d'un projet généré.
fn start_postgres() -> Container<GenericImage> {
    GenericImage::new(IMAGE.0, IMAGE.1)
        .with_wait_for(WaitFor::log(
            // PostgreSQL annonce une première fois qu'il accepte les connexions pendant
            // son initialisation, où il n'écoute que sur son socket local : attendre la
            // seconde annonce évite un test qui échoue une fois sur trois.
            LogWaitStrategy::stdout_or_stderr("database system is ready to accept connections")
                .with_times(2),
        ))
        .with_env_var("POSTGRES_USER", UTILISATEUR)
        .with_env_var("POSTGRES_PASSWORD", MOT_DE_PASSE)
        .with_env_var("POSTGRES_DB", BASE)
        .start()
        .expect("PostgreSQL doit démarrer — Docker est-il lancé ?")
}

/// L'URL de connexion à `postgres`, vue depuis l'hôte.
fn url_of(postgres: &Container<GenericImage>) -> String {
    let port = postgres
        .get_host_port_ipv4(5432.tcp())
        .expect("le port de PostgreSQL doit être publié");

    format!("postgres://{UTILISATEUR}:{MOT_DE_PASSE}@127.0.0.1:{port}/{BASE}")
}

/// Un projet neuf portant `jobs`, sa base pointée sur `url`.
fn project_with_jobs(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
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

    rbs(&racine).args(["add", "jobs"]).assert().success();

    racine
}

fn migrate(racine: &Path) {
    rbs(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();
}

/// Compile le projet, binaire compris.
fn compile(racine: &Path) {
    Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
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

/// Enfile un job directement dans la table, sans passer par un processus du projet.
///
/// L'`INSERT` est écrit ici plutôt que joué par un binaire du projet pour que rien de ce
/// qui enfile ne survive à l'enfilage : le seul processus rbs de ce test est le serveur,
/// et il est tué avant que le job ne s'exécute.
fn enqueue(postgres: &Container<GenericImage>) {
    psql(
        postgres,
        &format!(
            "INSERT INTO jobs (kind, payload) \
             VALUES ('log', '{{\"message\":\"{MESSAGE}\"}}'::json)"
        ),
    );
}

fn status(postgres: &Container<GenericImage>) -> String {
    psql(postgres, "SELECT status FROM jobs")
}

fn attempts(postgres: &Container<GenericImage>) -> String {
    psql(postgres, "SELECT attempts FROM jobs")
}

/// Attend que le job atteigne `attendu`, et dit s'il y est parvenu.
fn wait_for_status(postgres: &Container<GenericImage>, attendu: &str, limite: Duration) -> bool {
    let fin = Instant::now() + limite;

    while Instant::now() < fin {
        if status(postgres) == attendu {
            return true;
        }

        std::thread::sleep(Duration::from_millis(200));
    }

    false
}

/// Joue `request` dans le conteneur et rend sa sortie, sans en-tête ni alignement.
fn psql(postgres: &Container<GenericImage>, request: &str) -> String {
    let mut resultat = postgres
        .exec(ExecCommand::new([
            "psql",
            "-U",
            UTILISATEUR,
            "-d",
            BASE,
            "-tAc",
            request,
        ]))
        .expect("psql doit être lançable dans le conteneur");

    let output = resultat.stdout_to_vec().expect("sortie de psql lisible");

    String::from_utf8_lossy(&output).trim().to_string()
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Un port que personne n'écoute au moment de l'appel.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("l'hôte doit pouvoir prêter un port")
        .local_addr()
        .expect("adresse locale lisible")
        .port()
}

/// Attend que le serveur accepte les connexions sur `port`.
fn wait_for_listening(port: u16) {
    let limite = Instant::now() + Duration::from_secs(60);

    while Instant::now() < limite {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    panic!("le serveur n'écoute toujours pas sur {port} après 60 s");
}

/// Le binaire du projet, worker compris, arrêté quand ce garde tombe.
///
/// `Drop` plutôt qu'un `kill` en fin de test : une assertion qui échoue au milieu du
/// parcours déroule la pile sans jamais l'atteindre, et laisse derrière elle un serveur
/// qui écoute et un conteneur qu'il tient ouvert.
struct Serveur {
    processus: Option<std::process::Child>,
}

impl Serveur {
    /// Lance le binaire, l'intervalle de scrutation du worker fixé à `scrutation`.
    fn lancer(racine: &Path, binaire: &str, scrutation: u64) -> Self {
        let port = free_port();

        let processus = std::process::Command::new(common::cible().join("debug").join(binaire))
            .current_dir(racine)
            .env("RBS_SERVER__PORT", port.to_string())
            .env("RBS_JOBS__POLL_INTERVAL_SECS", scrutation.to_string())
            .env("RUST_LOG", "info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("le binaire du projet doit être lançable");

        wait_for_listening(port);

        // Le worker est détaché avant que le serveur ne se lie, mais rien ne garantit que
        // le runtime l'ait déjà fait tourner : sans cette pause, un job enfilé juste après
        // pourrait tomber dans son premier passage plutôt que dans son attente.
        std::thread::sleep(Duration::from_millis(500));

        Self {
            processus: Some(processus),
        }
    }

    /// Tue le serveur et rend ce qu'il a écrit sur ses deux flux.
    fn tuer(mut self) -> String {
        let mut processus = self.processus.take().expect("le serveur tourne encore");

        processus
            .kill()
            .expect("le serveur doit pouvoir être arrêté");

        let output = processus
            .wait_with_output()
            .expect("la sortie du serveur doit être lisible");

        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        if let Some(processus) = self.processus.as_mut() {
            let _ = processus.kill();
            let _ = processus.wait();
        }
    }
}

/// Le fragment ne dépose aucun fichier que le projet ne monterait pas.
///
/// Ce test-ci ne demande ni Docker ni compilation : il tourne sur chaque PR, là où les
/// deux autres attendent qu'on les réclame.
#[test]
fn every_file_the_fragment_ships_is_declared_in_its_manifest() {
    let racine = common::depot().join("crates/rbs-cli/templates/features/jobs");
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
