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
fn projet_commite(parent: &TempDir) -> PathBuf {
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
        let sortie = commande.output().expect("le binaire doit être lançable");

        Self {
            succes: sortie.status.success(),
            stdout: String::from_utf8_lossy(&sortie.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&sortie.stderr).into_owned(),
        }
    }
}

/// Installer deux fois la même feature laisse le projet exactement là où la première
/// installation l'avait laissé.
#[test]
fn installer_deux_fois_la_meme_feature_ne_produit_rien_la_seconde() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_commite(&parent);

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
fn une_ancre_supprimee_refuse_l_ecriture_et_affiche_le_bloc_a_coller() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_commite(&parent);

    let router = racine.join("src").join("router.rs");
    let ampute: Vec<String> = fs::read_to_string(&router)
        .expect("router.rs lisible")
        .lines()
        .filter(|ligne| !ligne.contains("// <rbs:routes>"))
        .map(str::to_string)
        .collect();
    fs::write(&router, ampute.join("\n")).expect("router.rs inscriptible");

    let avant = common::empreinte(&racine);
    let sortie = Sortie::de(rbs(&racine).args([
        "g",
        "crud",
        "notes",
        "--fields",
        "titre:string",
        "--force",
    ]));

    assert!(
        !sortie.succes,
        "la génération a abouti malgré l'ancre absente :\n{}",
        sortie.stdout
    );
    assert!(
        sortie.stderr.contains("<rbs:routes>") && sortie.stderr.contains("src/router.rs"),
        "l'erreur doit nommer l'ancre et son fichier :\n{}",
        sortie.stderr
    );
    assert!(
        sortie.stdout.contains("// <rbs:routes>") && sortie.stdout.contains("// </rbs:routes>"),
        "le bloc à recoller doit porter les deux balises :\n{}",
        sortie.stdout
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
fn un_working_tree_sale_refuse_sans_force_et_passe_avec() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_commite(&parent);

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
/// L'échec n'est pas injecté : le fichier que le plan écrira en second est posé en lecture
/// seule. `Permissions::set_readonly` vaut sur Unix comme sur Windows, et le plan est
/// appliqué dans l'ordre alphabétique — `Cargo.toml` puis `Dockerfile` sont bel et bien
/// écrits avant que `docker-compose.yml` ne fasse échouer l'ensemble.
#[test]
fn un_echec_en_cours_d_application_restaure_les_fichiers_deja_ecrits() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = projet_commite(&parent);

    let piege = racine.join("docker-compose.yml");
    fs::write(&piege, "# posé par le test, en lecture seule\n").expect("piège inscriptible");
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
    let sortie = Sortie::de(rbs(&racine).args(["add", "docker", "--force"]));

    assert!(
        !sortie.succes,
        "l'installation a abouti malgré un fichier non inscriptible :\n{}",
        sortie.stdout
    );
    assert!(
        !racine.join("Dockerfile").exists(),
        "le Dockerfile écrit avant l'échec n'a pas été retiré"
    );
    common::assert_intact(&avant, &racine, "l'échec a laissé le projet modifié");
}
