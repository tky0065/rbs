//! Ce que `rbs add auth` dépose dans un projet, éprouvé par la commande telle que
//! l'utilisateur la lance.
//!
//! Trois portées. Les tests de lecture d'ancres tournent sur chaque PR ; celui qui compile
//! le projet et celui qui migre une vraie base portent `#[ignore]`, comme `integration_new`
//! et `integration_crud` — ce sont eux qui prouvent que la feature installée tient debout.

use std::fs;
use std::io::{Read, Write};
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

/// `uuidv7()`, que les migrations générées posent en défaut de clé primaire, n'existe
/// qu'à partir de PostgreSQL 18.
const IMAGE: (&str, &str) = ("postgres", "18");

const UTILISATEUR: &str = "rbs";
const MOT_DE_PASSE: &str = "rbs";
const BASE: &str = "demo";

/// Le secret de signature que les tests posent dans le `.env` du projet.
///
/// `add auth` ne l'écrit que dans `.env.example` : le `.env` porte des secrets réels et
/// n'est pas versionné, rbs n'y dépose pas une valeur par défaut. Ces tests font donc ce
/// que fait l'utilisateur après l'installation.
///
/// Sans espace : une valeur d'un `.env` qui en porte se cite, faute de quoi dotenvy
/// refuse la ligne entière.
const SECRET: &str = "un-secret-de-test-de-plus-de-trente-deux-octets";

/// Un projet neuf, commité, prêt à recevoir une feature.
///
/// `add` refuse d'écrire dans un working tree sale : sans ce commit, la commande
/// s'arrête avant d'avoir rien fait.
fn projet_avec_auth(parent: &TempDir) -> PathBuf {
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args(["add", "auth"])
        .assert()
        .success();

    racine
}

/// Le contenu d'une ancre, balises exclues.
fn dans_l_ancre(racine: &Path, fichier: &str, ancre: &str) -> String {
    let source = fs::read_to_string(racine.join(fichier))
        .unwrap_or_else(|erreur| panic!("{fichier} illisible : {erreur}"));

    let ouverture = format!("// <rbs:{ancre}>");
    let fermeture = format!("// </rbs:{ancre}>");

    let debut = source
        .find(&ouverture)
        .unwrap_or_else(|| panic!("{fichier} ne porte pas `{ouverture}` :\n{source}"))
        + ouverture.len();
    let fin = source
        .find(&fermeture)
        .unwrap_or_else(|| panic!("{fichier} ne porte pas `{fermeture}` :\n{source}"));

    source[debut..fin].to_string()
}

/// Le critère du lot : l'installation complète les quatre ancres, et non deux.
#[test]
fn les_quatre_ancres_du_projet_sont_completees() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth(&parent);

    let attendu = [
        ("src/main.rs", "features", "mod auth;"),
        ("src/router.rs", "routes", ".merge(crate::auth::routes())"),
        (
            "src/openapi.rs",
            "openapi",
            "crate::auth::controller::login",
        ),
        ("migration/src/lib.rs", "migrations", "create_auth_tables"),
    ];

    for (fichier, ancre, ligne) in attendu {
        let contenu = dans_l_ancre(&racine, fichier, ancre);

        assert!(
            contenu.contains(ligne),
            "l'ancre `{ancre}` de {fichier} ne porte pas `{ligne}` :\n{contenu}"
        );
    }
}

/// Les cinq chemins sont montés dès l'installation : I7 les enregistrera dans le
/// document OpenAPI, J2 les jouera contre une vraie base.
#[test]
fn les_cinq_chemins_d_auth_sont_montes() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth(&parent);

    let module =
        fs::read_to_string(racine.join("src/auth/mod.rs")).expect("src/auth/mod.rs lisible");

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/auth/me",
    ] {
        assert!(
            module.contains(chemin),
            "`{chemin}` n'est pas monté :\n{module}"
        );
    }
}

/// Le secret et les durées de vie arrivent avec la feature, sous les noms qu'`AuthConfig`
/// attend : un projet qui les nomme autrement échoue au démarrage, pas à la compilation.
#[test]
fn la_configuration_et_l_environnement_recoivent_ce_qu_auth_exige() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth(&parent);

    let config = fs::read_to_string(racine.join("config/default.toml")).expect("config lisible");
    assert!(config.contains("[auth]"), "section absente :\n{config}");
    assert!(
        config.contains("access_ttl_secs") && config.contains("refresh_ttl_secs"),
        "durées de vie absentes :\n{config}"
    );

    let env = fs::read_to_string(racine.join(".env.example")).expect(".env.example lisible");
    assert!(env.contains("RBS_AUTH__SECRET"), "secret absent :\n{env}");

    let manifeste = fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml lisible");
    assert!(
        manifeste.contains("features = [\"auth\"]"),
        "le flag `auth` de rbs-core n'est pas activé :\n{manifeste}"
    );
}

