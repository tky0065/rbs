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
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, GenericImage, ImageExt};

mod common;

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

/// Le mot de passe des comptes ouverts par les parcours.
///
/// Huit caractères au moins : le DTO d'inscription le valide, et un mot de passe
/// trop court ferait rendre 422 là où le parcours attend 201.
const MOT_DE_PASSE_DU_COMPTE: &str = "un mot de passe assez long";

/// Un projet neuf, commité, prêt à recevoir une feature.
///
/// `add` refuse d'écrire dans un working tree sale : sans ce commit, la commande
/// s'arrête avant d'avoir rien fait.
fn project_with_auth(parent: &TempDir) -> PathBuf {
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
fn in_the_anchor(racine: &Path, fichier: &str, anchor: &str) -> String {
    let source = fs::read_to_string(racine.join(fichier))
        .unwrap_or_else(|erreur| panic!("{fichier} illisible : {erreur}"));

    let ouverture = format!("// <rbs:{anchor}>");
    let fermeture = format!("// </rbs:{anchor}>");

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
fn the_four_project_anchors_are_completed() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    let attendu = [
        ("src/lib.rs", "features", "pub mod auth;"),
        ("src/router.rs", "routes", ".merge(crate::auth::routes())"),
        (
            "src/openapi.rs",
            "openapi",
            "crate::auth::controller::login",
        ),
        ("migration/src/lib.rs", "migrations", "create_auth_tables"),
    ];

    for (fichier, anchor, ligne) in attendu {
        let contenu = in_the_anchor(&racine, fichier, anchor);

        assert!(
            contenu.contains(ligne),
            "l'ancre `{anchor}` de {fichier} ne porte pas `{ligne}` :\n{contenu}"
        );
    }
}

/// Les cinq chemins sont montés dès l'installation : I7 les enregistrera dans le
/// document OpenAPI, J2 les jouera contre une vraie base.
#[test]
fn the_five_auth_paths_are_mounted() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

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
fn the_configuration_and_the_environment_receive_what_auth_requires() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    let config = fs::read_to_string(racine.join("config/default.toml")).expect("config lisible");
    assert!(config.contains("[auth]"), "section absente :\n{config}");
    assert!(
        config.contains("access_ttl_secs") && config.contains("refresh_ttl_secs"),
        "durées de vie absentes :\n{config}"
    );

    let env = fs::read_to_string(racine.join(".env.example")).expect(".env.example lisible");
    assert!(env.contains("RBS_AUTH__SECRET"), "secret absent :\n{env}");

    // `auth` s'ajoute au pilote que la création a choisi : la liste porte les deux, et
    // écraser l'une par l'autre ferait échouer la compilation du projet.
    let manifeste = fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml lisible");
    let noyau = manifeste
        .lines()
        .find(|ligne| ligne.starts_with("rbs-core = "))
        .unwrap_or_else(|| panic!("`rbs-core` absente du manifeste :\n{manifeste}"));

    assert!(
        noyau.contains("\"auth\""),
        "le flag `auth` de rbs-core n'est pas activé : {noyau}"
    );
    assert!(
        noyau.contains("\"postgres\""),
        "le pilote a été écrasé par la feature : {noyau}"
    );
}

/// Sérialise les tests qui compilent puis exécutent un binaire du projet.
///
/// Tous partagent `CARGO_TARGET_DIR` : la crate `migration` de chaque projet s'écrit au
/// même `debug/migration`, quand bien même leurs contenus diffèrent, et deux projets de
/// même nom partagent aussi leur binaire. Or `cargo run` relâche son verrou avant
/// d'exécuter — un test lance donc un fichier qu'un autre est en train de remplacer.
///
/// Perdre cette course se voit : cargo répond `No such file or directory`. La gagner ne
/// se voit pas — le test exécute alors les migrations d'un autre projet et passe quand
/// même. C'est ce second cas qui justifie le verrou.
///
/// Une cible propre à chaque test coûterait plus cher : la cible partagée existe pour ne
/// pas recompiler six fois l'arborescence de dépendances.
static CIBLE_PARTAGEE: Mutex<()> = Mutex::new(());

