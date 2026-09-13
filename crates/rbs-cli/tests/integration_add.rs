//! Les quatre garanties du mécanisme `add`, éprouvées par la commande telle que
//! l'utilisateur la lance.
//!
//! E2 à E6 les ont prouvées au niveau de leur module, sur des répertoires construits à la
//! main. Ce n'est pas une redite : un test unitaire prouve que le moteur sait faire, un
//! test d'intégration prouve que la commande le fait — argument parsé, garde appliquée,
//! plan affiché, code de sortie compris.
//!
//! Le scénario de l'ancre disparue passe par `rbs generate crud` : `rbs add` n'écrit dans
//! aucune ancre, ses deux features n'apportant pas de code Rust. Les deux commandes
//! partagent `plan::Constructeur::inserer`, qui est ce que le scénario éprouve.
//!
//! Aucun `#[ignore]` : ces tests ne compilent pas le projet généré et n'ont pas besoin de
//! Docker. Ils doivent tourner sur chaque PR, sans quoi ils ne garantissent rien.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Le binaire livré, lancé depuis la racine d'un projet.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// Un projet neuf dont le working tree est propre.
fn committed_project(parent: &TempDir) -> PathBuf {
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");
    racine
}

/// Ce qu'une exécution du binaire a produit : code, sortie standard, sortie d'erreur.
struct Sortie {
    succes: bool,
    stdout: String,
    stderr: String,
}

impl Sortie {
    fn de(commande: &mut Command) -> Self {
        let output = commande.output().expect("le binaire doit être lançable");

        Self {
            succes: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// `add webhooks` pose d'un coup les migrations de `jobs`, `auth` et `webhooks`, sous un
/// même horodatage. Déclarées dans l'ordre d'installation, elles laissaient à
/// `migration/src/lib.rs` un ordre que rustfmt réécrit : `cargo fmt --check` échouait, et
/// avec lui la CI qu'engendre `rbs add ci`.
#[test]
fn adding_webhooks_leaves_a_project_that_cargo_fmt_accepts() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    rbs(&racine).args(["add", "webhooks"]).assert().success();

    let fmt = std::process::Command::new("cargo")
        .current_dir(&racine)
        .args(["fmt", "--all", "--check"])
        .output()
        .expect("cargo fmt doit être lançable");

    assert!(
        fmt.status.success(),
        "`cargo fmt --all --check` reformate le projet après `add webhooks` :\n{}",
        String::from_utf8_lossy(&fmt.stdout)
    );
}

/// Installer deux fois la même feature laisse le projet exactement là où la première
/// installation l'avait laissé.
#[test]
fn installing_the_same_feature_twice_produces_nothing_the_second_time() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    rbs(&racine).args(["add", "docker"]).assert().success();
    common::commiter(&racine, "docker");

    let avant = common::empreinte(&racine);
    let seconde = Sortie::de(rbs(&racine).args(["add", "docker"]));

    assert!(
        seconde.succes,
        "la seconde installation doit aboutir sans rien faire :\n{}",
        seconde.stderr
    );
    common::assert_intact(
        &avant,
        &racine,
        "la seconde installation a modifié le projet",
    );
}

/// Une ancre retirée par le développeur arrête la commande avant toute écriture, et le
/// bloc à recoller est affiché plutôt que deviné.
#[test]
fn a_deleted_anchor_refuses_the_write_and_prints_the_block_to_paste() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    let router = racine.join("src").join("router.rs");
    let ampute: Vec<String> = fs::read_to_string(&router)
        .expect("router.rs lisible")
        .lines()
        .filter(|ligne| !ligne.contains("// <rbs:routes>"))
        .map(str::to_string)
        .collect();
    fs::write(&router, ampute.join("\n")).expect("router.rs inscriptible");

    let avant = common::empreinte(&racine);
    let output = Sortie::de(rbs(&racine).args([
        "g",
        "crud",
        "notes",
        "--fields",
        "titre:string",
        "--force",
    ]));

