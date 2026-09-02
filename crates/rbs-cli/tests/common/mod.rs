//! Ce que les tests d'intégration partagent : où trouver le dépôt, le noyau et la cible.
//!
//! Ces tests compilent des projets Axum + SeaORM complets. La cible commune n'est pas un
//! détail de confort : sans elle, chaque test recompile toute l'arborescence de
//! dépendances pour son compte.

// Chaque test d'intégration compile ce module pour son propre compte, et aucun n'en
// appelle la totalité : ce qui sert à l'un est mort pour l'autre.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, GenericImage, ImageExt};

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
pub fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

/// Le noyau du dépôt, dont dépendra le projet créé.
pub fn noyau() -> PathBuf {
    depot().join("crates/rbs-core")
}

/// Répertoire de compilation partagé par les projets créés en test.
pub fn cible() -> PathBuf {
    depot().join("target/rbs-integration")
}

/// Le même, propre au moteur demandé.
///
/// Les trois moteurs activent des features `sea-orm` différentes : les faire partager une
/// cible ferait recompiler `sea-orm` et `sqlx` à chaque bascule, y compris pour les tests
/// qui n'ont rien demandé. Un arbre par moteur reste chaud d'un run à l'autre.
pub fn cible_pour(moteur: &str) -> PathBuf {
    depot().join(format!("target/rbs-integration-{moteur}"))
}

/// Prend pour soi la cible de compilation `cible`, jusqu'à ce que le garde soit lâché.
///
/// Chaque fichier de `tests/` est un binaire que `cargo test` lance de front avec les
/// autres : rien de ce qui vit dans un processus ne les sépare, et ils écrivent pourtant
/// la même cible. Le garde se prend donc avant la première invocation de `cargo` ou de
/// `rbs`, et se tient jusqu'à la dernière — y compris pendant qu'un binaire bâti tourne,
/// cargo relâchant le sien avant de l'exécuter.
pub fn verrou(cible: &Path) -> std::fs::File {
    test_cible::verrou(cible)
}

/// Un projet neuf, créé par le binaire livré, dans `parent`.
pub fn projet(parent: &Path) -> PathBuf {
    let noyau = noyau();

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent)
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    parent.join("demo-api")
}