/// Prend la cible de compilation pour soi, jusqu'à la fin du test.
///
/// Le verrou se prend après le conteneur : les PostgreSQL n'ont rien à partager et
/// démarrent en parallèle.
///
/// Deux verrous, parce que la course a deux portées : le `Mutex` tranche entre les tests
/// de ce fichier sans toucher au système de fichiers, celui de la cible entre ce binaire
/// et les autres de `tests/`, que `cargo test` lance de front.
fn own_target() -> (MutexGuard<'static, ()>, std::fs::File) {
    // Un test qui panique empoisonne le verrou. Sans cette reprise, les suivants
    // échoueraient tous sur un message qui ne dit rien de leur propre défaut.
    let exclusivite = CIBLE_PARTAGEE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    (exclusivite, common::verrou(&common::cible()))
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
fn the_project_carrying_auth_compiles_without_a_warning_and_is_formatted() {
    let _cible = own_target();

    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

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
    // `lib.rs` emporte tout `src/auth/`, y compris son `mod tests`.
    for racine_de_modules in ["src/lib.rs", "migration/src/lib.rs"] {
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
fn the_auth_migration_creates_the_schema_then_returns_it_to_its_initial_state() {
    let postgres = start_postgres();
    let _cible = own_target();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth_on(&url_of(&postgres), &parent);

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
fn the_auth_tests_of_the_generated_project_pass() {
    let postgres = start_postgres();
    let _cible = own_target();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth_on(&url_of(&postgres), &parent);

    migrate(&racine);

    // `--include-ignored` : les tests du fragment joignent la base et sont `#[ignore]`.
    // Sans lui, `cargo test` sortait en 0 sans en jouer un seul, et ce test passait au
    // vert sans rien prouver de ce que reçoit l'utilisateur.
    let sortie = Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace", "--", "--include-ignored"])
        .output()
        .expect("cargo doit être lançable");

    let rendu = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        sortie.status.success(),
        "la suite du projet engendré échoue :\n{rendu}"
    );
    assert!(
        rendu.contains("auth::tests::") && rendu.contains(" ... ok"),
        "aucun test du fragment auth n'a tourné :\n{rendu}"
    );
}

/// Le mot de passe traverse le serveur, son hash y est calculé, et le journal n'en garde
/// ni l'un ni l'autre.
///
/// La vérification porte sur la sortie réelle du binaire, à `RUST_LOG=debug` : c'est le
/// niveau auquel se voit une trace de mise au point laissée derrière soi, et cette sortie
/// couvre aussi les middlewares du noyau, qu'une capture posée dans le projet manquerait.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_hash_does_not_appear_in_the_server_logs() {
    const MOT_DE_PASSE_EN_CLAIR: &str = "un mot de passe assez long";

    let postgres = start_postgres();
    let _cible = own_target();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth_on(&url_of(&postgres), &parent);

    migrate(&racine);

    compile(&racine);

    let serveur = Serveur::lancer(&racine, "demo-api", "debug");
    let port = serveur.port();

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&format!(
            r#"{{"email":"journal@example.test","password":"{MOT_DE_PASSE_EN_CLAIR}"}}"#
        )),
    );

    let journal = serveur.journal();

    assert_eq!(
        statut, 201,
        "l'inscription doit aboutir, sans quoi aucun hash n'a été calculé :\n{corps}\n{journal}"
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

/// Le critère du lot : les huit étapes du parcours, un seul compte, contre le binaire
/// réellement lancé.
///
/// Les tests d'auth que le projet reçoit montent le `Router` en mémoire et éprouvent
/// chaque garantie isolément. Ce qu'ils ne peuvent pas montrer, c'est que les états
/// s'enchaînent : que le jeton émis par `login` est celui que la garde accepte, que la
/// paire rendue par `refresh` est utilisable, que le `logout` porte sur la ligne que
/// `refresh` venait d'ouvrir.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_auth_journey_plays_end_to_end() {
    const EMAIL: &str = "parcours@exemple.test";

    let postgres = start_postgres();
    let _cible = own_target();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth_on(&url_of(&postgres), &parent);

    migrate(&racine);
    compile(&racine);

    let serveur = Serveur::lancer(&racine, "demo-api", "info");
    let port = serveur.port();

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&credentials(EMAIL)),
    );
    assert_eq!(statut, 201, "l'inscription doit aboutir : {corps}");

    let (statut, premiere) = request(port, "POST", "/auth/login", None, Some(&credentials(EMAIL)));
    assert_eq!(
        statut, 200,
        "la connexion doit rendre une paire : {premiere}"
    );

    let (statut, corps) = request(port, "GET", "/auth/me", None, None);
    assert_eq!(
        statut, 401,
        "une route protégée doit refuser une requête sans jeton : {corps}"
    );

    let (statut, corps) = request(port, "GET", "/auth/me", Some(&access(&premiere)), None);
    assert_eq!(
        statut, 200,
        "le jeton émis par `login` doit être celui que la garde accepte : {corps}"
    );

    let (statut, seconde) = request(
        port,
        "POST",
        "/auth/refresh",
        None,
        Some(&renewal(&premiere)),
    );
    assert_eq!(
        statut, 200,
        "le rafraîchissement doit rendre une paire : {seconde}"
    );

    let (statut, corps) = request(port, "GET", "/auth/me", Some(&access(&seconde)), None);
    assert_eq!(
        statut, 200,
        "la paire rendue par `refresh` doit être utilisable : {corps}"
    );

    let (statut, corps) = request(port, "POST", "/auth/logout", None, Some(&renewal(&seconde)));
    assert_eq!(statut, 204, "la déconnexion doit aboutir : {corps}");

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/refresh",
        None,
        Some(&renewal(&seconde)),
    );
    assert_eq!(statut, 401, "le refresh révoqué doit être refusé : {corps}");

    // Le rejeu se joue sur un second compte de sessions, et en dernier : il emporte
    // désormais les sessions sœurs, et le tester plus tôt tuerait celles que les étapes
    // suivantes utilisent.
    let (statut, troisieme) = request(port, "POST", "/auth/login", None, Some(&credentials(EMAIL)));
    assert_eq!(
        statut, 200,
        "la reconnexion doit rendre une paire : {troisieme}"
    );

    let (statut, quatrieme) = request(
        port,
        "POST",
        "/auth/refresh",
        None,
        Some(&renewal(&troisieme)),
    );
    assert_eq!(
        statut, 200,
        "le rafraîchissement doit rendre une paire : {quatrieme}"
    );

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/refresh",
        None,
        Some(&renewal(&troisieme)),
    );
    assert_eq!(
        statut, 401,
        "le refresh déjà consommé doit être refusé : {corps}"
    );

    // Ce que le rejeu vient de déclencher : un jeton volé et rejoué ferme toute la
    // famille, faute de quoi l'attaquant garderait une paire valide indéfiniment.
    let (statut, corps) = request(
        port,
        "POST",
        "/auth/refresh",
        None,
        Some(&renewal(&quatrieme)),
    );
    assert_eq!(
        statut, 401,
        "un rejeu détecté doit emporter les sessions sœurs : {corps}"
    );
}