    assert!(
        !output.succes,
        "la génération a abouti malgré l'ancre absente :\n{}",
        output.stdout
    );
    assert!(
        output.stderr.contains("<rbs:routes>") && output.stderr.contains("src/router.rs"),
        "l'erreur doit nommer l'ancre et son fichier :\n{}",
        output.stderr
    );
    assert!(
        output.stdout.contains("// <rbs:routes>") && output.stdout.contains("// </rbs:routes>"),
        "le bloc à recoller doit porter les deux balises :\n{}",
        output.stdout
    );
    common::assert_intact(
        &avant,
        &racine,
        "l'ancre absente n'a pas empêché l'écriture",
    );
}

/// Un projet porteur de modifications non commitées refuse l'installation, qui rendrait
/// les siennes indiscernables — sauf si le développeur passe outre.
#[test]
fn a_dirty_working_tree_refuses_without_force_and_passes_with_it() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    let main = racine.join("src").join("main.rs");
    let salissure = format!(
        "{}\n// modification non commitée\n",
        fs::read_to_string(&main).expect("main.rs lisible")
    );
    fs::write(&main, salissure).expect("main.rs inscriptible");

    let avant = common::empreinte(&racine);
    let refus = Sortie::de(rbs(&racine).args(["add", "docker"]));

    assert!(
        !refus.succes,
        "l'installation a abouti sur un projet sale :\n{}",
        refus.stdout
    );
    assert!(
        refus.stderr.contains("working tree") && refus.stderr.contains("src/main.rs"),
        "le refus doit nommer ce qui est modifié :\n{}",
        refus.stderr
    );
    common::assert_intact(&avant, &racine, "le refus a tout de même écrit");

    rbs(&racine)
        .args(["add", "docker", "--force"])
        .assert()
        .success();

    assert!(
        racine.join("Dockerfile").exists(),
        "`--force` n'a pas installé la feature"
    );
}

/// Un échec au milieu de l'application ne laisse pas un projet à moitié modifié.
///
/// L'échec n'est pas injecté : `docker-compose.yml` porte son ancre — la planification
/// calcule donc l'insertion avec succès — mais le fichier est posé en lecture seule, et
/// c'est l'écriture qui échoue. `Permissions::set_readonly` vaut sur Unix comme sur
/// Windows. Le plan écrit `Dockerfile` et `.dockerignore` avant de buter sur
/// `docker-compose.yml`, troisième fichier du plan : c'est leur retrait qui prouve la
/// restauration.
#[test]
fn a_failure_during_application_restores_the_files_already_written() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    let piege = racine.join("docker-compose.yml");
    fs::write(
        &piege,
        "services:\n  # <rbs:services>\n  # </rbs:services>\n",
    )
    .expect("piège inscriptible");
    let mut permissions = fs::metadata(&piege).expect("piège lisible").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&piege, permissions).expect("permissions modifiables");

    assert!(
        fs::write(&piege, "vérification").is_err(),
        "le piège n'a pas pris : ce test ne prouverait rien. Sous un compte privilégié, la \
         lecture seule est sans effet."
    );

    common::commiter(&racine, "piège");
    let avant = common::empreinte(&racine);

    let output = Sortie::de(rbs(&racine).args(["add", "docker"]));

    assert!(
        !output.succes,
        "l'installation a abouti malgré un fichier non inscriptible :\n{}",
        output.stdout
    );
    assert!(
        !racine.join("Dockerfile").exists(),
        "le Dockerfile écrit avant l'échec n'a pas été retiré"
    );
    common::assert_intact(&avant, &racine, "l'échec a laissé le projet modifié");
}

