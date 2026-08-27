//! Ce que `rbs add auth` dépose dans un projet, éprouvé par la commande telle que
//! l'utilisateur la lance.
//!
//! Deux tests de portée très différente. Le premier lit les ancres du projet et tourne
//! sur chaque PR ; le second compile ce qui a été déposé, et porte `#[ignore]` comme
//! `integration_new` — c'est le seul qui prouve que la feature installée tient debout.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

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
        ("migration/src/lib.rs", "migrations", "create_users"),
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

    Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["fmt", "--all", "--check"])
        .assert()
        .success();
}
