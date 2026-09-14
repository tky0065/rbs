//! `--json` sur les commandes qui planifient, joué par le binaire livré.
//!
//! Un agent lit le plan ou l'erreur dans un document plutôt que dans de l'ANSI : ces
//! tests analysent la sortie standard **entière**, parce qu'une seule ligne humaine
//! glissée devant le document suffirait à la rendre inanalysable.
//!
//! Aucun `#[ignore]` : ni Docker ni compilation du projet engendré, ces tests doivent
//! tourner sur chaque PR.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

mod common;

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Un projet neuf, créé comme un utilisateur le ferait.
fn projet_neuf(parent: &TempDir) -> PathBuf {
    rbs(parent.path())
        .args(["new", "demo", "--yes"])
        .assert()
        .success();

    parent.path().join("demo")
}

/// Ce qu'une exécution du binaire a produit : code, sortie standard, sortie d'erreur.
struct Sortie {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn lancer(repertoire: &Path, arguments: &[&str]) -> Sortie {
    let output = rbs(repertoire)
        .args(arguments)
        .output()
        .expect("le binaire doit être lançable");

    Sortie {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// La sortie standard entière, analysée comme un seul document.
fn document(sortie: &Sortie) -> Value {
    serde_json::from_str(&sortie.stdout).unwrap_or_else(|faute| {
        panic!(
            "la sortie standard n'est pas un document JSON ({faute}) :\n{}\n--- stderr :\n{}",
            sortie.stdout, sortie.stderr
        )
    })
}

/// Le plan d'`add` sort en JSON, seul sur la sortie standard, et `--dry-run` n'écrit
/// toujours rien.
#[test]
fn add_dry_run_json_prints_the_plan_as_the_only_document_and_writes_nothing() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);
    let avant = common::empreinte(&racine);

    let sortie = lancer(&racine, &["add", "cors", "--dry-run", "--json"]);

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "add");
    assert_eq!(plan["applique"], false);

    let actions = plan["actions"]
        .as_array()
        .expect("`actions` est un tableau");
    assert!(
        actions
            .iter()
            .any(|action| action["effet"]["type"] == "creer"
                && action["effet"]["contenu"]
                    .as_str()
                    .is_some_and(|contenu| !contenu.is_empty())),
        "aucune création à contenu complet :\n{plan:#}"
    );
    assert!(
        actions
            .iter()
            .any(|action| action["effet"]["type"] == "inserer"
                && action["effet"]["ancre"] == "layers"),
        "aucune insertion dans `layers` :\n{plan:#}"
    );

    common::assert_intact(&avant, &racine, "`--dry-run --json` a écrit dans le projet");
}

/// Une ancre retirée arrête la commande comme sans `--json` — code 1, rien d'écrit — mais
/// l'erreur sort en document sur la sortie standard, bloc à recoller compris, et la
/// sortie d'erreur reste vide.
#[test]
fn a_missing_anchor_is_rendered_as_a_json_error_with_its_block_and_exit_code_1() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);

    let router = racine.join("src").join("router.rs");
    let source = fs::read_to_string(&router).expect("router.rs lisible");
    let ampute: String = source
        .lines()
        .filter(|ligne| !ligne.contains("// <rbs:layers>") && !ligne.contains("// </rbs:layers>"))
        .map(|ligne| format!("{ligne}\n"))
        .collect();
    assert_ne!(ampute, source, "l'ancre `layers` n'a pas été retirée");
    fs::write(&router, ampute).expect("router.rs inscriptible");
    let avant = common::empreinte(&racine);

    let sortie = lancer(&racine, &["add", "cors", "--json", "--force"]);

    assert_eq!(sortie.code, Some(1), "{}", sortie.stdout);
    assert_eq!(
        sortie.stderr, "",
        "sous --json, l'erreur ne passe que par le document"
    );
    let erreur = &document(&sortie)["erreur"];
    assert_eq!(erreur["code"], "ancre_absente", "{erreur:#}");
    assert_eq!(
        erreur["bloc"], "// <rbs:layers>\n// </rbs:layers>",
        "{erreur:#}"
    );
    assert!(
        erreur["message"]
            .as_str()
            .is_some_and(|message| message.contains("src/router.rs")),
        "le message doit nommer le fichier :\n{erreur:#}"
    );
    assert!(
        erreur["remede"]
            .as_str()
            .is_some_and(|remede| remede.contains("src/router.rs")),
        "un bloc sans remède laisserait deviner où le coller :\n{erreur:#}"
    );

    common::assert_intact(
        &avant,
        &racine,
        "l'ancre absente n'a pas empêché l'écriture",
    );
}

/// `generate crud` rend lui aussi son plan en un document que `serde_json` analyse.
#[test]
fn generate_crud_dry_run_json_prints_an_analysable_plan() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);
    let avant = common::empreinte(&racine);

    let sortie = lancer(
        &racine,
        &[
            "generate",
            "crud",
            "articles",
            "--fields",
            "title:string",
            "--dry-run",
            "--json",
            "--force",
        ],
    );

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "generate crud");
    assert_eq!(plan["applique"], false);
    assert!(
        plan["actions"]
            .as_array()
            .is_some_and(|actions| !actions.is_empty()),
        "un CRUD neuf a des fichiers à écrire :\n{plan:#}"
    );

    common::assert_intact(&avant, &racine, "`--dry-run --json` a écrit dans le projet");
}

