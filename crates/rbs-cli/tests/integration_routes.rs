//! `rbs routes` et `rbs openapi export` sur un vrai projet, compilé.
//!
//! Le tri, la garde et les deux rendus se prouvent dans `src/routes.rs`, sur un document
//! fixe. Ce fichier prouve ce que seul un projet réel prouve : que le document que le
//! binaire `openapi` imprime porte bien les gardes que les fragments ont posées, et que la
//! sortie standard ne porte que lui pendant que le projet compile.
//!
//! Aucune base n'est touchée : ces tests sont lents sans être dockerisés.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

mod common;

/// Le binaire livré, lancé dans `racine`.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// Ce que la commande a écrit sur ses deux flux, réunis.
fn sortie(commande: &mut Command) -> (i32, String, String) {
    let rendu = commande.output().expect("le binaire doit s'exécuter");

    (
        rendu.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&rendu.stdout).into_owned(),
        String::from_utf8_lossy(&rendu.stderr).into_owned(),
    )
}

/// Pas d'`#[ignore]` : les deux refus arrivent avant que cargo ne soit lancé, et c'est
/// le point.
#[test]
fn a_project_without_the_openapi_binary_is_refused_by_both_commands() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());
    fs::remove_file(racine.join("src/bin/openapi.rs")).expect("le binaire doit se supprimer");

    for arguments in [
        vec!["routes"],
        vec!["routes", "--json"],
        vec!["openapi", "export"],
        vec!["openapi", "export", "--out", "openapi.json"],
    ] {
        let (code, stdout, stderr) = sortie(rbs(&racine).args(&arguments));
        let rendu = format!("{stdout}{stderr}");

        // Sans `--out`, la sortie standard d'`export` est le document, que `> fichier` recueille ;
        // sous `--json`, celle de `routes` aussi. Un refus n'y laisse rien, remède compris.
        if arguments == ["routes", "--json"] || arguments == ["openapi", "export"] {
            assert!(
                stdout.is_empty(),
                "{arguments:?} : stdout doit rester vide :\n{stdout}"
            );
        }

        assert_eq!(code, 1, "{arguments:?} :\n{rendu}");
        assert!(
            rendu.contains("src/bin/openapi.rs"),
            "{arguments:?} :\n{rendu}"
        );
        assert!(rendu.contains("[[bin]]"), "{arguments:?} :\n{rendu}");
        assert!(!rendu.contains("panicked"), "{arguments:?} :\n{rendu}");
    }

    assert!(
        !racine.join("openapi.json").exists(),
        "un refus ne doit rien écrire"
    );
}

/// Un projet `--with auth` : ses routes d'authentification déclarent le jeton, `/health`
/// n'en demande aucun. Le nom est propre à ce banc — `demo-api` est celui de neuf autres
/// suites qui partagent la même cible.
fn projet_avec_auth() -> (TempDir, PathBuf) {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(parent.path())
        .args([
            "new",
            "routes-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/routes_api",
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--with",
            "auth",
            "--yes",
        ])
        .assert()
        .success();

    let racine = parent.path().join("routes-api");
    (parent, racine)
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn the_routes_of_an_authenticated_project_name_their_guard() {
    let (_parent, racine) = projet_avec_auth();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    let (code, stdout, stderr) = sortie(
        rbs(&racine)
            .env("CARGO_TARGET_DIR", common::cible())
            .args(["routes", "--json"]),
    );
    assert_eq!(code, 0, "{stdout}{stderr}");

    // La sortie standard entière, et non une ligne choisie : la compilation du projet
    // passe par stderr, et un seul octet d'elle ici ferait échouer le premier `jq` venu.
    let routes: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|faute| panic!("stdout n'est pas un JSON ({faute}) :\n{stdout}"));
    let routes = routes.as_array().expect("le rendu est un tableau");

    let health = routes
        .iter()
        .find(|route| route["chemin"] == "/health" && route["methode"] == "GET")
        .unwrap_or_else(|| panic!("GET /health absente : {routes:?}"));
    assert_eq!(health["garde"], "public", "{health}");

    assert!(
        routes.iter().any(|route| route["garde"] == "bearer"),
        "aucune route protégée dans un projet --with auth : {routes:?}"
    );

    let (code, table, stderr) = sortie(
        rbs(&racine)
            .env("CARGO_TARGET_DIR", common::cible())
            .arg("routes"),
    );
    assert_eq!(code, 0, "{table}{stderr}");
    assert!(
        table.starts_with("MÉTHODE  CHEMIN"),
        "la table n'ouvre pas sur son en-tête :\n{table}"
    );

    let (code, stdout, stderr) =
        sortie(rbs(&racine).env("CARGO_TARGET_DIR", common::cible()).args([
            "openapi",
            "export",
            "--out",
            "openapi.json",
        ]));
    assert_eq!(code, 0, "{stdout}{stderr}");

    let ecrit = fs::read_to_string(racine.join("openapi.json")).expect("le fichier est écrit");
    let document: Value = serde_json::from_str(&ecrit).expect("le fichier est un JSON");
    assert!(document.get("openapi").is_some(), "{ecrit}");

    let (code, stdout, stderr) = sortie(
        rbs(&racine)
            .env("CARGO_TARGET_DIR", common::cible())
            .args(["openapi", "export"]),
    );
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        stdout, ecrit,
        "le document sur la sortie standard diffère de celui écrit par --out"
    );
}
