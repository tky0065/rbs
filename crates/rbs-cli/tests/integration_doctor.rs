//! `rbs doctor` sur un projet dont un fichier ment.
//!
//! Ces deux fautes se lisent dans le manifeste et la configuration : ni conteneur ni
//! compilation du projet engendré, là où les autres tests d'intégration en demandent.
//! Un diagnostic qui exigerait la base qu'il diagnostique ne servirait à personne.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Port réservé, où rien n'écoute : le contrôle `base` y renonce aussitôt, plutôt que
/// d'attendre trois secondes un serveur que ces tests n'ont aucune raison d'avoir.
const INJOIGNABLE: &str = "postgres://rbs:rbs@127.0.0.1:1/demo_api";

/// Une feature installée dont la configuration a perdu sa section n'a plus de réglages :
/// le projet compile et échoue au démarrage, quand `doctor` peut le dire à froid.
#[test]
fn jobs_declared_without_its_section_is_diagnosed() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);

    rbs(&projet).args(["add", "jobs"]).assert().success();
    retirer_la_section(&projet, "[jobs]");

    let rendu = diagnostic(&projet);
    let ligne = ligne(&rendu, "jobs");

    assert!(ligne.contains('✗'), "{ligne}\n\n{rendu}");
    assert!(
        ligne.contains("[jobs]"),
        "le constat doit nommer la section manquante : {ligne}"
    );
}

/// L'envers du refus de `rbs new --database mysql --database-url postgres://…` : la même
/// contradiction, constatée après coup sur un projet dont un fichier a été édité.
#[test]
fn a_driver_at_odds_with_the_url_is_named_on_both_sides() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    viser(&projet, "mysql://root:root@127.0.0.1:1/demo_api");

    let rendu = diagnostic(&projet);
    let ligne = ligne(&rendu, "base");

    assert!(ligne.contains('✗'), "{ligne}\n\n{rendu}");
    assert!(
        ligne.contains("sqlx-postgres") && ligne.contains("mysql"),
        "le constat doit nommer les deux valeurs en conflit : {ligne}"
    );
}

/// Un guide absent se diagnostique à froid, comme le reste : c'est le seul moyen pour un
/// développeur d'apprendre que son projet a cessé de dire aux agents comment travailler.
#[test]
fn a_deleted_agents_file_is_diagnosed() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);
    fs::remove_file(projet.join("AGENTS.md")).expect("le fichier existe");

    let rendu = diagnostic(&projet);

    assert!(rendu.contains("AGENTS.md"), "{rendu}");
    assert!(rendu.contains("rbs upgrade"), "{rendu}");
}

/// Le contrôle « le CLI d'abord » nomme le module écrit à la main sur une ligne
/// d'avertissement, non d'échec — ce qu'un projet légitime doit pouvoir garder en CI.
///
/// L'assertion porte sur le marqueur de la ligne `agents`, et non sur le code de sortie :
/// celui-ci vaut 1 par le contrôle `base`, dont la base est injoignable ici, et ne dirait
/// donc rien de ce contrôle-ci.
#[test]
fn a_module_written_by_hand_is_reported_as_a_warning_not_a_failure() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);
    fs::create_dir_all(projet.join("src/webhooks")).expect("répertoire créable");
    fs::write(projet.join("src/webhooks/mod.rs"), "// à la main\n").expect("l'écriture aboutit");

    let rendu = diagnostic(&projet);
    let ligne = ligne(&rendu, "agents");

    assert!(ligne.contains('!'), "{ligne}\n\n{rendu}");
    assert!(!ligne.contains('✗'), "{ligne}\n\n{rendu}");
    assert!(
        ligne.contains("webhooks"),
        "le constat doit nommer le module : {ligne}"
    );
}

/// Le `.env` du projet, réécrit pour viser `url`.
fn viser(projet: &Path, url: &str) {
    let env = projet.join(".env");
    let source = fs::read_to_string(&env).expect(".env lisible");
    let reecrit: String = source
        .lines()
        .map(|ligne| match ligne.starts_with("RBS_DATABASE__URL=") {
            true => format!("RBS_DATABASE__URL={url}\n"),
            false => format!("{ligne}\n"),
        })
        .collect();

    assert!(reecrit.contains(url), ".env sans RBS_DATABASE__URL");
    fs::write(&env, reecrit).expect(".env inscriptible");
}

/// Retire une section de `config/default.toml`, en la mettant en commentaire.
fn retirer_la_section(projet: &Path, section: &str) {
    let config = projet.join("config/default.toml");
    let source = fs::read_to_string(&config).expect("config lisible");
    assert!(source.contains(section), "{section} absente de {source}");
    fs::write(&config, source.replace(section, &format!("# {section}")))
        .expect("config inscriptible");
}

/// Le rapport rendu par le binaire livré.
///
/// L'assertion porte sur la ligne rendue et non sur le code de sortie : celui-ci vaut 1
/// dès qu'un contrôle échoue, quel qu'il soit, et ne dirait donc pas lequel. C'est la
/// ligne qui distingue le constat visé d'un autre échec du même rapport.
fn diagnostic(projet: &Path) -> String {
    let sortie = rbs(projet).arg("doctor").assert().get_output().clone();

    String::from_utf8_lossy(&sortie.stdout).into_owned()
}

/// La ligne du rapport que rend le contrôle `titre`.
fn ligne(rendu: &str, titre: &str) -> String {
    rendu
        .lines()
        .find(|ligne| ligne.contains(titre))
        .unwrap_or_else(|| panic!("aucune ligne « {titre} » dans :\n{rendu}"))
        .to_owned()
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
