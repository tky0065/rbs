//! `rbs test` vu depuis la ligne de commande.
//!
//! Le premier critère se prouve sans base ni compilation : hors d'un projet, la commande
//! doit se dire absente en une phrase, jamais en une trace de panique. Le second n'a pas
//! de raccourci — seul un projet réel, migré contre un vrai PostgreSQL, dit si `rbs test`
//! monte la base avant de lancer `cargo test`, si `--include-ignored` est bien passé, et
//! si le code d'un test rouge remonte tel quel plutôt que d'être aplati à 1.

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Un `rbs` prêt à s'exécuter dans `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Lance `commande` et rend son code de sortie et sa sortie standard et d'erreur réunies.
fn lancer(commande: &mut Command) -> (i32, String) {
    let rendu = commande.output().expect("le binaire doit s'exécuter");

    let sortie = format!(
        "{}{}",
        String::from_utf8_lossy(&rendu.stdout),
        String::from_utf8_lossy(&rendu.stderr)
    );

    (rendu.status.code().unwrap_or(-1), sortie)
}

#[test]
fn outside_a_project_the_command_says_so_and_fails() {
    let ailleurs = TempDir::new().expect("répertoire temporaire créable");

    let mut commande = rbs(ailleurs.path());
    commande.arg("test");
    let (code, sortie) = lancer(&mut commande);

    assert_eq!(code, 2, "code de sortie inattendu :\n{sortie}");
    assert!(
        sortie.contains("projet rbs"),
        "le message ne dit pas ce qui manque :\n{sortie}"
    );
    assert!(
        !sortie.contains("panicked"),
        "une trace de panique est remontée :\n{sortie}"
    );
}

/// La chaîne complète : base montée et migrée, suite verte avec `--include-ignored`, puis
/// un test rouge dont le code doit remonter tel quel.
///
/// Les deux moitiés partagent le même projet plutôt que d'en engendrer un chacune : la
/// seconde n'a besoin que d'un fichier de plus, et un projet Axum + SeaORM complet ne se
/// compile pas deux fois pour le prix d'une preuve qui tient en un seul enchaînement.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn rbs_test_migrates_runs_the_suite_then_reports_a_red_test_unflattened() {
    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);

    let parent = TempDir::new().expect("répertoire temporaire créable");

    // Nom distinct de `demo-api`, déjà pris par neuf autres suites dans la même cible
    // partagée : deux projets du même nom s'y gêneraient.
    rbs(parent.path())
        .args([
            "new",
            "test-api",
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

    let projet = parent.path().join("test-api");

    // Le squelette écrit un compose réel pour une base locale : le laisser ferait `rbs
    // test` monter un second PostgreSQL sur le port de l'URL, par-dessus le conteneur
    // déjà démarré ci-dessus.
    std::fs::remove_file(projet.join("docker-compose.yml")).expect("le compose doit exister");

    rbs(&projet)
        .args(["generate", "crud", "articles", "--fields", "title:string"])
        .assert()
        .success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    let mut commande = rbs(&projet);
    commande
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("test");
    let (code, sortie) = lancer(&mut commande);

    assert_eq!(code, 0, "code de sortie inattendu :\n{sortie}");
    assert!(
        sortie
            .lines()
            .any(|ligne| ligne.trim_start().starts_with("test articles::")
                && ligne.trim_end().ends_with("ok")),
        "aucun test de la feature articles n'a tourné :\n{sortie}"
    );
    // La preuve que `--include-ignored` a bien été passé : sans lui, les tests qui
    // joignent la base — la majorité d'un CRUD généré — resteraient de côté.
    assert!(
        sortie
            .lines()
            .filter(|ligne| ligne.contains("test result:"))
            .all(|ligne| ligne.contains(" 0 ignored")),
        "une suite a laissé des tests de côté malgré --include-ignored :\n{sortie}"
    );

    std::fs::create_dir_all(projet.join("tests")).expect("tests/ doit pouvoir se créer");
    std::fs::write(
        projet.join("tests/rouge.rs"),
        "#[test]\nfn rouge() {\n    panic!(\"rendu rouge exprès\");\n}\n",
    )
    .expect("tests/rouge.rs doit pouvoir s'écrire");

    let mut commande = rbs(&projet);
    commande
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["test", "rouge"]);
    let (code, sortie) = lancer(&mut commande);

    // 101 est le code que le harnais de test de Rust rend sur un panic, propagé tel
    // quel : une CI qui verrait 1 ne pourrait plus distinguer un test rouge d'une
    // commande qui n'a pas pu démarrer.
    assert_eq!(
        code, 101,
        "le code de `cargo test` n'est pas remonté tel quel :\n{sortie}"
    );
    assert!(
        sortie.contains("rouge"),
        "la sortie ne nomme pas le test rendu rouge :\n{sortie}"
    );
}
