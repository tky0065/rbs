//! Ce que `rbs remove` garantit, éprouvé par la commande telle que l'utilisateur la lance.
//!
//! Un test unitaire prouve que le moteur sait faire ; celui-ci prouve que le projet
//! survit — c'est le seul de la séquence à le faire. Deux de ses quatre tests sont
//! `#[ignore]` : ils compilent le projet engendré, ce qu'aucune PR ne peut se payer sur
//! chaque poussée. Les deux autres n'appellent ni cargo ni Docker, donc tournent à chaque
//! PR : `a_dry_run_writes_nothing`, même précédent que
//! `adding_auth_to_a_sqlite_project_succeeds_and_names_the_service_left_to_mount` dans
//! `integration_add.rs`, et `a_file_the_developer_changed_stops_the_command`, aussi bon
//! marché malgré l'étiquette qu'il portait à tort.
//!
//! `a_project_still_compiles_and_stays_diagnosable_once_the_fragment_is_removed` va plus
//! loin que son nom initial : une revue a établi qu'un retrait pourtant propre faisait
//! échouer `rbs doctor`, la feature quittant le manifeste sans que l'inventaire
//! d'`AGENTS.md` soit rafraîchi. C'est corrigé, mais ce test est le seul témoin permanent
//! possible — sans lui, la garde se redégraderait en silence, comme trois autres avant
//! elle dans cette séquence. Prouver un succès de `doctor` exige une base réellement
//! joignable : ce test démarre donc son propre PostgreSQL, plutôt que de se contenter de
//! l'URL de convenance qu'écrit `common::projet`.

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Le binaire livré, lancé depuis la racine d'un projet.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// Compile le projet engendré sans lancer ses tests : ce que ces scénarios prouvent —
/// qu'un retrait laisse un projet qui compile — n'exige pas la base que `cargo test`
/// réclamerait, ni le temps qu'il prendrait.
fn cargo_check(racine: &Path) -> Command {
    let mut commande = Command::new("cargo");
    commande
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["check", "--workspace"]);
    commande
}

/// Pointe le `.env` du projet vers une base réellement joignable.
///
/// `common::projet` fige une URL de convenance (`localhost:5432`), qu'aucun serveur
/// n'écoute ici : le contrôle `base` de `rbs doctor` interroge la base pour de vrai, et
/// échouerait sans cette bascule vers le conteneur que le test démarre.
fn pointer_vers(racine: &Path, url: &str) {
    let env = racine.join(".env");
    let source = std::fs::read_to_string(&env).expect("le .env se lit");
    let reecrit: String = source
        .lines()
        .map(|ligne| match ligne.starts_with("RBS_DATABASE__URL=") {
            true => format!("RBS_DATABASE__URL={url}\n"),
            false => format!("{ligne}\n"),
        })
        .collect();

    assert!(
        reecrit.contains(url),
        "le .env ne porte pas RBS_DATABASE__URL"
    );
    std::fs::write(&env, reecrit).expect("le .env s'écrit");
}

/// `--dry-run` calcule le plan et n'écrit rien : `add` avait déjà ce témoin dans
/// `integration_add.rs`, `remove` non — c'est la correction R1.
#[test]
fn a_dry_run_writes_nothing() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    let avant = common::empreinte(&racine);
    rbs(&racine)
        .args(["remove", "cors", "--dry-run"])
        .assert()
        .success();

    common::assert_intact(&avant, &racine, "un --dry-run a écrit dans le projet");
}

/// Un projet neuf, `cors` posée, puis retirée : il doit compiler aux deux bouts, et se
/// diagnostiquer sans faute — la garde R12.
#[test]
#[ignore = "démarre PostgreSQL et compile le projet engendré"]
fn a_project_still_compiles_and_stays_diagnosable_once_the_fragment_is_removed() {
    let postgres = common::start_postgres();

    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    pointer_vers(&racine, &common::url_of(&postgres));
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    rbs(&racine).args(["remove", "cors"]).assert().success();

    // Un par un, et non le répertoire `src/modules/cors` entier : celui-ci reste sur le
    // disque, vide — `Applicateur::write` supprime les fichiers d'un plan de retrait mais
    // ne remonte jamais purger un parent devenu vide. Git ne suit pas les répertoires
    // vides et la compilation n'en dépend pas, donc rien ici ne le prouverait à tort ;
    // c'est un manque réel, distinct de ce que R12 corrige, à consigner pour la revue.
    for fichier in ["mod.rs", "config.rs", "tests.rs"] {
        assert!(
            !racine.join("src/modules/cors").join(fichier).exists(),
            "{fichier} doit partir"
        );
    }
    assert!(
        !std::fs::read_to_string(racine.join("src/router.rs"))
            .expect("le routeur se lit")
            .contains("cors::layer"),
        "la ligne d'ancre doit partir"
    );

    // La cible de compilation est partagée par tous les binaires de `tests/` : le garde
    // se prend avant le premier cargo et se tient jusqu'au dernier, `rbs doctor` compris —
    // son contrôle `base` compile la crate `migration` sur la même cible.
    let _cible = common::verrou(&common::cible());

    cargo_check(&racine).assert().success();

    let diagnostic = rbs(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("doctor")
        .output()
        .expect("le binaire tourne");

    assert!(
        diagnostic.status.success(),
        "`rbs doctor` doit réussir sur un projet qui vient de subir un retrait propre :\n{}",
        String::from_utf8_lossy(&diagnostic.stdout)
    );
}

/// Un fichier que le développeur a modifié arrête la commande, et rien n'est écrit.
///
/// N'appelle ni cargo ni Docker : aussi bon marché que `a_dry_run_writes_nothing`, il
/// tourne à chaque PR.
#[test]
fn a_file_the_developer_changed_stops_the_command() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    let touche = racine.join("src/modules/cors/config.rs");
    let source = std::fs::read_to_string(&touche).expect("le fichier se lit");
    std::fs::write(&touche, format!("{source}// une ligne à moi\n")).expect("le fichier s'écrit");
    common::commiter(&racine, "retouche du développeur");

    let avant = common::empreinte(&racine);
    let sortie = rbs(&racine)
        .args(["remove", "cors"])
        .output()
        .expect("le binaire tourne");

    assert!(
        !sortie.status.success(),
        "la divergence doit arrêter la commande"
    );
    common::assert_intact(&avant, &racine, "un refus ne doit rien écrire");

    rbs(&racine)
        .args(["remove", "cors", "--force"])
        .assert()
        .success();
}

/// L'aller-retour rend un projet qui compile encore.
///
/// Chaque étape est commitée : `remove`, comme `add`, refuse un working tree sale, et
/// laisser le résultat d'une pose non commitée ferait échouer le retrait qui la suit — pas
/// sur la garde que ce test éprouve, mais sur celle, plus générale, qui protège toute
/// commande touchant au projet.
#[test]
#[ignore = "compile le projet engendré"]
fn the_round_trip_leaves_a_project_that_still_compiles() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    rbs(&racine).args(["remove", "cors"]).assert().success();
    common::commiter(&racine, "cors retirée");

    rbs(&racine).args(["add", "cors"]).assert().success();

    let _cible = common::verrou(&common::cible());
    cargo_check(&racine).assert().success();
}
