//! La chaîne complète, du projet vide aux tests d'une feature CRUD qui passent.
//!
//! C8 prouve qu'un projet neuf compile ; celui-ci va jusqu'à la base : une feature
//! générée, sa migration appliquée, et les tests générés exécutés contre un vrai
//! PostgreSQL. Aucune étape n'est simulée — le binaire `rbs` est invoqué comme un
//! utilisateur l'invoquerait.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Stdio;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{ExecCommand, IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{GenericImage, ImageExt};

mod common;

const UTILISATEUR: &str = "rbs";
const MOT_DE_PASSE: &str = "rbs";
const BASE: &str = "demo";

// L'image MySQL ne crée que `root` : `MYSQL_USER` n'est pas passé par le démarreur commun.
const UTILISATEUR_MYSQL: &str = "root";

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_generated_crud_migrates_and_passes_its_tests_against_postgresql() {
    // Le conteneur d'abord : son port détermine l'URL que le projet portera dans son
    // `.env`. Créer le projet avant obligerait à réécrire ce fichier après coup.
    let (nom, version) = common::postgres_image();
    let postgres = GenericImage::new(nom, version)
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

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // Les tests générés montent l'application sur la base décrite par le `.env` : ils ne
    // passent que si la migration a bien été appliquée juste avant, et sont `#[ignore]`
    // pour cette raison — sans `--include-ignored`, cette étape ne lancerait plus rien.
    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace", "--", "--include-ignored"])
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

    // Le filtre s'exige de même : une condition mal traduite en SQL ne se voit qu'ici, la
    // requête étant construite à la génération et jouée contre une vraie base.
    assert!(
        joues.contains("test articles::tests::the_filter_narrows_the_list ... ok"),
        "le scénario de filtrage n'a pas été joué :\n{joues}"
    );
    assert!(
        joues.contains("test articles::tests::an_unknown_sort_column_returns_400 ... ok"),
        "le refus d'une colonne de tri inconnue n'a pas été joué :\n{joues}"
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

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

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

/// La migration d'une suppression logique s'applique, et son unicité ne porte que sur les
/// lignes vivantes.
///
/// Les tests unitaires lisent une chaîne de caractères : seul ce banc dit si PostgreSQL
/// accepte l'index partiel que la template écrit, et si le repository engendré filtre
/// bien ce qu'il doit taire. Le nom de la feature — `soft_articles` — reste propre à ce
/// banc : il partage `target/rbs-integration` avec les deux tests ci-dessus, dont les
/// features `articles`, `users` et `posts` ne le recouvrent pas.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_soft_deleting_crud_migrates_and_hides_its_deleted_rows() {
    let (nom, version) = common::postgres_image();
    let postgres = GenericImage::new(nom, version)
        .with_wait_for(WaitFor::log(
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
            "soft_articles",
            "--fields",
            "title:string:unique",
            "--soft-delete",
        ])
        .assert()
        .success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // La promesse même de l'index partiel — la seule raison de le préférer à une unicité
    // globale — est qu'une valeur redevient disponible dès que la ligne qui la portait est
    // supprimée. `a_replayed_unique_value_returns_409` plus bas ne prouve que l'autre
    // moitié, le refus tant que cette ligne reste vivante ; une clause `.and_where(...)`
    // oubliée la laisserait passer sans être vue.
    let mut rebond = postgres
        .exec(ExecCommand::new([
            "psql",
            "-U",
            UTILISATEUR,
            "-d",
            BASE,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            "insert into soft_articles (id, title, created_at, updated_at) \
             values ('00000000-0000-4000-8000-000000000001', 'rebond-apres-suppression', now(), now()); \
             update soft_articles set deleted_at = now() \
             where id = '00000000-0000-4000-8000-000000000001'; \
             insert into soft_articles (id, title, created_at, updated_at) \
             values ('00000000-0000-4000-8000-000000000002', 'rebond-apres-suppression', now(), now());",
        ]))
        .expect("psql doit pouvoir s'exécuter dans le conteneur");

    let sortie_rebond = format!(
        "{}{}",
        String::from_utf8_lossy(&rebond.stdout_to_vec().expect("la sortie de psql se lit")),
        String::from_utf8_lossy(&rebond.stderr_to_vec().expect("l'erreur de psql se lit")),
    );

    assert_eq!(
        sortie_rebond.matches("INSERT 0 1").count(),
        2,
        "la valeur unique ne se libère pas après la suppression de la ligne qui la \
         portait — l'index n'est plus partiel :\n{sortie_rebond}"
    );

    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "--workspace", "--", "--include-ignored"])
        .assert()
        .success()
        .get_output()
        .clone();

    let joues = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    // La suppression logique tient tout entière dans ce scénario : la ligne créée
    // disparaît des lectures (`GET` y rend 404) sans quitter la table, et un second
    // `DELETE` retrouve la même absence plutôt qu'un nouveau succès — la garde manquante
    // que la template commente rendrait 204 les deux fois.
    assert!(
        joues.contains("test soft_articles::tests::the_full_lifecycle_goes_through_the_api ... ok"),
        "le cycle complet n'a pas été joué :\n{joues}"
    );

    // Le doublon n'est refusé que tant que la ligne d'origine reste vivante : c'est
    // l'index partiel qui le permet, et c'est lui, et lui seul, que ce test met à
    // l'épreuve contre une vraie base — un rendu correct en apparence mais que PostgreSQL
    // refuserait à l'application ne se verrait qu'ici.
    assert!(
        joues.contains("test soft_articles::tests::a_replayed_unique_value_returns_409 ... ok"),
        "le refus du doublon n'a pas été joué :\n{joues}"
    );

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["doctor"])
        .assert()
        .success();
}