/// Le critère exécutable du lot, pris au niveau qu'exige la CI générée.
///
/// `--all-targets` et non `check` seul : sans lui, `src/auth/tests.rs` n'est jamais
/// compilé. Et `clippy -D warnings` plutôt que `check`, parce que c'est la commande que
/// le workflow d'`rbs add ci` lance : un fragment qui laisse un warning derrière lui
/// rendrait rouge, dès le premier push, une CI portant du code que l'utilisateur n'a pas
/// écrit.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_projet_portant_auth_compile_sans_warning_et_est_formate() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth(&parent);

    Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .assert()
        .success();

    // `rustfmt` sur les deux racines de modules, et non `cargo fmt` : lancé sous
    // `cargo test`, celui-ci retrouve le workspace de rbs lui-même et signale ses
    // fichiers — le test échouait alors sur du code sans rapport avec le projet généré,
    // et `--manifest-path` n'y change rien. rustfmt suit les déclarations de modules :
    // `main.rs` emporte tout `src/auth/`, y compris son `mod tests`.
    for racine_de_modules in ["src/main.rs", "migration/src/lib.rs"] {
        Command::new("rustfmt")
            .args(["--edition", "2024", "--check"])
            .arg(racine.join(racine_de_modules))
            .assert()
            .success();
    }
}

/// Le schéma que la feature installe, éprouvé contre un vrai PostgreSQL.
///
/// Les deux critères du lot y tiennent : la migration s'applique et se défait sans rien
/// laisser derrière elle, et les deux garanties d'intégrité que l'auth exige — un email
/// unique, un `token_hash` indexé — sont bien dans la base et non seulement dans le
/// fichier de migration.
#[test]
#[ignore = "démarre PostgreSQL et compile la crate de migration : plusieurs minutes"]
fn la_migration_d_auth_cree_le_schema_puis_le_rend_a_son_etat_initial() {
    let postgres = demarrer_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth_sur(&url_de(&postgres), &parent);

    let avant = tables(&postgres);

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    let apres = tables(&postgres);
    for table in ["users", "refresh_tokens"] {
        assert!(
            apres.contains(&table.to_string()),
            "`{table}` absente après la migration : {apres:?}"
        );
    }

    assert_eq!(
        psql(
            &postgres,
            "SELECT count(*) FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage k \
               ON tc.constraint_name = k.constraint_name \
             WHERE tc.table_name = 'users' \
               AND tc.constraint_type = 'UNIQUE' \
               AND k.column_name = 'email'"
        ),
        "1",
        "`email` n'est pas unique"
    );

    assert_eq!(
        psql(
            &postgres,
            "SELECT count(*) FROM pg_indexes \
             WHERE tablename = 'refresh_tokens' AND indexdef LIKE '%token_hash%'"
        ),
        "1",
        "`token_hash` n'est pas indexé"
    );

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "down"])
        .assert()
        .success();

    // `seaql_migrations` est créée par le migrateur lui-même et lui survit : la
    // comparaison porte sur les tables de la feature, que `down` doit avoir emportées.
    let rendu = tables(&postgres);
    for table in ["users", "refresh_tokens"] {
        assert!(
            !rendu.contains(&table.to_string()),
            "`{table}` survit à `migrate down` : {rendu:?}"
        );
    }
    assert!(
        rendu.len() <= avant.len() + 1,
        "`down` a laissé des tables derrière lui : {avant:?} puis {rendu:?}"
    );
}

/// Les quatre critères du lot, joués par les tests que l'utilisateur reçoit.
///
/// Ce que `rbs add auth` dépose dans `src/auth/tests.rs` est ce qui prouve la feature :
/// un test qui passerait ici sans passer chez l'utilisateur ne prouverait rien de ce
/// qu'il reçoit.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn les_tests_d_auth_du_projet_genere_passent() {
    let postgres = demarrer_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth_sur(&url_de(&postgres), &parent);

    migrer(&racine);

    Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace"])
        .assert()
        .success();
}