/// Un fragment qui apporte du code Rust, fabriqué pour le test.
///
/// Aucune feature livrée n'en apporte encore : le moule ne s'éprouve que sur un fragment
/// qui exerce les sept sections du manifeste — fichiers, ancres, migration, dépendances
/// tierces, feature Cargo, section de configuration, variable d'environnement.
fn fragment_has_code() -> TempDir {
    let repertoire = TempDir::new().expect("répertoire temporaire créable");
    let essai = repertoire.path().join("essai");
    fs::create_dir(&essai).expect("le fragment se crée");

    fs::write(
        essai.join("feature.toml"),
        "[feature]\ndescription = \"un fragment de test\"\n\n\
         [[files]]\nsource = \"mod.rs.jinja\"\ndestination = \"src/essai/mod.rs\"\n\n\
         [[files]]\nsource = \"service.rs.jinja\"\ndestination = \"src/essai/service.rs\"\n\n\
         [[anchors]]\nanchor = \"features\"\ncontent = \"mod essai;\"\n\n\
         [[anchors]]\nanchor = \"routes\"\ncontent = \".merge(crate::essai::routes())\"\n\n\
         [[anchors]]\nanchor = \"state_champs\"\ncontent = \"essai: crate::essai::Client,\"\n\n\
         [[anchors]]\nanchor = \"state_init\"\ncontent = \"essai: crate::essai::client()?,\"\n\n\
         [migration]\nsource = \"table.rs.jinja\"\nname = \"create_essais\"\n\n\
         [[dependencies]]\nname = \"lettre\"\nversion = \"0.11\"\n\
         default_features = false\nfeatures = [\"smtp-transport\", \"builder\"]\n\n\
         [[dependencies]]\nname = \"axum\"\nversion = \"0.8\"\n\n\
         [cargo.rbs-core]\nfeatures = [\"auth\"]\n\n\
         [[config]]\nfile = \"config/default.toml\"\nsection = \"essai\"\n\
         content = \"\"\"\nttl_secs = 900\n\"\"\"\n\n\
         [[env]]\nkey = \"RBS_ESSAI__SECRET\"\nvalue = \"changez-moi\"\n\
         comment = \"au moins 32 octets\"\n",
    )
    .expect("le manifeste s'écrit");

    for (nom, contenu) in [
        ("mod.rs.jinja", "// {@ crate_name @}\npub mod service;\n"),
        ("service.rs.jinja", "pub fn rien() {}\n"),
        ("table.rs.jinja", "// la table des essais\n"),
    ] {
        fs::write(essai.join(nom), contenu).expect("la template s'écrit");
    }

    repertoire
}

/// `rbs add essai`, servi par le fragment de test.
fn add_trial(racine: &Path, fragments: &TempDir, arguments: &[&str]) -> Sortie {
    let mut commande = rbs(racine);
    commande
        .args(["add", "essai"])
        .arg("--template-dir")
        .arg(fragments.path())
        .args(arguments);

    Sortie::de(&mut commande)
}

/// Le critère de la tâche, éprouvé par la commande telle que l'utilisateur la lance :
/// la crate que le fragment déclare arrive dans le `Cargo.toml` du projet, et celle que
/// le projet portait déjà n'y arrive pas deux fois.
#[test]
fn the_fragment_dependencies_reach_the_project_manifest() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let output = add_trial(&racine, &fragments, &[]);
    assert!(
        output.succes,
        "l'installation doit aboutir :\n{}",
        output.stderr
    );

    let manifeste =
        fs::read_to_string(racine.join("Cargo.toml")).expect("le manifeste est lisible");

    assert!(
        manifeste.contains(
            "lettre = { version = \"0.11\", default-features = false, \
             features = [\"smtp-transport\", \"builder\"] }"
        ),
        "{manifeste}"
    );
    assert_eq!(
        manifeste
            .lines()
            .filter(|ligne| ligne.starts_with("axum"))
            .count(),
        1,
        "`axum`, que le squelette déclare déjà, a été redéclarée :\n{manifeste}"
    );
}