/// Le même scénario contre SQLite, qui écrit l'index partiel unique autrement que
/// PostgreSQL — la garde `and_where` de la template ne change pas, mais le SQL qu'elle
/// produit sous ce moteur n'est jamais exercé par le banc ci-dessus.
///
/// Aucun conteneur n'est requis : la base est un fichier du projet.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_soft_deleting_crud_migrates_and_hides_its_deleted_rows_on_sqlite() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            "sqlite",
            "--database-url",
            "sqlite://demo_api.db?mode=rwc",
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
            "soft_articles",
            "--fields",
            "title:string:unique",
            "--soft-delete",
        ])
        .assert()
        .success();

    // Une cible propre à ce banc, comme pour les trois moteurs d'`integration_new` :
    // SQLite active des features `sea-orm` que PostgreSQL n'active pas, et une cible
    // commune ferait recompiler l'un pour l'autre à chaque bascule.
    let cible = common::cible_pour("soft-delete-sqlite");
    let _verrou = common::verrou(&cible);

    rbs(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["migrate", "up"])
        .assert()
        .success();

    // Le même rebond qu'au banc PostgreSQL, contre le fichier SQLite : la base est ici un
    // simple fichier, `sqlite3` s'y connecte directement sans conteneur à traverser.
    //
    // SeaORM range l'`uuid` en BLOB de 16 octets sur SQLite, jamais en texte : un `id`
    // écrit comme la chaîne à tirets que Postgres accepte nativement casse le décodage de
    // la première lecture venue (« invalid length: expected 16 bytes, found 36 »), sur une
    // colonne que le rebond ne met pourtant pas en cause. D'où les littéraux `x'…'`.
    let rebond = std::process::Command::new("sqlite3")
        .arg(projet.join("demo_api.db"))
        .arg(
            "insert into soft_articles (id, title, created_at, updated_at) \
             values (x'00000000000040008000000000000001', 'rebond-apres-suppression', datetime('now'), datetime('now')); \
             update soft_articles set deleted_at = datetime('now') \
             where id = x'00000000000040008000000000000001'; \
             insert into soft_articles (id, title, created_at, updated_at) \
             values (x'00000000000040008000000000000002', 'rebond-apres-suppression', datetime('now'), datetime('now'));",
        )
        .output()
        .expect("sqlite3 doit être lançable");

    assert!(
        rebond.status.success(),
        "la valeur unique ne se libère pas après la suppression de la ligne qui la \
         portait sur SQLite — l'index n'est plus partiel :\n{}",
        String::from_utf8_lossy(&rebond.stderr)
    );

    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["test", "--workspace", "--", "--include-ignored"])
        .output()
        .expect("cargo doit être lançable");

    let joues = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        sortie.status.success(),
        "la suite du projet engendré échoue sur SQLite :\n{joues}"
    );

    assert!(
        joues.contains("test soft_articles::tests::the_full_lifecycle_goes_through_the_api ... ok"),
        "le cycle complet n'a pas été joué sur SQLite :\n{joues}"
    );
    assert!(
        joues.contains("test soft_articles::tests::a_replayed_unique_value_returns_409 ... ok"),
        "le refus du doublon n'a pas été joué sur SQLite :\n{joues}"
    );
}

