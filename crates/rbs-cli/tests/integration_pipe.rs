//! Ce que le CLI fait quand son lecteur s'en va.
//!
//! `rbs doctor | head -3` referme le tube dès la troisième ligne : les macros d'affichage
//! paniquent sur cette fin-là, et la commande rendait une trace de panique au lieu des
//! trois lignes demandées.
//!
//! Le lecteur qui s'en va est ici un tube dont l'extrémité de lecture est refermée avant
//! que le binaire n'ait écrit : sa première écriture échoue, sans dépendre de la vitesse
//! d'un `head`. Ni conteneur ni compilation du projet engendré — le contrôle `base` vise
//! un port où rien n'écoute, et renonce aussitôt.

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

mod common;

/// Port réservé, où rien n'écoute : le contrôle `base` y renonce plutôt que d'attendre
/// un serveur que ce test n'a aucune raison d'avoir.
const INJOIGNABLE: &str = "postgres://rbs:rbs@127.0.0.1:1/demo_api";

/// Le script de complétion part par un générateur qui relève l'échec d'écriture par un
/// `expect` : c'est le puits qui ne passe par aucune macro d'affichage.
#[test]
fn completions_survive_a_reader_that_leaves() {
    let temporaire = TempDir::new().expect("répertoire temporaire créable");

    let sortie = sans_lecteur(temporaire.path(), &["completions", "bash"]);

    assert_sortie_propre(&sortie);
}

/// Le rapport de diagnostic mêle le rendu des constats, le plan de la réparation et sa
/// conclusion : les trois passent par la sortie standard, et une ancre retirée les fait
/// tous écrire sans qu'aucune base soit nécessaire.
#[test]
fn doctor_survives_a_reader_that_leaves() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);
    amputer_l_ancre_des_routes(&projet);

    let sortie = sans_lecteur(&projet, &["doctor", "--fix"]);

    assert_sortie_propre(&sortie);
}

/// Une commande qui écrit annonce son plan puis ce qu'elle a fait : c'est le chemin de
/// `ui`, celui que les deux lignes de conclusion de `doctor` empruntent aussi.
#[test]
fn add_survives_a_reader_that_leaves() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let sortie = sans_lecteur(&projet, &["add", "cors", "--force"]);

    assert_sortie_propre(&sortie);
}

/// Le document JSON sort d'une seule écriture, celle que `head -1` referme le plus tôt.
#[test]
fn the_json_report_survives_a_reader_that_leaves() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);

    let sortie = sans_lecteur(&projet, &["doctor", "--json"]);

    assert_sortie_propre(&sortie);
}

/// Lance `rbs` depuis `repertoire`, sortie standard branchée sur un tube dont l'autre
/// extrémité est refermée aussitôt.
fn sans_lecteur(repertoire: impl AsRef<Path>, arguments: &[&str]) -> Output {
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_rbs"))
        .args(arguments)
        .current_dir(repertoire)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire rbs doit être lançable");

    drop(enfant.stdout.take());

    enfant
        .wait_with_output()
        .expect("la commande doit se terminer")
}

/// Échoue si la commande a paniqué, sur le message comme sur le code de sortie.
///
/// `doctor` rend 1 sur un projet qui demande de l'attention : c'est 101, celui d'une
/// panique, qui est proscrit ici.
fn assert_sortie_propre(sortie: &Output) {
    let stderr = String::from_utf8_lossy(&sortie.stderr);

    assert!(
        !stderr.contains("panicked"),
        "la commande a paniqué sur un tube refermé :\n{stderr}"
    );
    assert!(
        matches!(sortie.status.code(), Some(0 | 1)),
        "code de sortie {:?} :\n{stderr}",
        sortie.status.code()
    );
}

/// Fait viser `url` au projet, comme les tests de diagnostic le font.
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

/// Retire l'ancre des routes, seule chose que `--fix` a alors à reposer.
fn amputer_l_ancre_des_routes(projet: &Path) {
    let routeur = projet.join("src/router.rs");
    let source = fs::read_to_string(&routeur).expect("le routeur est lisible");
    let ampute: String = source
        .lines()
        .filter(|ligne| !ligne.contains("rbs:routes>"))
        .map(|ligne| format!("{ligne}\n"))
        .collect();

    assert_ne!(source, ampute, "l'ancre des routes est introuvable");
    fs::write(&routeur, ampute).expect("le routeur est réécrivable");
}
