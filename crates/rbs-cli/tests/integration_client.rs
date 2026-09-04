//! `rbs generate client` sur un vrai projet, compilé.
//!
//! Le rendu se prouve dans `src/client/ts.rs`, sans rien compiler. Ce fichier prouve ce que
//! seul un projet réel prouve : que le binaire `openapi` compile, que sa sortie s'analyse,
//! et que le fichier écrit est bien celui que le plan annonçait.
//!
//! Aucune base n'est touchée : ces tests sont lents sans être dockerisés.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

const CLIENT: &str = "clients/ts/client.ts";

/// Le binaire livré, lancé dans `racine`.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// Un projet neuf portant une feature CRUD, prêt à recevoir son client.
///
/// Le `TempDir` est rendu avec la racine : le lâcher effacerait le projet sous le test.
fn projet_avec_crud() -> (TempDir, PathBuf) {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    rbs(&racine)
        .args([
            "generate",
            "crud",
            "articles",
            "--fields",
            "title:string,body:text,published:bool",
            "--force",
        ])
        .assert()
        .success();

    (parent, racine)
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn the_command_writes_a_client_that_carries_one_method_per_operation() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let client = fs::read_to_string(racine.join(CLIENT)).expect("le client doit être écrit");

    for methode in [
        "articlesList(",
        "articlesCreate(",
        "articlesFind(",
        "articlesUpdate(",
        "articlesDelete(",
        "health(",
    ] {
        assert!(client.contains(methode), "{methode} absente :\n{client}");
    }

    // Les types viennent des DTO du projet : sans eux, le client rendrait `unknown` et
    // n'aurait plus d'intérêt sur celui qu'on écrirait à la main.
    assert!(
        client.contains("export interface ArticleResponse {"),
        "{client}"
    );
    assert!(
        client.contains("export interface CreateArticle {"),
        "{client}"
    );
}

/// La signature interne accepte un objet fermé, et c'est ce qui rend le client compilable.
///
/// Une interface de query n'a pas d'index signature : TypeScript refuse de l'assigner à un
/// `Record<string, unknown>`, et le client engendré ne passait pas `tsc --strict`. Poser
/// l'index signature sur les interfaces aurait levé l'erreur en rendant les query ouvertes,
/// où une clé mal orthographiée passe sans un mot — le paramètre a donc cédé, pas le type
/// public.
#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn the_generated_client_passes_a_closed_query_type_to_its_request_helper() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let client = fs::read_to_string(racine.join(CLIENT)).expect("le client doit être écrit");

    assert!(client.contains("query?: object;"), "{client}");
    assert!(
        !client.contains("query?: Record<string, unknown>"),
        "un `Record` ici refuserait toute interface de query :\n{client}"
    );
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn regenerating_the_client_changes_nothing() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let premier = fs::read_to_string(racine.join(CLIENT)).expect("le client doit être écrit");

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let second = fs::read_to_string(racine.join(CLIENT)).expect("le client doit être relu");

    assert_eq!(premier, second, "la régénération doit être idempotente");
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn dry_run_writes_nothing() {
    let (_parent, racine) = projet_avec_crud();

    // Une première passe avant l'empreinte : lire le document impose de compiler le
    // projet, et cargo écrit alors son `Cargo.lock`. C'est son écriture à lui, non celle
    // de rbs — la prendre pour une violation du `--dry-run` accuserait la mauvaise
    // commande.
    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force", "--dry-run"])
        .assert()
        .success();

    let avant = common::empreinte(&racine);

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force", "--dry-run"])
        .assert()
        .success();

    common::assert_intact(&avant, &racine, "une génération en --dry-run");
    assert!(
        !racine.join(CLIENT).exists(),
        "`--dry-run` ne doit pas créer le client"
    );
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn an_explicit_out_directory_is_honoured() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args([
            "generate", "client", "--lang", "ts", "--out", "web/api", "--force",
        ])
        .assert()
        .success();

    assert!(racine.join("web/api/client.ts").exists());
    assert!(
        !racine.join(CLIENT).exists(),
        "`--out` doit déplacer la sortie, non la dupliquer"
    );
}

/// Pas d'`#[ignore]` : le refus arrive avant que cargo ne soit lancé, et c'est le point.
#[test]
fn a_project_without_the_openapi_binary_is_refused_before_cargo_runs() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    fs::remove_file(racine.join("src/bin/openapi.rs")).expect("le binaire doit se supprimer");

    let sortie = rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let rendu = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(rendu.contains("src/bin/openapi.rs"), "{rendu}");
    assert!(rendu.contains("[[bin]]"), "{rendu}");
    assert!(
        !racine.join(CLIENT).exists(),
        "un refus ne doit rien écrire"
    );
}