/// Le même scénario contre MySQL, le seul moteur des trois qui ne sait pas restreindre un
/// index à un sous-ensemble de lignes.
///
/// Deux choses s'y jouent, qu'aucun autre banc ne joue. La branche `get_database_backend()`
/// de la migration n'est prise qu'ici : ailleurs, la clause `and_where` part toujours.
/// Et `Index::create().if_not_exists()` n'avait jamais rencontré MySQL — le banc des trois
/// moteurs engendre un CRUD sans champ `unique` ni `index`, donc sans le moindre
/// `CREATE INDEX`. Une migration qui échouerait ici échouerait chez tout utilisateur MySQL
/// dès son premier champ indexé, drapeau ou non.
///
/// L'unicité y reste globale : le rebond que PostgreSQL et SQLite accordent après une
/// suppression logique est ici refusé, et c'est ce refus que le banc exige — la
/// documentation le promet, seul un vrai MySQL le prouve.
#[test]
#[ignore = "démarre MySQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_soft_deleting_crud_keeps_a_global_uniqueness_on_mysql() {
    let mysql = common::start_mysql();
    let url = common::url_of_mysql(&mysql);

    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            "mysql",
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

    // Un nom de feature propre à ce banc : la cible de compilation est celle du moteur,
    // partagée avec la branche MySQL d'`integration_new`, dont la feature `articles` ne
    // doit pas se confondre avec celle-ci.
    rbs(&projet)
        .args([
            "generate",
            "crud",
            "soft_memos",
            "--fields",
            "title:string:unique",
            "--soft-delete",
        ])
        .assert()
        .success();

    let cible = common::cible_pour("mysql");
    let _verrou = common::verrou(&cible);

    rbs(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["migrate", "up"])
        .assert()
        .success();

    // Le rebond que les deux autres moteurs accordent, rejoué ici pour constater qu'il est
    // refusé : l'index unique de MySQL ignore `deleted_at`, la valeur reste réservée.
    let mut rebond = mysql
        .exec(ExecCommand::new([
            "mysql",
            &format!("-u{UTILISATEUR_MYSQL}"),
            &format!("-p{MOT_DE_PASSE}"),
            BASE,
            "-e",
            "insert into soft_memos (id, title, created_at, updated_at) \
             values (x'00000000000040008000000000000001', 'rebond-apres-suppression', now(), now()); \
             update soft_memos set deleted_at = now() \
             where id = x'00000000000040008000000000000001'; \
             insert into soft_memos (id, title, created_at, updated_at) \
             values (x'00000000000040008000000000000002', 'rebond-apres-suppression', now(), now());",
        ]))
        .expect("mysql doit pouvoir s'exécuter dans le conteneur");

    let sortie_rebond = format!(
        "{}{}",
        String::from_utf8_lossy(&rebond.stdout_to_vec().expect("la sortie de mysql se lit")),
        String::from_utf8_lossy(&rebond.stderr_to_vec().expect("l'erreur de mysql se lit")),
    );

    assert!(
        sortie_rebond.contains("Duplicate entry"),
        "MySQL a laissé passer le doublon après suppression logique : son index unique \
         n'est plus global, alors qu'il ne sait pas être partiel :\n{sortie_rebond}"
    );

    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["test", "--workspace", "--", "--include-ignored"])
        .output()
        .expect("cargo doit être lançable");

    let joues = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        sortie.status.success(),
        "la suite du projet engendré échoue sur MySQL :\n{joues}"
    );

    assert!(
        joues.contains("test soft_memos::tests::the_full_lifecycle_goes_through_the_api ... ok"),
        "le cycle complet n'a pas été joué sur MySQL :\n{joues}"
    );
    assert!(
        joues.contains("test soft_memos::tests::a_replayed_unique_value_returns_409 ... ok"),
        "le refus du doublon n'a pas été joué sur MySQL :\n{joues}"
    );
}