/// Le critère de la tâche : deux ancres et non une, un champ se déclarant dans la struct
/// et s'initialisant dans `new`.
#[test]
fn both_state_anchors_receive_the_declared_content() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let output = add_trial(&racine, &fragments, &[]);
    assert!(
        output.succes,
        "l'installation doit aboutir :\n{}",
        output.stderr
    );

    let state = fs::read_to_string(racine.join("src/state.rs")).expect("state.rs est lisible");

    for (anchor, ligne) in [
        ("state_champs", "essai: crate::essai::Client,"),
        ("state_init", "essai: crate::essai::client()?,"),
    ] {
        let ouverture = format!("// <rbs:{anchor}>");
        let fermeture = format!("// </rbs:{anchor}>");
        let debut = state
            .find(&ouverture)
            .unwrap_or_else(|| panic!("state.rs ne porte pas `{ouverture}` :\n{state}"))
            + ouverture.len();
        let fin = state
            .find(&fermeture)
            .unwrap_or_else(|| panic!("state.rs ne porte pas `{fermeture}` :\n{state}"));

        assert!(
            state[debut..fin].contains(ligne),
            "l'ancre `{anchor}` ne porte pas `{ligne}` :\n{state}"
        );
    }
}

/// Le critère de la tâche : un projet créé avant ce lot n'est pas cassé en silence.
///
/// Le développeur a pu réécrire son `state.rs`, et le CLI ne sait qu'insérer dans une
/// ancre : faute de la trouver, il rend le bloc à coller et n'écrit rien.
#[test]
fn a_missing_state_anchor_stops_the_install_without_writing_anything() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let state = racine.join("src/state.rs");
    let source = fs::read_to_string(&state).expect("state.rs est lisible");
    assert!(
        source.contains("rbs:state_champs"),
        "l'ancre visée doit exister avant d'être retirée, sans quoi le test ne prouve rien"
    );
    let ampute = source
        .lines()
        .filter(|ligne| !ligne.contains("rbs:state_champs"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&state, format!("{ampute}\n")).expect("state.rs s'écrit");
    common::commiter(&racine, "état sans son ancre");

    let avant = common::empreinte(&racine);
    let output = add_trial(&racine, &fragments, &[]);

    assert!(
        !output.succes,
        "l'installation doit sortir en erreur :\n{}",
        output.stdout
    );
    assert!(
        output.stderr.contains("<rbs:state_champs>") && output.stderr.contains("src/state.rs"),
        "l'erreur doit nommer l'ancre et son fichier :\n{}",
        output.stderr
    );
    assert!(
        output.stdout.contains("// <rbs:state_champs>")
            && output.stdout.contains("// </rbs:state_champs>"),
        "le bloc à coller doit être affiché :\n{}",
        output.stdout
    );
    common::assert_intact(
        &avant,
        &racine,
        "l'ancre absente n'a pas empêché l'écriture",
    );
}

/// Un projet créé avant la 1.5.0 porte `// <rbs:state_init>` sous
/// `core: CoreState::new(db, config)`, qui a déjà consommé `config` quand `webhooks` la
/// lit : le projet ne compilerait plus, et seul `cargo build` le dirait. La commande
/// refuse avant d'écrire, nomme la ligne et affiche le bloc à remonter.
#[test]
fn webhooks_on_a_project_whose_state_init_anchor_follows_core_refuses_and_shows_the_block() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    let state = racine.join("src/state.rs");
    let source = fs::read_to_string(&state).expect("state.rs est lisible");
    let bloc = "            // <rbs:state_init>\n            // </rbs:state_init>\n";
    let core = "            core: CoreState::new(db, config),\n";
    let courant = format!("{bloc}{core}");
    assert!(
        source.contains(&courant),
        "le squelette doit poser l'ancre juste au-dessus de `core:`, sans quoi le test ne \
         prouve rien :\n{source}"
    );
    fs::write(&state, source.replace(&courant, &format!("{core}{bloc}")))
        .expect("state.rs s'écrit");
    common::commiter(&racine, "état d'avant 1.5.0");

    let avant = common::empreinte(&racine);
    let output = Sortie::de(rbs(&racine).args(["add", "webhooks"]));

    assert!(
        !output.succes,
        "l'installation doit sortir en erreur :\n{}",
        output.stdout
    );
    assert!(
        output.stderr.contains(
            "ancre // <rbs:state_init> placée sous `core: CoreState::new(` dans src/state.rs"
        ),
        "l'erreur doit nommer l'ancre, la ligne et le fichier :\n{}",
        output.stderr
    );
    assert!(
        output.stdout.contains(
            "remontez ce bloc au-dessus de `core: CoreState::new(` :\n// <rbs:state_init>\n\
             // </rbs:state_init>"
        ),
        "le bloc à remonter doit être affiché :\n{}",
        output.stdout
    );
    common::assert_intact(
        &avant,
        &racine,
        "l'ancre mal placée n'a pas empêché l'écriture",
    );
}

