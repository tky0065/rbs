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
use testcontainers::core::ExecCommand;
use testcontainers::{Container, GenericImage};

mod common;

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
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_jobs(&common::url_of(&postgres), &parent);

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
            sous_conteneur.contains(&format!("test jobs::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Le critère de la portabilité : le dépilage tient sur les trois moteurs.
///
/// Le test de concurrence vit dans le projet engendré — c'est lui qui compte, et il est
/// exigé nommément : `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun**
/// test, et un fragment qui cesserait de livrer ses tests laisserait celui-ci au vert.
///
/// Une cible de compilation par moteur : les trois activent des features `sea-orm`
/// différentes, et une cible commune ferait recompiler `sea-orm` et `sqlx` à chaque
/// bascule — y compris pour les tests qui n'ont rien demandé.
#[test]
#[ignore = "démarre PostgreSQL et MySQL et compile un projet par moteur : plusieurs minutes"]
fn the_dequeue_never_hands_the_same_job_twice_on_the_three_engines() {
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
        let racine = project_with_jobs_on(moteur, &url, parent);

        migrate_dans(&racine, &cible);

        let (abouti, joues) = cargo_test_brut(&racine, &cible, &["--", "--ignored"]);
        assert!(
            abouti,
            "les tests du projet ont échoué sur {moteur} :\n{joues}"
        );
        assert!(
            joues.contains(
                "test jobs::tests::two_concurrent_workers_never_reserve_the_same_job ... ok"
            ),
            "le test de concurrence n'a pas été joué sur {moteur} :\n{joues}"
        );

        // `doctor` interroge la base pour sa version : une requête écrite pour PostgreSQL
        // le ferait échouer ici, et nulle part ailleurs.
        let diagnostic = rbs(&racine)
            .env("CARGO_TARGET_DIR", &cible)
            .arg("doctor")
            .assert()
            .success()
            .get_output()
            .clone();

        let rendu = String::from_utf8_lossy(&diagnostic.stdout).into_owned();

        // `rbs doctor` sort en 0 même quand un contrôle échoue : c'est la ligne rendue
        // qui tranche, non le code de sortie. Le contrôle `base` annonce d'abord la
        // compilation de la crate migration, sous son titre et sans verdict : c'est le
        // marqueur qui distingue cette annonce du constat.
        let ligne = rendu
            .lines()
            .find(|ligne| ligne.contains("base") && ligne.contains(['✓', '!', '✗']))
            .unwrap_or_else(|| panic!("`doctor` ne rend aucun constat « base » :\n{rendu}"));

        assert!(
            ligne.contains('✓'),
            "`doctor` refuse la base sur {moteur} : {ligne}"
        );
        assert!(
            moteur == "postgres" || !rendu.contains("PostgreSQL"),
            "`doctor` nomme encore PostgreSQL sur {moteur} :\n{rendu}"
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
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_jobs(&common::url_of(&postgres), &parent);

    // Tenu jusqu'à la fin du test, et non le temps des seuls cargo : les deux serveurs
    // lancés plus bas exécutent le binaire bâti dans cette cible, qu'un autre projet
    // remplacerait sous leurs pieds.
    let _cible = common::verrou(&common::cible());

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

/// Un projet neuf portant `jobs`, sa base pointée sur `url`.
fn project_with_jobs(url: &str, parent: &TempDir) -> PathBuf {
    project_with_jobs_on("postgres", url, parent)
}

/// Le même, sur le moteur demandé.
fn project_with_jobs_on(moteur: &str, url: &str, parent: &TempDir) -> PathBuf {
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

    rbs(&racine).args(["add", "jobs"]).assert().success();

    racine
}

fn migrate(racine: &Path) {
    migrate_dans(racine, &common::cible());
}

fn migrate_dans(racine: &Path, cible: &Path) {
    rbs(racine)
        .env("CARGO_TARGET_DIR", cible)
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
    cargo_test_dans(racine, &common::cible(), arguments)
}

fn cargo_test_dans(racine: &Path, cible: &Path, arguments: &[&str]) -> String {
    let (abouti, journal) = cargo_test_brut(racine, cible, arguments);
    assert!(abouti, "les tests du projet ont échoué :\n{journal}");

    journal
}

/// Le même, rendant l'issue plutôt que de trancher : l'appelant sait quel moteur il joue.
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

/// Enfile un job directement dans la table, sans passer par un processus du projet.
///
/// L'`INSERT` est écrit ici plutôt que joué par un binaire du projet pour que rien de ce
/// qui enfile ne survive à l'enfilage : le seul processus rbs de ce test est le serveur,
/// et il est tué avant que le job ne s'exécute.
/// Enfile en SQL brut, identifiant compris.
///
/// La colonne ne porte plus de défaut : c'est le modèle qui pose l'identifiant, et un
/// insert qui contourne le modèle doit donc le fournir. La valeur est fixe et de forme
/// v7 — un seul job traverse ce test.
fn enqueue(postgres: &Container<GenericImage>) {
    psql(
        postgres,
        &format!(
            "INSERT INTO jobs (id, kind, payload) \
             VALUES ('0199c0de-0000-7000-8000-000000000001', 'log', \
             '{{\"message\":\"{MESSAGE}\"}}'::json)"
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
            common::UTILISATEUR,
            "-d",
            common::BASE,
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