/// Le CRUD à routes de contenu compile contre le trait que le fragment installe.
///
/// Les tests unitaires comparent des chaînes de caractères : seul ce banc dit si les
/// appels engendrés — `state.storage()`, puis les cinq méthodes de `Storage` derrière —
/// satisfont réellement la signature du trait, et si le tout passe clippy sans
/// avertissement.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn an_uploading_crud_compiles_against_the_storage_trait() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "storage"]).assert().success();
    rbs(&projet)
        .args([
            "generate",
            "crud",
            "attachments",
            "--fields",
            "title:string",
            "--with-upload",
        ])
        .assert()
        .success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
        .assert()
        .success();

    // `--all-targets`, pour que `src/attachments/tests.rs` entre dans la compilation
    // vérifiée, et `-D warnings` : c'est la commande que `rbs add ci` inscrit dans le
    // workflow livré, celle qui jugerait ce code chez l'utilisateur.
    Command::new("cargo")
        .current_dir(&projet)
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
}

/// `--with-upload` sur un projet sans le fragment `storage` refuse en nommant la commande
/// qui répare, avant d'avoir rien écrit.
#[test]
#[ignore = "engendre un projet complet"]
fn uploading_without_the_storage_feature_is_refused() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let sortie = rbs(&projet)
        .args([
            "generate",
            "crud",
            "attachments",
            "--fields",
            "title:string",
            "--with-upload",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();

    let erreur = String::from_utf8_lossy(&sortie.stderr);
    assert!(
        erreur.contains("rbs add storage"),
        "le refus doit nommer la commande qui répare : {erreur}"
    );
    assert!(
        !projet.join("src/attachments").exists(),
        "le refus tombe avant tout écrit : src/attachments ne devrait pas exister"
    );
}

/// Ce que ni la compilation ni clippy ne peuvent dire : l'octet déposé par `PUT` est celui
/// que `GET` rend, et `HEAD` reflète bien la présence ou l'absence d'un contenu.
///
/// Le backend `fs` du fragment `storage` n'exige aucun service : un répertoire suffit. La
/// base est SQLite, un fichier du projet, pour la même raison — aucun conteneur à faire
/// tenir en plus du binaire compilé.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet et lance son binaire : plusieurs minutes"]
fn the_deposited_content_round_trips_through_the_running_server() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            "sqlite",
            "--database-url",
            "sqlite://demo_api.db?mode=rwc",
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let projet = parent.path().join("demo-api");

    rbs(&projet).args(["add", "storage"]).assert().success();
    rbs(&projet)
        .args([
            "generate",
            "crud",
            "attachments",
            "--fields",
            "title:string",
            "--with-upload",
        ])
        .assert()
        .success();

    // Une cible propre à ce banc : SQLite active des features `sea-orm` que PostgreSQL
    // n'active pas, comme pour les bancs de suppression logique ci-dessus — une cible
    // commune ferait recompiler l'un pour l'autre à chaque bascule.
    let cible = common::cible_pour("attachments-sqlite");
    let _verrou = common::verrou(&cible);

    rbs(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["migrate", "up"])
        .assert()
        .success();

    Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .arg("build")
        .assert()
        .success();

    let serveur = Serveur::lancer(&projet, &cible, "demo-api");
    let port = serveur.port;

    let (statut, corps) = json_request(port, "POST", "/attachments", br#"{"title":"note"}"#);
    assert_eq!(
        statut,
        201,
        "la création doit aboutir : {}",
        String::from_utf8_lossy(&corps)
    );
    let cree: serde_json::Value =
        serde_json::from_slice(&corps).expect("le corps créé doit être du JSON");
    let id = cree["id"].as_str().expect("la réponse doit porter un id");

    const DEPOSE: &[u8] = b"un contenu binaire quelconque, pas seulement du texte \xff\xfe";

    let (statut, _) = raw_request(port, "PUT", &format!("/attachments/{id}/content"), DEPOSE);
    assert_eq!(statut, 204, "le dépôt du contenu doit aboutir");

    let (statut, relu) = raw_request(port, "GET", &format!("/attachments/{id}/content"), &[]);
    assert_eq!(statut, 200, "le contenu déposé doit se relire");
    assert_eq!(
        relu, DEPOSE,
        "l'octet rendu par GET diffère de celui déposé par PUT"
    );

    let (statut, _) = raw_request(port, "HEAD", &format!("/attachments/{id}/content"), &[]);
    assert_eq!(
        statut, 204,
        "HEAD doit refléter la présence d'un contenu déposé"
    );

    // Un identifiant jamais créé : aucune ligne, donc aucun contenu à trouver.
    let (statut, _) = raw_request(
        port,
        "HEAD",
        "/attachments/00000000-0000-4000-8000-000000000099/content",
        &[],
    );
    assert_eq!(statut, 404, "HEAD doit refléter l'absence de contenu");
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
/// `Drop` plutôt qu'un arrêt explicite : une assertion qui échoue au milieu du parcours
/// déroule la pile sans jamais l'atteindre, et laisserait sinon un serveur ouvert derrière
/// elle.
struct Serveur {
    processus: std::process::Child,
    port: u16,
}

impl Serveur {
    fn lancer(racine: &std::path::Path, cible: &std::path::Path, binaire: &str) -> Self {
        let port = free_port();

        let processus = std::process::Command::new(cible.join("debug").join(binaire))
            .current_dir(racine)
            .env("RBS_SERVER__PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("le binaire du projet doit être lançable");

        wait_for_listening(port);

        Self { processus, port }
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        let _ = self.processus.kill();
        let _ = self.processus.wait();
    }
}

/// Joue une requête `POST` au corps JSON, et rend son statut avec son corps en octets.
fn json_request(port: u16, methode: &str, chemin: &str, corps: &[u8]) -> (u16, Vec<u8>) {
    envoyer(port, methode, chemin, "application/json", corps)
}

/// Joue une requête dont le corps est binaire, et rend son statut avec son corps en
/// octets.
///
/// Écrite à la main plutôt qu'avec un client HTTP : le corps d'un `GET .../content` n'est
/// pas du texte, et un client qui le décoderait comme tel perdrait la preuve même que ce
/// banc cherche à faire — que l'octet rendu est bien celui qui a été déposé.
fn raw_request(port: u16, methode: &str, chemin: &str, corps: &[u8]) -> (u16, Vec<u8>) {
    envoyer(port, methode, chemin, "application/octet-stream", corps)
}

fn envoyer(
    port: u16,
    methode: &str,
    chemin: &str,
    content_type: &str,
    corps: &[u8],
) -> (u16, Vec<u8>) {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("le serveur doit répondre");

    let mut entete =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
    if !corps.is_empty() {
        entete.push_str(&format!(
            "Content-Type: {content_type}\r\nContent-Length: {}\r\n",
            corps.len()
        ));
    }
    entete.push_str("\r\n");

    let mut trame = entete.into_bytes();
    trame.extend_from_slice(corps);

    flux.write_all(&trame).expect("la requête doit partir");

    let mut reponse = Vec::new();
    flux.read_to_end(&mut reponse)
        .expect("la réponse doit être lisible");

    let separateur = reponse
        .windows(4)
        .position(|fenetre| fenetre == b"\r\n\r\n")
        .expect("la réponse doit séparer ses en-têtes de son corps");

    let entetes = String::from_utf8_lossy(&reponse[..separateur]);
    let statut = entetes
        .lines()
        .next()
        .and_then(|ligne| ligne.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("réponse sans ligne de statut lisible :\n{entetes}"));

    (statut, reponse[separateur + 4..].to_vec())
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
