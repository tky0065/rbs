//! Les clés d'API d'un projet réel, jugées par un vrai PostgreSQL.
//!
//! C'est la seule chose qui prouve que le fragment fonctionne : ni sa délégation posée
//! dans `impl HasAuth`, ni le plafond de rôle qu'elle applique, ni la trace d'usage
//! qu'elle écrit ne disent quoi que ce soit tant qu'aucun projet ne les a joués. Le
//! fragment ne livre que des tests `#[ignore]` — chacun joint la base — d'où l'unique
//! portée sous conteneur, sans le second groupe que `webhooks` sépare pour ses tests sans
//! base : `api-keys` n'en livre aucun.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Ce que le fragment livre et qui joint la base, nommés avec le sous-module — `accept` ou
/// `routes` — où le découpage des tests l'a rangé.
///
/// `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans cette
/// liste, un fragment qui cesserait de livrer ses tests laisserait la suite au vert sans
/// qu'une seule transaction ait été ouverte.
const TESTS_SOUS_CONTENEUR: [(&str, &str); 11] = [
    ("accept", "a_valid_key_identifies_its_bearer"),
    ("accept", "a_key_of_a_demoted_owner_no_longer_administers"),
    ("accept", "a_user_cannot_mint_an_admin_key"),
    ("accept", "a_revoked_key_is_refused_like_an_unknown_one"),
    ("accept", "an_expired_key_is_refused_like_an_unknown_one"),
    ("accept", "three_close_calls_write_the_usage_trace_once"),
    (
        "accept",
        "accepting_a_key_leaves_the_verification_date_in_the_extensions",
    ),
    ("routes", "the_key_is_returned_once_and_never_by_the_list"),
    ("routes", "the_creation_response_is_never_cached"),
    ("routes", "revoking_a_key_of_another_account_answers_404"),
    ("routes", "a_key_opens_a_route_that_identity_guards"),
];

/// L'insertion tombe **à l'intérieur** du bloc `impl`, et non entre deux items : c'est la
/// seule chose que l'ancre `auth_impl` a de particulier, et la seule qui puisse casser.
#[test]
fn adding_the_fragment_writes_the_delegation_inside_the_auth_implementation() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "init");

    Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "api-keys"])
        .current_dir(&racine)
        .assert()
        .success();

    let module = fs::read_to_string(racine.join("src/auth/mod.rs")).expect("le module se lit");
    let ouverture = module
        .find("impl HasAuth for AppState {")
        .expect("l'impl est là");
    let fermeture = module[ouverture..].find("\n}").expect("l'impl se ferme") + ouverture;
    let delegation = module
        .find("crate::modules::api_keys::service::accept")
        .expect("la délégation est posée");
    assert!(
        ouverture < delegation && delegation < fermeture,
        "la délégation est tombée hors du bloc impl :\n{module}"
    );

    for (fichier, attendu) in [
        (
            "src/router.rs",
            ".merge(crate::modules::api_keys::routes())",
        ),
        (
            "src/openapi.rs",
            "crate::modules::api_keys::controller::create,",
        ),
        ("migration/src/lib.rs", "create_api_keys"),
    ] {
        let source = fs::read_to_string(racine.join(fichier)).expect("le fichier se lit");
        assert!(
            source.contains(attendu),
            "{fichier} ne porte pas `{attendu}`"
        );
    }

    // Le code de sortie de `doctor` vaut 1 dès qu'un contrôle échoue, quel qu'il soit — ici
    // `base` et `.env`, faute d'une vraie base à `localhost:5432` : ce que ce test vise est
    // la ligne `api-keys`, pas l'issue globale de la commande. `integration_doctor` le dit
    // déjà et l'assertion suit la même règle plutôt que le `.success()` d'un modèle qui ne
    // s'applique pas ici.
    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .arg("doctor")
        .current_dir(&racine)
        .assert()
        .get_output()
        .clone();
    let rendu = String::from_utf8_lossy(&sortie.stdout).into_owned();
    assert!(
        rendu
            .lines()
            .any(|ligne| ligne.contains("api-keys") && ligne.contains('✓')),
        "{rendu}"
    );
}