/// L'installation d'un fragment à code Rust ne se rejoue pas.
///
/// La vérification porte sur `[package.metadata.rbs]` et non sur la présence des
/// fichiers : la migration du fragment est horodatée, et une seconde installation qui se
/// fierait aux fichiers en déposerait une seconde, à un instant différent.
#[test]
fn two_successive_installs_write_nothing_the_second_time() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let premiere = add_trial(&racine, &fragments, &[]);
    assert!(
        premiere.succes,
        "la première installation doit aboutir :\n{}",
        premiere.stderr
    );
    common::commiter(&racine, "essai");

    let avant = common::empreinte(&racine);
    let seconde = add_trial(&racine, &fragments, &[]);

    assert!(
        seconde.succes,
        "la seconde installation doit aboutir sans rien faire :\n{}",
        seconde.stderr
    );
    common::assert_intact(
        &avant,
        &racine,
        "la seconde installation a modifié le projet",
    );
}

/// Un fichier installé puis supprimé par le développeur ne fait pas réinstaller la
/// feature à moitié : elle reste inscrite, et rien n'est réécrit.
#[test]
fn a_deleted_file_does_not_make_the_feature_reinstall() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    assert!(add_trial(&racine, &fragments, &[]).succes);
    fs::remove_file(racine.join("src/essai/service.rs")).expect("le fichier se supprime");
    common::commiter(&racine, "essai, sans son service");

    let avant = common::empreinte(&racine);
    let seconde = add_trial(&racine, &fragments, &[]);

    assert!(
        seconde.succes,
        "la commande doit aboutir sans rien faire :\n{}",
        seconde.stderr
    );
    common::assert_intact(&avant, &racine, "la feature s'est réinstallée à moitié");
}

/// Un échec au milieu de l'installation d'un fragment à code ne laisse rien derrière.
///
/// Le piège est le même qu'à l'installation de `docker` : le second fichier du plan est
/// posé en lecture seule, et l'écriture y échoue pour de vrai.
#[test]
fn a_failure_midway_restores_the_files_already_written() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let piege = racine.join("src/essai/service.rs");
    fs::create_dir_all(piege.parent().expect("le parent existe")).expect("répertoire créable");
    fs::write(&piege, "// posé par le test, en lecture seule\n").expect("piège inscriptible");
    let mut permissions = fs::metadata(&piege).expect("piège lisible").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&piege, permissions).expect("permissions modifiables");

    assert!(
        fs::write(&piege, "vérification").is_err(),
        "le piège n'a pas pris : ce test ne prouverait rien. Sous un compte privilégié, la \
         lecture seule est sans effet."
    );

    common::commiter(&racine, "piège");
    let avant = common::empreinte(&racine);

    // `--force` dépasse la garde du conflit, que le piège déclenche : ce qui est éprouvé
    // ici est l'échec d'écriture qui vient après, pas la garde qui l'aurait devancé.
    let output = add_trial(&racine, &fragments, &["--force"]);

    assert!(
        !output.succes,
        "l'installation a abouti malgré un fichier non inscriptible :\n{}",
        output.stdout
    );
    common::assert_intact(&avant, &racine, "l'échec a laissé le projet modifié");
}