/// Un projet neuf est déjà à la version du CLI : le document le dit par des `actions`
/// vides et `applique` à `false`, sans ligne humaine autour.
#[test]
fn upgrade_dry_run_json_prints_an_analysable_document() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);

    let sortie = lancer(&racine, &["upgrade", "--dry-run", "--json"]);

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "upgrade");
    assert_eq!(plan["applique"], false);
    assert_eq!(plan["actions"], Value::Array(Vec::new()), "{plan:#}");
}

/// Hors d'un projet, l'erreur sort elle aussi en document, sous son code stable.
#[test]
fn add_json_outside_a_project_renders_pas_un_projet() {
    let vide = TempDir::new().expect("répertoire temporaire créable");

    let sortie = lancer(vide.path(), &["add", "cors", "--json"]);

    assert_eq!(sortie.code, Some(2), "{}", sortie.stdout);
    assert_eq!(
        sortie.stderr, "",
        "sous --json, l'erreur ne passe que par le document"
    );
    let erreur = &document(&sortie)["erreur"];
    assert_eq!(erreur["code"], "pas_un_projet", "{erreur:#}");
    assert_eq!(erreur["bloc"], Value::Null, "{erreur:#}");
}

/// Sans `--dry-run`, le plan s'écrit et le document le dit : c'est le chemin qu'un agent
/// prend pour installer, et le seul où `applique` vaut `true`.
#[test]
fn add_json_without_dry_run_writes_the_plan_and_says_it_was_applied() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);

    let sortie = lancer(&racine, &["add", "cors", "--json", "--force"]);

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    assert_eq!(
        sortie.stderr, "",
        "cors n'a rien à dire sur la sortie d'erreur"
    );
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "add");
    assert_eq!(plan["applique"], true, "{plan:#}");

    assert!(
        racine.join("src/modules/cors/mod.rs").is_file(),
        "le document dit le plan appliqué, mais le module cors n'est pas écrit"
    );
    let router = fs::read_to_string(racine.join("src/router.rs")).expect("router.rs lisible");
    assert!(
        router.contains(".layer(crate::modules::cors::layer())"),
        "la couche cors n'a pas été insérée :\n{router}"
    );
}

/// Un `AGENTS.md` supprimé est ce qu'`upgrade` a toujours à rétablir, même sur un projet
/// à jour : sans lui, le plan n'aurait rien à écrire et `applique` resterait à `false`.
#[test]
fn upgrade_json_without_dry_run_restores_the_guide_and_says_it_was_applied() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);
    let guide = racine.join("AGENTS.md");
    fs::remove_file(&guide).expect("le guide est là");

    let sortie = lancer(&racine, &["upgrade", "--json"]);

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "upgrade");
    assert_eq!(plan["applique"], true, "{plan:#}");
    assert!(
        guide.is_file(),
        "le document dit le plan appliqué, mais AGENTS.md n'est pas rétabli"
    );
}

/// `generate job` rend son plan comme les autres commandes qui planifient : un seul
/// document, et `--dry-run` n'écrit toujours rien.
#[test]
fn generate_job_dry_run_json_prints_an_analysable_plan() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    rbs(parent.path())
        .args(["new", "demo", "--yes", "--with", "jobs"])
        .assert()
        .success();
    let racine = parent.path().join("demo");
    let avant = common::empreinte(&racine);

    let sortie = lancer(
        &racine,
        &["generate", "job", "purge", "--dry-run", "--json", "--force"],
    );

    assert_eq!(sortie.code, Some(0), "{}", sortie.stderr);
    let plan = document(&sortie);
    assert_eq!(plan["commande"], "generate job");
    assert_eq!(plan["applique"], false);
    assert!(
        plan["actions"].as_array().is_some_and(|actions| actions
            .iter()
            .any(|action| action["chemin"] == "src/modules/jobs/purge.rs")),
        "le plan crée le fichier du job :\n{plan:#}"
    );

    common::assert_intact(&avant, &racine, "`--dry-run --json` a écrit dans le projet");
}

/// Sans la file, le refus sort en document, et son remède est la commande qui l'installe.
#[test]
fn generate_job_json_without_jobs_renders_jobs_absent() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_neuf(&parent);

    let sortie = lancer(&racine, &["generate", "job", "purge", "--json", "--force"]);

    assert_eq!(sortie.code, Some(2), "{}", sortie.stdout);
    assert_eq!(
        sortie.stderr, "",
        "sous --json, l'erreur ne passe que par le document"
    );
    let erreur = &document(&sortie)["erreur"];
    assert_eq!(erreur["code"], "jobs_absent", "{erreur:#}");
    assert_eq!(erreur["remede"], "rbs add jobs", "{erreur:#}");
}

/// Relancé sur un job déjà en place, `generate job` n'écrit rien, et son document le dit
/// comme celui d'`add` : `applique` à `false`.
#[test]
fn generate_job_json_rerun_on_a_job_in_place_says_nothing_was_applied() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    rbs(parent.path())
        .args(["new", "demo", "--yes", "--with", "jobs"])
        .assert()
        .success();
    let racine = parent.path().join("demo");

    let premiere = lancer(&racine, &["generate", "job", "purge", "--json", "--force"]);
    assert_eq!(premiere.code, Some(0), "{}", premiere.stderr);
    assert_eq!(document(&premiere)["applique"], true, "{}", premiere.stdout);

    let seconde = lancer(&racine, &["generate", "job", "purge", "--json", "--force"]);
    assert_eq!(seconde.code, Some(0), "{}", seconde.stderr);
    assert_eq!(document(&seconde)["applique"], false, "{}", seconde.stdout);
}