/// Le mot de passe traverse le serveur, son hash y est calculé, et le journal n'en garde
/// ni l'un ni l'autre.
///
/// La vérification porte sur la sortie réelle du binaire, à `RUST_LOG=debug` : c'est le
/// niveau auquel se voit une trace de mise au point laissée derrière soi, et cette sortie
/// couvre aussi les middlewares du noyau, qu'une capture posée dans le projet manquerait.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_hash_n_apparait_pas_dans_les_logs_du_serveur() {
    const MOT_DE_PASSE_EN_CLAIR: &str = "un mot de passe assez long";

    let postgres = demarrer_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_avec_auth_sur(&url_de(&postgres), &parent);

    migrer(&racine);

    Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
        .assert()
        .success();

    let port = port_libre();
    let mut serveur = std::process::Command::new(common::cible().join("debug/demo-api"))
        .current_dir(&racine)
        .env("RBS_SERVER__PORT", port.to_string())
        .env("RUST_LOG", "debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire du projet doit être lançable");

    attendre_ecoute(port);

    let reponse = poster_json(
        port,
        "/auth/register",
        &format!(r#"{{"email":"journal@exemple.test","password":"{MOT_DE_PASSE_EN_CLAIR}"}}"#),
    );

    serveur.kill().expect("le serveur doit pouvoir être arrêté");
    let sortie = serveur
        .wait_with_output()
        .expect("la sortie du serveur doit être lisible");
    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        reponse.starts_with("HTTP/1.1 201"),
        "l'inscription doit aboutir, sans quoi aucun hash n'a été calculé :\n{reponse}\n{journal}"
    );
    // Sans cette ligne, un journal vide — serveur muet, capture manquée — ferait passer
    // les deux recherches qui suivent sans rien prouver.
    assert!(
        journal.contains("/auth/register"),
        "le journal capturé ne porte pas la requête : y chercher le hash ne prouve rien :\n{journal}"
    );
    assert!(
        !journal.contains("$argon2"),
        "le hash est journalisé :\n{journal}"
    );
    assert!(
        !journal.contains(MOT_DE_PASSE_EN_CLAIR),
        "le mot de passe est journalisé :\n{journal}"
    );
}

/// Un PostgreSQL neuf, prêt à recevoir le schéma d'un projet généré.
fn demarrer_postgres() -> Container<GenericImage> {
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
fn url_de(postgres: &Container<GenericImage>) -> String {
    let port = postgres
        .get_host_port_ipv4(5432.tcp())
        .expect("le port de PostgreSQL doit être publié");

    format!("postgres://{UTILISATEUR}:{MOT_DE_PASSE}@127.0.0.1:{port}/{BASE}")
}

/// Un projet neuf portant `auth`, sa base pointée sur `url` et son secret posé.
///
/// Les migrations ne sont pas appliquées : le test du schéma a besoin de l'état d'avant.
fn projet_avec_auth_sur(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
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

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args(["add", "auth"])
        .assert()
        .success();

    let env = racine.join(".env");
    let mut contenu = fs::read_to_string(&env).expect(".env lisible");
    contenu.push_str(&format!("\nRBS_AUTH__SECRET={SECRET}\n"));
    fs::write(&env, contenu).expect(".env inscriptible");

    racine
}

/// Applique les migrations du projet.
fn migrer(racine: &Path) {
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();
}

/// Un port que personne n'écoute au moment de l'appel.
fn port_libre() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("l'hôte doit pouvoir prêter un port")
        .local_addr()
        .expect("adresse locale lisible")
        .port()
}

/// Attend que le serveur accepte les connexions sur `port`.
fn attendre_ecoute(port: u16) {
    let limite = Instant::now() + Duration::from_secs(60);

    while Instant::now() < limite {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    panic!("le serveur n'écoute toujours pas sur {port} après 60 s");
}

/// Poste `corps` en JSON sur `chemin` et rend la réponse HTTP brute.
///
/// La requête est écrite à la main plutôt que par un client HTTP : ce test n'a besoin que
/// d'une ligne de statut, et la dépendance se paierait sur toute la CI.
fn poster_json(port: u16, chemin: &str, corps: &str) -> String {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("le serveur doit répondre");

    let requete = format!(
        "POST {chemin} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{corps}",
        corps.len()
    );

    flux.write_all(requete.as_bytes())
        .expect("la requête doit partir");

    let mut reponse = String::new();
    flux.read_to_string(&mut reponse)
        .expect("la réponse doit être lisible");

    reponse
}

/// Les tables du schéma public, triées.
fn tables(postgres: &testcontainers::Container<GenericImage>) -> Vec<String> {
    psql(
        postgres,
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' ORDER BY 1",
    )
    .lines()
    .map(str::to_string)
    .collect()
}

/// Joue `requete` dans le conteneur et rend sa sortie, sans en-tête ni alignement.
fn psql(postgres: &testcontainers::Container<GenericImage>, requete: &str) -> String {
    let mut resultat = postgres
        .exec(ExecCommand::new([
            "psql",
            "-U",
            UTILISATEUR,
            "-d",
            BASE,
            "-tAc",
            requete,
        ]))
        .expect("psql doit être lançable dans le conteneur");

    let sortie = resultat.stdout_to_vec().expect("sortie de psql lisible");

    String::from_utf8_lossy(&sortie).trim().to_string()
}