/// Une ancre que le projet ne porte plus arrête l'installation avant toute écriture.
///
/// Le développeur a pu réécrire son routeur, et le CLI ne sait qu'insérer dans une ancre :
/// faute de la trouver, il rend le bloc à coller et n'écrit rien. Un fragment à demi
/// installé coûterait plus cher à défaire qu'à installer.
#[test]
fn a_missing_anchor_stops_the_install_without_writing_anything() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);
    let fragments = fragment_has_code();

    let router = racine.join("src/router.rs");
    let source = fs::read_to_string(&router).expect("le routeur est lisible");
    let ampute = source
        .lines()
        .filter(|ligne| !ligne.contains("rbs:routes"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(
        source, ampute,
        "l'ancre visée doit exister avant d'être retirée, sans quoi le test ne prouve rien"
    );
    fs::write(&router, format!("{ampute}\n")).expect("le routeur s'écrit");
    common::commiter(&racine, "routeur sans son ancre");

    let avant = common::empreinte(&racine);
    let output = add_trial(&racine, &fragments, &[]);

    assert!(
        !output.succes,
        "l'installation doit sortir en erreur :\n{}",
        output.stdout
    );
    assert!(
        output.stderr.contains("<rbs:routes>") && output.stderr.contains("src/router.rs"),
        "l'erreur doit nommer l'ancre et son fichier :\n{}",
        output.stderr
    );
    assert!(
        output.stdout.contains("// <rbs:routes>") && output.stdout.contains("// </rbs:routes>"),
        "le bloc à coller doit être affiché :\n{}",
        output.stdout
    );
    common::assert_intact(
        &avant,
        &racine,
        "l'ancre absente n'a pas empêché l'écriture",
    );
}

/// Le critère de la tâche, sur le fragment livré et non sur un fragment de test : ce que
/// `rbs add redis` écrit dans le projet.
///
/// Les trois pièces du moule y passent d'un coup — les deux ancres d'état, les deux
/// crates tierces, la section de configuration —, ce qu'aucune feature livrée n'avait
/// encore exercé.
#[test]
fn the_redis_fragment_writes_the_state_anchors_the_dependencies_and_the_cache_section() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    let output = Sortie::de(rbs(&racine).args(["add", "redis"]));
    assert!(
        output.succes,
        "l'installation doit aboutir :\n{}",
        output.stderr
    );

    let state = fs::read_to_string(racine.join("src/state.rs")).expect("state.rs est lisible");
    for (anchor, ligne) in [
        ("state_champs", "pub cache: crate::modules::cache::Cache,"),
        (
            "state_init",
            "cache: crate::modules::cache::Cache::from_config()?,",
        ),
    ] {
        let ouverture = format!("// <rbs:{anchor}>");
        let fermeture = format!("// </rbs:{anchor}>");
        let debut = state
            .find(&ouverture)
            .unwrap_or_else(|| panic!("state.rs ne porte pas `{ouverture}` :\n{state}"))
            + ouverture.len();
        let fin = state
            .find(&fermeture)
            .unwrap_or_else(|| panic!("state.rs ne porte pas `{fermeture}` :\n{state}"));

        assert!(
            state[debut..fin].contains(ligne),
            "l'ancre `{anchor}` ne porte pas `{ligne}` :\n{state}"
        );
    }

    let manifeste =
        fs::read_to_string(racine.join("Cargo.toml")).expect("le manifeste est lisible");
    assert!(
        manifeste.contains("redis = { version = \"1.7\", features = [\"tokio-comp\"] }"),
        "la crate `redis` manque au manifeste :\n{manifeste}"
    );
    assert!(
        manifeste.contains("deadpool-redis = \"0.23\""),
        "la crate `deadpool-redis` manque au manifeste :\n{manifeste}"
    );

    let config =
        fs::read_to_string(racine.join("config/default.toml")).expect("la config est lisible");
    assert!(config.contains("[cache]"), "section absente :\n{config}");
    assert!(
        config.contains("url = \"redis://127.0.0.1:6379\"") && config.contains("ttl_secs = 300"),
        "la section `[cache]` est incomplète :\n{config}"
    );
}