/// Un projet engendré avant que l'ancre n'existe : la commande n'écrit **rien** et montre
/// le bloc. C'est la convention du dépôt, et c'est le parc entier qui la reçoit.
#[test]
fn without_the_anchor_nothing_is_written_and_the_block_is_shown() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "init");

    // `add auth` d'abord : c'est lui qui pose le fichier porteur.
    Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "auth"])
        .current_dir(&racine)
        .assert()
        .success();

    let chemin = racine.join("src/auth/mod.rs");
    let ampute = fs::read_to_string(&chemin)
        .expect("le module se lit")
        .lines()
        .filter(|ligne| !ligne.trim().starts_with("// <rbs:auth_impl"))
        .filter(|ligne| !ligne.trim().starts_with("// </rbs:auth_impl"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&chemin, format!("{ampute}\n")).expect("le module se réécrit");
    common::commiter(&racine, "sans l'ancre");

    let avant = common::empreinte(&racine);

    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "api-keys"])
        .current_dir(&racine)
        .assert()
        .failure();
    let sortie = sortie.get_output();
    let erreur = String::from_utf8_lossy(&sortie.stderr).into_owned();
    let document = String::from_utf8_lossy(&sortie.stdout).into_owned();

    // `ui::error` écrit sur la sortie d'erreur, `ui::info` sur la sortie standard : le
    // constat et le bloc à coller vivent sur deux flux distincts, comme le prouve déjà
    // `integration_add::the_task_criterion...` sur l'ancre `state_champs`.
    //
    // Le bloc à coller ici est celui de `plan::Error::Anchor`, générique à toute ancre :
    // il rétablit les deux balises, pas le contenu qu'un fragment y aurait posé — la
    // délégation elle-même ne réapparaît que si `add api-keys` est relancée après. C'est
    // `rbs doctor` qui porte le remède complet, avec la méthode entière à coller ; le
    // remède de `add`, lui, ne connaît que l'ancre.
    assert!(
        erreur.contains("<rbs:auth_impl>"),
        "l'erreur doit nommer l'ancre :\n{erreur}"
    );
    assert!(
        document.contains("// <rbs:auth_impl>") && document.contains("// </rbs:auth_impl>"),
        "le bloc à coller doit montrer les deux balises :\n{document}"
    );
    common::assert_intact(
        &avant,
        &racine,
        "une ancre absente n'autorise aucune écriture",
    );
}

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_api_keys_on(&common::url_of(&postgres), &parent);

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    migrate_dans(&racine, &common::cible());

    // La passe est filtrée sur les tests du fragment : le projet engendré porte aussi
    // `mail`, entraîné par `auth`, dont un test réclame un serveur SMTP que cette suite ne
    // monte pas. Le monter dupliquerait `integration_mail`, qui démarre déjà un Mailpit
    // pour lui seul.
    let (abouti, sous_conteneur) = cargo_test_brut(
        &racine,
        &common::cible(),
        &["modules::api_keys::", "--", "--ignored"],
    );
    assert!(
        abouti,
        "`cargo test -- --ignored` du fragment a échoué :\n{sous_conteneur}"
    );

    for (sous_module, test) in TESTS_SOUS_CONTENEUR {
        assert!(
            sous_conteneur.contains(&format!(
                "test modules::api_keys::tests::{sous_module}::{test} ... ok"
            )),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}

/// Un projet neuf portant `api-keys`, sa base pointée sur `url`.
///
/// Le fragment déclare `requires = ["auth"]`, et `auth` exige à son tour `rate-limit` et
/// `mail` : `rbs add api-keys` sur un projet nu doit poser les trois **puis** les clés
/// d'API.
fn project_with_api_keys_on(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            url,
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "api-keys"]).assert().success();

    for requis in [
        "src/auth/mod.rs",
        "src/modules/rate_limit/mod.rs",
        "src/modules/mail/mod.rs",
    ] {
        assert!(
            racine.join(requis).exists(),
            "le fragment requis n'a pas été entraîné : {requis}"
        );
    }
    assert!(racine.join("src/modules/api_keys/mod.rs").exists());

    racine
}

fn migrate_dans(racine: &Path, cible: &Path) {
    rbs(racine)
        .env("CARGO_TARGET_DIR", cible)
        .args(["migrate", "up"])
        .assert()
        .success();
}

/// Joue `cargo test` dans le projet et rend son issue et ses deux flux réunis.
fn cargo_test_brut(racine: &Path, cible: &Path, arguments: &[&str]) -> (bool, String) {
    let output = std::process::Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", cible)
        .arg("test")
        .arg("--workspace")
        .args(arguments)
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (output.status.success(), journal)
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Le fragment ne dépose aucun fichier que le projet ne monterait pas.
///
/// Ce test-ci ne demande ni Docker ni compilation : il tourne sur chaque PR, là où l'autre
/// attend qu'on le réclame.
#[test]
fn every_file_the_fragment_ships_is_declared_in_its_manifest() {
    let racine = common::depot().join("crates/rbs-cli/templates/features/api-keys");
    let manifeste = fs::read_to_string(racine.join("feature.toml")).expect("manifeste lisible");

    for entree in fs::read_dir(&racine).expect("le fragment se lit") {
        let nom = entree
            .expect("entrée lisible")
            .file_name()
            .to_string_lossy()
            .into_owned();

        if nom == "feature.toml" {
            continue;
        }

        assert!(
            manifeste.contains(&nom),
            "`{nom}` est livrée sans être déclarée dans feature.toml"
        );
    }
}