/// Le corps d'inscription et de connexion d'un compte.
fn credentials(email: &str) -> String {
    format!(r#"{{"email":"{email}","password":"{MOT_DE_PASSE_DU_COMPTE}"}}"#)
}

/// Le corps qu'attendent `refresh` et `logout`.
fn renewal(paire: &Value) -> String {
    format!(r#"{{"refresh_token":"{}"}}"#, field(paire, "refresh_token"))
}

/// Le jeton d'accès d'une paire.
fn access(paire: &Value) -> String {
    field(paire, "access_token")
}

fn field(paire: &Value, name: &str) -> String {
    paire[name]
        .as_str()
        .unwrap_or_else(|| panic!("la paire doit porter `{name}` : {paire}"))
        .to_owned()
}

/// L'étape du parcours que le projet généré ne permet pas de jouer : `require_role`
/// devant un binaire réel.
///
/// `add auth` livre la garde mais aucune route qui la porte — la seule du projet généré
/// est montée dans son module de tests, et ne répond donc jamais sur le réseau.
/// `examples/blog-auth` en porte une dans son binaire, dont c'est ici la première
/// exécution : la CI n'en prouve que la compilation.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_guarded_route_rejects_an_authenticated_user() {
    const EMAIL: &str = "sans-droits@exemple.test";
    const ARTICLE: &str = r#"{"title":"Un title","body":"Un corps.","published":true}"#;

    let postgres = start_postgres();
    let _cible = own_target();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = blog_auth_on(&url_of(&postgres), &parent);

    migrate(&racine);
    compile(&racine);

    let serveur = Serveur::lancer(&racine, "blog-auth", "info");
    let port = serveur.port();

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&credentials(EMAIL)),
    );
    assert_eq!(statut, 201, "l'inscription doit aboutir : {corps}");

    let (statut, paire) = request(port, "POST", "/auth/login", None, Some(&credentials(EMAIL)));
    assert_eq!(statut, 200, "la connexion doit rendre une paire : {paire}");

    let (statut, corps) = request(port, "POST", "/posts", None, Some(ARTICLE));
    assert_eq!(
        statut, 401,
        "sans jeton, la route gardée doit dire « identifie-toi » et non « tu n'as pas le droit » : {corps}"
    );

    let (statut, corps) = request(port, "POST", "/posts", Some(&access(&paire)), Some(ARTICLE));
    assert_eq!(
        statut, 403,
        "un `user` authentifié doit être refusé sur une route réservée aux admins : {corps}"
    );

    // Sans cette ligne, un 403 rendu par une route cassée passerait pour une garde qui
    // fonctionne.
    let (statut, corps) = request(port, "GET", "/posts", Some(&access(&paire)), None);
    assert_eq!(
        statut, 200,
        "la ressource doit rester servie au même compte en lecture : {corps}"
    );
}