/// Commite tout ce que porte `racine`, pour que son working tree y soit propre.
///
/// `rbs new` initialise le dépôt sans commiter : sans ce premier commit, aucun fichier
/// n'est suivi et la garde du working tree n'a rien à protéger.
pub fn commiter(racine: &Path, message: &str) {
    for arguments in [
        vec!["config", "user.email", "rbs@example.test"],
        vec!["config", "user.name", "rbs"],
        vec!["add", "-A"],
        vec!["commit", "--quiet", "-m", message],
    ] {
        let output = std::process::Command::new("git")
            .args(&arguments)
            .current_dir(racine)
            .output()
            .expect("git doit être lançable");

        assert!(
            output.status.success(),
            "git {arguments:?} a échoué :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// L'état complet d'un projet : chaque fichier par son contenu, `.git` et `target` exclus.
pub type Empreinte = BTreeMap<PathBuf, String>;

/// Relève l'empreinte de `racine`.
pub fn empreinte(racine: &Path) -> Empreinte {
    let mut fichiers = Empreinte::new();
    collect(racine, racine, &mut fichiers);
    fichiers
}

/// Échoue si le projet a bougé, en ne montrant que ce qui a bougé.
///
/// Comparer les deux empreintes par `assert_eq!` déverserait le projet entier deux fois :
/// la différence tient en trois lignes, et c'est elle qui se lit.
pub fn assert_intact(avant: &Empreinte, racine: &Path, contexte: &str) {
    let apres = empreinte(racine);
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

    assert!(ecarts.is_empty(), "{contexte} :\n{}", ecarts.join("\n"));
}

fn collect(racine: &Path, repertoire: &Path, fichiers: &mut Empreinte) {
    let entrees = fs::read_dir(repertoire).expect("répertoire lisible");

    for entree in entrees {
        let chemin = entree.expect("entrée lisible").path();
        let nom = chemin.file_name().unwrap_or_default().to_string_lossy();

        if nom == ".git" || nom == "target" {
            continue;
        }

        if chemin.is_dir() {
            collect(racine, &chemin, fichiers);
        } else {
            let relatif = chemin.strip_prefix(racine).expect("sous la racine");
            let contenu = fs::read_to_string(&chemin).unwrap_or_default();
            fichiers.insert(relatif.to_path_buf(), contenu);
        }
    }
}

// --- Bases sous conteneur ---------------------------------------------------------
//
// Deux fichiers de tests démarrent les mêmes bases : `integration_jobs`, qui y joue la
// file, et `integration_new`, qui y joue la suite d'un projet engendré par moteur. Les
// démarreurs vivent donc ici plutôt que dans l'un des deux.

// Le même fichier que celui qu'emploie le banc des générateurs : trois constantes
// éparpillées, c'est trois occasions qu'une branche de la matrice n'aille pas où elle dit.
#[path = "../../src/test_postgres.rs"]
mod test_postgres;

// Même partage, même raison : le verrou de la cible sert au banc des générateurs comme à
// ces tests, et les deux mondes de compilation ne se relient que par `#[path]`.
#[path = "../../src/test_cible.rs"]
mod test_cible;

/// L'image PostgreSQL du harnais : la 18 livrée, ou le plancher 14 sous `RBS_TEST_PG`.
pub use test_postgres::image as postgres_image;

pub const UTILISATEUR: &str = "rbs";
pub const MOT_DE_PASSE: &str = "rbs";
pub const BASE: &str = "demo";

/// Un PostgreSQL neuf, prêt à recevoir le schéma d'un projet généré.
pub fn start_postgres() -> Container<GenericImage> {
    let (nom, version) = postgres_image();
    GenericImage::new(nom, version)
        .with_wait_for(WaitFor::log(
            // PostgreSQL annonce une première fois qu'il accepte les connexions pendant
            // son initialisation, où il n'écoute que sur son socket local : attendre la
            // seconde annonce évite un test qui échoue une fois sur trois.
            LogWaitStrategy::stdout_or_stderr("database system is ready to accept connections")
                .with_times(2),
        ))
        .with_env_var("POSTGRES_USER", UTILISATEUR)
        .with_env_var("POSTGRES_PASSWORD", MOT_DE_PASSE)
        .with_env_var("POSTGRES_DB", BASE)
        .start()
        .expect("PostgreSQL doit démarrer — Docker est-il lancé ?")
}

/// MySQL 8, dont `FOR UPDATE SKIP LOCKED` est contemporain.
pub fn start_mysql() -> Container<GenericImage> {
    GenericImage::new("mysql", "8")
        .with_wait_for(WaitFor::log(
            // Comme PostgreSQL, MySQL annonce deux fois qu'il est prêt : la première fois
            // pendant son initialisation, où il n'écoute que localement.
            LogWaitStrategy::stdout_or_stderr("ready for connections").with_times(2),
        ))
        .with_env_var("MYSQL_ROOT_PASSWORD", MOT_DE_PASSE)
        .with_env_var("MYSQL_DATABASE", BASE)
        .start()
        .expect("MySQL doit démarrer — Docker est-il lancé ?")
}

/// L'URL de connexion à `mysql`, vue depuis l'hôte.
pub fn url_of_mysql(mysql: &Container<GenericImage>) -> String {
    let port = mysql
        .get_host_port_ipv4(3306.tcp())
        .expect("le port de MySQL doit être publié");

    format!("mysql://root:{MOT_DE_PASSE}@127.0.0.1:{port}/{BASE}")
}

/// L'URL de connexion à `postgres`, vue depuis l'hôte.
pub fn url_of(postgres: &Container<GenericImage>) -> String {
    let port = postgres
        .get_host_port_ipv4(5432.tcp())
        .expect("le port de PostgreSQL doit être publié");

    format!("postgres://{UTILISATEUR}:{MOT_DE_PASSE}@127.0.0.1:{port}/{BASE}")
}
