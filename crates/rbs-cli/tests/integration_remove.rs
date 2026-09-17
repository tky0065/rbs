//! Ce que `rbs remove` garantit, éprouvé par la commande telle que l'utilisateur la lance.
//!
//! Un test unitaire prouve que le moteur sait faire ; celui-ci prouve que le projet
//! survit — c'est le seul de la séquence à le faire. Deux de ses cinq tests sont
//! `#[ignore]` : ils compilent le projet engendré, ce qu'aucune PR ne peut se payer sur
//! chaque poussée. Les trois autres n'appellent ni cargo ni Docker, donc tournent à chaque
//! PR : `a_dry_run_writes_nothing`, même précédent que
//! `adding_auth_to_a_sqlite_project_succeeds_and_names_the_service_left_to_mount` dans
//! `integration_add.rs`, `a_file_the_developer_changed_stops_the_command`, aussi bon
//! marché malgré l'étiquette qu'il portait à tort, et
//! `the_directory_of_a_removed_fragment_does_not_survive_it`, que seul `auth` met à
//! l'épreuve.
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

    // Le répertoire entier, et non ses trois fichiers un à un : l'application purge le
    // parent que la dernière suppression vide. Le point de montage, lui, garde son
    // `mod.rs` et reste — la remontée s'arrête au premier répertoire non vide.
    assert!(
        !racine.join("src/modules/cors").exists(),
        "le répertoire du fragment doit partir avec ses fichiers"
    );
    assert!(
        racine.join("src/modules/mod.rs").is_file(),
        "src/modules/ garde son point de montage"
    );
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

/// Le répertoire d'un fragment retiré ne survit pas au retrait.
///
/// `cors` ne le prouverait pas : il s'installe sous `src/modules/`, dont le `mod.rs`
/// survit et garde le répertoire non vide. `auth` est le seul fragment qui s'installe
/// ailleurs, sous `src/auth/`, et c'est lui qui mord : laissé vide, il fait dire à
/// `rbs doctor` « écrit hors du CLI : auth » sur un projet qui vient pourtant de subir un
/// retrait propre. N'appelle ni cargo ni Docker.
#[test]
fn the_directory_of_a_removed_fragment_does_not_survive_it() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "auth"]).assert().success();
    common::commiter(&racine, "auth posée");
    assert!(
        racine.join("src/auth").is_dir(),
        "`auth` s'installe bien hors de src/modules/"
    );

    rbs(&racine).args(["remove", "auth"]).assert().success();

    assert!(
        !racine.join("src/auth").exists(),
        "src/auth survit vide : `rbs doctor` le nommerait « écrit hors du CLI »"
    );
}

/// L'aller-retour rend un projet identique, aux horodatages de migration près — le critère
/// de la section 8 de la spec — et qui compile encore.
///
/// Deux fragments plutôt qu'un : `cors` n'a pas de migration, `jobs` en a une, et c'est
/// elle qui met à l'épreuve la seule exception que la spec accorde.
///
/// Chaque étape est commitée : `remove`, comme `add`, refuse un working tree sale, et
/// laisser le résultat d'une pose non commitée ferait échouer le retrait qui la suit — pas
/// sur la garde que ce test éprouve, mais sur celle, plus générale, qui protège toute
/// commande touchant au projet.
#[test]
#[ignore = "compile le projet engendré"]
fn the_round_trip_leaves_an_identical_project_that_still_compiles() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    for feature in ["cors", "jobs"] {
        rbs(&racine).args(["add", feature]).assert().success();
        common::commiter(&racine, "feature posée");

        let avant = empreinte_datee(&racine);

        rbs(&racine).args(["remove", feature]).assert().success();
        common::commiter(&racine, "feature retirée");
        rbs(&racine).args(["add", feature]).assert().success();

        assert_identique(&avant, &empreinte_datee(&racine), feature);
        common::commiter(&racine, "feature reposée");
    }

    let _cible = common::verrou(&common::cible());
    cargo_check(&racine).assert().success();
}

/// L'empreinte du projet, horodatages de migration masqués.
///
/// `rbs add` date la migration qu'il pose de l'instant où il la pose : son nom de fichier
/// et la ligne qui l'enregistre dans `migration/src/lib.rs` diffèrent d'une pose à l'autre,
/// et c'est la seule différence que la spec accorde.
fn empreinte_datee(racine: &Path) -> common::Empreinte {
    common::empreinte(racine)
        .into_iter()
        .map(|(chemin, contenu)| {
            (
                masque_horodatage(&chemin.to_string_lossy()).into(),
                masque_horodatage(&contenu),
            )
        })
        .collect()
}

/// `m20260917_141948` → `m<horodatage>`.
fn masque_horodatage(texte: &str) -> String {
    let lettres: Vec<char> = texte.chars().collect();
    let chiffres = |debut: usize, combien: usize| {
        debut + combien <= lettres.len()
            && lettres[debut..debut + combien]
                .iter()
                .all(char::is_ascii_digit)
    };

    let mut rendu = String::with_capacity(texte.len());
    let mut rang = 0;
    while rang < lettres.len() {
        if lettres[rang] == 'm'
            && chiffres(rang + 1, 8)
            && lettres.get(rang + 9) == Some(&'_')
            && chiffres(rang + 10, 6)
        {
            rendu.push_str("m<horodatage>");
            rang += 16;
            continue;
        }

        rendu.push(lettres[rang]);
        rang += 1;
    }

    rendu
}

/// Échoue si les deux empreintes diffèrent, en ne montrant que ce qui diffère.
fn assert_identique(avant: &common::Empreinte, apres: &common::Empreinte, feature: &str) {
    let mut ecarts = Vec::new();

    for (chemin, contenu) in avant {
        match apres.get(chemin) {
            None => ecarts.push(format!("  - {} a disparu", chemin.display())),
            Some(actuel) if actuel != contenu => {
                ecarts.push(format!("  ~ {} a changé", chemin.display()));
            }
            Some(_) => {}
        }
    }
    for chemin in apres.keys() {
        if !avant.contains_key(chemin) {
            ecarts.push(format!("  + {} est apparu", chemin.display()));
        }
    }

    assert!(
        ecarts.is_empty(),
        "l'aller-retour de {feature} n'a pas rendu le projet identique :\n{}",
        ecarts.join("\n")
    );
}