/// `examples/blog-auth`, copié hors du dépôt et repointé sur `url`.
///
/// La copie est ce qui permet de le lancer sans écrire dans le dépôt — au prix du chemin
/// relatif vers le noyau, qu'il faut refaire.
fn blog_auth_on(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("blog-auth");
    copy(&common::depot().join("examples/blog-auth"), &racine);

    let manifeste = racine.join("Cargo.toml");
    let source = fs::read_to_string(&manifeste).expect("le manifeste de l'exemple est lisible");

    // Séparateurs normalisés : un `\` de Windows est un échappement dans une chaîne TOML
    // basique, et le chemin y arriverait mutilé.
    let noyau = common::noyau().display().to_string().replace('\\', "/");
    const RELATIF: &str = r#"path = "../../crates/rbs-core""#;

    assert!(
        source.contains(RELATIF),
        "l'exemple ne pointe plus le noyau par `{RELATIF}` : la copie compilerait contre autre chose :\n{source}"
    );

    fs::write(
        &manifeste,
        source.replace(RELATIF, &format!(r#"path = "{noyau}""#)),
    )
    .expect("le manifeste de la copie est inscriptible");

    // Le `.env` est réécrit en entier : la base est celle du conteneur, et le secret que
    // l'installation a tiré est remplacé par une valeur connue, sans quoi les jetons que
    // le test forge lui-même ne seraient plus vérifiables.
    fs::write(
        racine.join(".env"),
        format!("RBS_ENV=development\nRBS_DATABASE__URL={url}\nRBS_AUTH__SECRET={SECRET}\n"),
    )
    .expect("le `.env` de la copie est inscriptible");

    racine
}

/// Copie `source` dans `destination`, `target` et `.git` exclus.
fn copy(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("répertoire de destination créable");

    for entree in fs::read_dir(source).expect("répertoire source lisible") {
        let chemin = entree.expect("entrée lisible").path();
        let nom = chemin.file_name().expect("entrée nommée").to_owned();

        if nom == "target" || nom == ".git" {
            continue;
        }

        let cible = destination.join(&nom);

        if chemin.is_dir() {
            copy(&chemin, &cible);
        } else {
            fs::copy(&chemin, &cible).expect("fichier copiable");
        }
    }
}

/// Un PostgreSQL neuf, prêt à recevoir le schéma d'un projet généré.
fn start_postgres() -> Container<GenericImage> {
    let (nom, version) = common::postgres_image();
    GenericImage::new(nom, version)
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

/// Un projet neuf portant `auth`, sa base pointée sur `url` et son secret posé.
///
/// Les migrations ne sont pas appliquées : le test du schéma a besoin de l'état d'avant.
fn project_with_auth_on(url: &str, parent: &TempDir) -> PathBuf {
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
fn migrate(racine: &Path) {
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(racine)
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

/// Le binaire d'un projet, lancé sur un port libre, arrêté quand ce garde tombe.
///
/// `Drop` plutôt qu'un `kill` en fin de test : une assertion qui échoue au milieu d'un
/// parcours déroule la pile sans jamais l'atteindre, et laisse derrière elle un serveur
/// qui écoute et un conteneur qu'il tient ouvert.
///
/// `journal` est la valeur de `RUST_LOG` : `debug` pour qui inspecte la sortie, `info`
/// pour les autres — le flux part dans un tuyau que personne ne vide tant que le serveur
/// tourne.
struct Serveur {
    processus: Option<std::process::Child>,
    port: u16,
}

impl Serveur {
    fn lancer(racine: &Path, binaire: &str, journal: &str) -> Self {
        let port = free_port();

        let processus = std::process::Command::new(common::cible().join("debug").join(binaire))
            .current_dir(racine)
            .env("RBS_SERVER__PORT", port.to_string())
            .env("RUST_LOG", journal)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("le binaire du projet doit être lançable");

        wait_for_listening(port);

        Self {
            processus: Some(processus),
            port,
        }
    }

    fn port(&self) -> u16 {
        self.port
    }

    /// Arrête le serveur et rend ce qu'il a écrit sur ses deux flux.
    fn journal(mut self) -> String {
        // `wait_with_output` consomme le `Child` : le retirer du garde laisse `Drop` sans
        // rien à moissonner, plutôt qu'avec un processus déjà attendu.
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

/// Joue une requête sur le serveur local et rend son statut avec son corps décodé.
///
/// La requête est écrite à la main plutôt que par un client HTTP : ces tests n'ont besoin
/// que d'un statut et de deux champs de JSON, et la dépendance se paierait sur toute la CI.
fn request(
    port: u16,
    methode: &str,
    chemin: &str,
    jeton: Option<&str>,
    corps: Option<&str>,
) -> (u16, Value) {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("le serveur doit répondre");

    let corps = corps.unwrap_or_default();
    let mut entete =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");

    if let Some(jeton) = jeton {
        entete.push_str(&format!("Authorization: Bearer {jeton}\r\n"));
    }

    if !corps.is_empty() {
        entete.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            corps.len()
        ));
    }

    entete.push_str("\r\n");
    entete.push_str(corps);

    flux.write_all(entete.as_bytes())
        .expect("la requête doit partir");

    let mut reponse = String::new();
    flux.read_to_string(&mut reponse)
        .expect("la réponse doit être lisible");

    decode(&reponse)
}

/// Sépare le statut du corps d'une réponse HTTP brute.
fn decode(reponse: &str) -> (u16, Value) {
    let statut = reponse
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("réponse sans ligne de statut lisible :\n{reponse}"));

    let corps = reponse
        .split_once("\r\n\r\n")
        .map(|(_, corps)| corps)
        .unwrap_or_default()
        .trim();

    // Le 204 du logout n'a pas de corps, et le message d'un serveur qui refuse la requête
    // avant de le router n'est pas du JSON : rendre le texte brut plutôt que d'échouer ici
    // laisse l'assertion appelante afficher ce que le serveur a réellement dit.
    let corps = serde_json::from_str(corps).unwrap_or_else(|_| Value::String(corps.to_string()));

    (statut, corps)
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

/// Joue `request` dans le conteneur et rend sa sortie, sans en-tête ni alignement.
fn psql(postgres: &testcontainers::Container<GenericImage>, request: &str) -> String {
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