/// Le critère de la tâche : le second `rbs add redis` n'écrit rien.
///
/// Distinct de `two_successive_installs_write_nothing_the_second_time`, qui l'éprouve
/// sur un fragment fabriqué : celui-ci porte sur le fragment livré, avec les lignes qu'il
/// insère réellement dans quatre fichiers du projet.
#[test]
fn installing_redis_twice_writes_nothing_the_second_time() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_project(&parent);

    rbs(&racine).args(["add", "redis"]).assert().success();
    common::commiter(&racine, "redis");

    let avant = common::empreinte(&racine);
    let seconde = Sortie::de(rbs(&racine).args(["add", "redis"]));

    assert!(
        seconde.succes,
        "la seconde installation doit aboutir sans rien faire :\n{}",
        seconde.stderr
    );
    common::assert_intact(
        &avant,
        &racine,
        "la seconde installation a modifié le projet",
    );
}

/// Un projet SQLite, créé par le binaire livré et commité : le seul preset de `rbs new`
/// qui n'écrit pas de compose, et sur lequel `rbs add auth` échouait tout net.
fn committed_sqlite_project(parent: &TempDir) -> PathBuf {
    let noyau = common::noyau();

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--database",
            "sqlite",
            "--database-url",
            "sqlite://demo_api.db?mode=rwc",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let racine = parent.path().join("demo-api");
    common::commiter(&racine, "projet neuf");
    racine
}

/// `auth` entraîne `mail`, dont le service `mailpit` va dans un compose que le projet
/// SQLite n'a pas. L'installation aboutit, n'invente pas de compose, et dit ce qui reste à
/// monter — dès le `--dry-run`, avant que quoi que ce soit ne s'écrive.
#[test]
fn adding_auth_to_a_sqlite_project_succeeds_and_names_the_service_left_to_mount() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = committed_sqlite_project(&parent);
    assert!(!racine.join("docker-compose.yml").exists());

    let avant = common::empreinte(&racine);
    let essai = Sortie::de(rbs(&racine).args(["add", "auth", "--dry-run"]));
    assert!(essai.succes, "le plan doit se calculer :\n{}", essai.stderr);
    assert!(
        essai.stdout.contains("docker-compose.yml absent") && essai.stdout.contains("mailpit:"),
        "le --dry-run doit nommer le fichier absent et le service :\n{}",
        essai.stdout
    );
    common::assert_intact(&avant, &racine, "un --dry-run n'écrit rien");

    let output = Sortie::de(rbs(&racine).args(["add", "auth"]));
    assert!(
        output.succes,
        "l'installation doit aboutir :\n{}",
        output.stderr
    );
    assert!(
        output.stdout.contains("docker-compose.yml absent") && output.stdout.contains("mailpit:"),
        "l'installation doit nommer le fichier absent et le service :\n{}",
        output.stdout
    );
    assert!(
        !racine.join("docker-compose.yml").exists(),
        "aucun compose ne doit être inventé"
    );
    assert!(
        racine.join("src/auth/service/mod.rs").is_file(),
        "auth doit être posée"
    );
    let manifeste =
        fs::read_to_string(racine.join("Cargo.toml")).expect("le manifeste est lisible");
    for feature in ["\"auth\"", "\"mail\""] {
        assert!(
            manifeste.contains(feature),
            "{feature} manque :\n{manifeste}"
        );
    }
}
