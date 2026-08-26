//! Banc d'essai des générateurs : un vrai projet, créé par le vrai binaire, compilé.
//!
//! Les tests de rendu vérifient des chaînes ; seul `rustc` prouve que ces chaînes forment
//! du Rust valide contre SeaORM, utoipa et validator. Le projet est donc créé par
//! `rbs new`, sa feature écrite, puis compilé.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Mutex, PoisonError};

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::GenericImage;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, ImageExt};

/// PostgreSQL 18 en conteneur, et l'URL de connexion qui y mène.
///
/// La version n'est pas négociable : `uuidv7()` n'est native qu'à partir de la 18, et la
/// spec assume ce plancher plutôt qu'une fonction PL/pgSQL de compatibilité.
pub(crate) struct BaseDeTest {
    _conteneur: Container<GenericImage>,
    url: String,
}

impl BaseDeTest {
    pub(crate) fn demarrer() -> Self {
        // Le message d'ouverture paraît deux fois : le serveur temporaire de l'initdb
        // l'écrit avant que la base définitive n'existe. S'y connecter à la première
        // occurrence donne un refus, ou pire, une base qui disparaît sous le test.
        let ouverture = || WaitFor::message_on_stderr("ready to accept connections");

        let conteneur = GenericImage::new("postgres", "18")
            .with_wait_for(ouverture())
            .with_wait_for(ouverture())
            .with_env_var("POSTGRES_USER", "rbs")
            .with_env_var("POSTGRES_PASSWORD", "rbs")
            .with_env_var("POSTGRES_DB", "demo_api")
            .start()
            .expect("PostgreSQL 18 doit démarrer — Docker requis");
        let port = conteneur
            .get_host_port_ipv4(5432.tcp())
            .expect("port de la base exposé");

        Self {
            url: format!("postgres://rbs:rbs@127.0.0.1:{port}/demo_api"),
            _conteneur: conteneur,
        }
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

/// Sérialise les invocations de cargo sur les projets d'essai.
///
/// Ils partagent `target/rbs-integration` pour ne pas recompiler Axum et SeaORM à chaque
/// test. Or cargo y dépose l'artefact final du paquet visé — `debug/demo-api`,
/// `debug/libmigration.rlib` — sous un nom qui ne distingue pas un projet d'un autre :
/// deux invocations concurrentes se relisent mutuellement leurs binaires.
///
/// Ce verrou ne suffit pas à les isoler complètement : deux projets qui compilent un même
/// module de migration — même nom de fichier, même crate `migration` — se sont montrés
/// capables d'échanger leur code compilé d'un projet à l'autre. Deux tests lourds ne
/// doivent donc jamais poser une feature ni une migration de même nom.
static CARGO: Mutex<()> = Mutex::new(());

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
pub(crate) fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

/// Un projet neuf, créé par le binaire livré, prêt à recevoir une feature.
pub(crate) struct Projet {
    _parent: TempDir,
    racine: PathBuf,
}

impl Projet {
    pub(crate) fn neuf() -> Self {
        Self::neuf_sur("postgres://rbs:rbs@localhost:5432/demo_api")
    }

    /// Un projet neuf dont le `.env` vise `url`.
    ///
    /// Ce que le projet lit de sa base passe par sa configuration : un test qui exerce
    /// l'application montée doit donc pointer le `.env` sur le conteneur, et non fournir
    /// l'URL à l'exécution.
    pub(crate) fn neuf_sur(url: &str) -> Self {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let noyau = depot().join("crates/rbs-core");

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(parent.path())
            .args([
                "new",
                "demo-api",
                "--database-url",
                url,
                "--core-path",
                noyau.to_str().expect("chemin du noyau représentable"),
                "--yes",
            ])
            .assert()
            .success();

        let racine = parent.path().join("demo-api");

        Self {
            _parent: parent,
            racine,
        }
    }

    pub(crate) fn racine(&self) -> &Path {
        &self.racine
    }

    /// Écrit `src/<module>/` avec les fichiers donnés, et déclare le module.
    ///
    /// L'insertion dans l'ancre est faite à la main : le moteur d'ancres est une tâche
    /// distincte, dont ce banc ne doit pas dépendre.
    pub(crate) fn poser_feature(&self, module: &str, fichiers: &[(&str, &str)]) {
        let repertoire = self.racine.join("src").join(module);
        fs::create_dir_all(&repertoire).expect("répertoire de feature créable");

        // Un `mod.rs` fourni l'emporte : dès que la feature porte son propre `routes()`,
        // la liste de déclarations déduite des noms de fichiers ne suffit plus.
        let declarations = fichiers
            .iter()
            .find(|(nom, _)| *nom == "mod.rs")
            .map_or_else(
                || {
                    fichiers
                        .iter()
                        .map(|(nom, _)| {
                            let module = nom.trim_end_matches(".rs");
                            format!("pub mod {module};\n")
                        })
                        .collect()
                },
                |(_, contenu)| (*contenu).to_string(),
            );

        fs::write(repertoire.join("mod.rs"), declarations).expect("mod.rs écrivable");

        for (nom, contenu) in fichiers.iter().filter(|(nom, _)| *nom != "mod.rs") {
            fs::write(repertoire.join(nom), contenu).expect("fichier de feature écrivable");
        }

        let main = self.racine.join("src/main.rs");
        let source = fs::read_to_string(&main).expect("main.rs lisible");

        fs::write(
            &main,
            source.replace(
                "// <rbs:features>",
                &format!("// <rbs:features>\nmod {module};"),
            ),
        )
        .expect("main.rs écrivable");
    }

    /// Monte les routes de `module` et déclare ses `handlers` dans le document OpenAPI.
    ///
    /// Le remplissage des ancres est fait à la main, comme celui de `<rbs:features>` :
    /// le moteur d'ancres est une tâche distincte, dont ce banc ne doit pas dépendre.
    pub(crate) fn monter_feature(&self, module: &str, handlers: &[&str]) {
        let routeur = self.racine.join("src/router.rs");
        let source = fs::read_to_string(&routeur).expect("routeur lisible");

        fs::write(
            &routeur,
            source.replace(
                "// <rbs:routes>",
                &format!("// <rbs:routes>\n        .merge(crate::{module}::routes())"),
            ),
        )
        .expect("routeur écrivable");

        let document = self.racine.join("src/openapi.rs");
        let source = fs::read_to_string(&document).expect("document lisible");
        let chemins: String = handlers
            .iter()
            .map(|handler| format!("\n        crate::{module}::controller::{handler},"))
            .collect();

        fs::write(
            &document,
            source.replace("// <rbs:openapi>", &format!("// <rbs:openapi>{chemins}")),
        )
        .expect("document écrivable");
    }

    /// Ajoute un module de test au binaire du projet.
    ///
    /// Le projet généré est un binaire : un test d'intégration ne peut pas atteindre ses
    /// modules. Ce qui doit inspecter le projet de l'intérieur passe donc par ici.
    pub(crate) fn poser_test_unitaire(&self, nom: &str, contenu: &str) {
        let sources = self.racine.join("src");
        fs::write(sources.join(format!("{nom}.rs")), contenu).expect("module de test écrivable");

        let main = sources.join("main.rs");
        let source = fs::read_to_string(&main).expect("main.rs lisible");

        fs::write(&main, format!("#[cfg(test)]\nmod {nom};\n{source}")).expect("main.rs écrivable");
    }

    /// Recopie le projet sous `target/atelier/` et rend son chemin.
    ///
    /// Le répertoire temporaire disparaît avec le test ; les critères qui demandent une
    /// revue à l'œil — Swagger UI, la mise en page des logs — ont besoin d'un projet qui
    /// survit, qu'on démarre et qu'on regarde.
    pub(crate) fn conserver(&self) -> PathBuf {
        let destination = depot().join("target/atelier");
        let _ = fs::remove_dir_all(&destination);
        fs::create_dir_all(&destination).expect("répertoire d'atelier créable");

        let sortie = std::process::Command::new("cp")
            .arg("-R")
            .arg(self.racine.join("."))
            .arg(&destination)
            .output()
            .expect("copie lançable");

        assert!(
            sortie.status.success(),
            "copie du projet impossible :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );

        destination
    }

    /// Applique les migrations du projet contre `url`, puis retire de quoi les appliquer.
    ///
    /// La crate `migration` n'a pas de binaire et `rbs migrate` n'existe pas encore : la
    /// montée passe par un test jetable, effacé aussitôt pour que `tester()` ne trouve
    /// dans le projet que du code généré.
    pub(crate) fn migrer(&self, url: &str) {
        const MONTEE: &str = r#"use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::Database;

#[tokio::test]
async fn appliquer() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL doit être fournie");
    let db = Database::connect(&url).await.expect("connexion à la base");

    Migrator::up(&db, None).await.expect("montée des migrations");
}
"#;

        self.poser_test_de_migration("montee", MONTEE);
        self.tester_migration(url);
        fs::remove_file(self.racine.join("migration/tests/montee.rs"))
            .expect("test jetable effacé");
    }

    /// Lance cargo sur le projet, un seul appel à la fois.
    fn cargo(&self, arguments: &[&str], variables: &[(&str, &str)]) -> Output {
        let _exclusivite = CARGO.lock().unwrap_or_else(PoisonError::into_inner);

        std::process::Command::new("cargo")
            .current_dir(&self.racine)
            .env("CARGO_TARGET_DIR", depot().join("target/rbs-integration"))
            .envs(variables.iter().copied())
            .args(arguments)
            .output()
            .expect("cargo doit être lançable")
    }

    /// Lance les tests du projet, et rapporte leur sortie.
    pub(crate) fn tester(&self) {
        let sortie = self.cargo(&["test"], &[]);
        let journal = String::from_utf8_lossy(&sortie.stdout);

        assert!(
            sortie.status.success(),
            "les tests du projet échouent :\n{journal}\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
        // Un `cargo test` qui ne trouve aucun test sort vert : sans ce garde-fou, une
        // feature dont les tests ne seraient pas compilés passerait pour vérifiée.
        assert!(
            tests_executes(&journal) > 0,
            "aucun test n'a été exécuté :\n{journal}"
        );
    }

    /// Ajoute une migration au projet et l'inscrit dans l'ancre du `Migrator`.
    pub(crate) fn poser_migration(&self, module: &str, contenu: &str) {
        let sources = self.racine.join("migration/src");
        fs::write(sources.join(format!("{module}.rs")), contenu).expect("migration écrivable");

        let lib = sources.join("lib.rs");
        let source = fs::read_to_string(&lib).expect("lib.rs de migration lisible");

        fs::write(
            &lib,
            format!("mod {module};\n\n{}", source).replace(
                "// <rbs:migrations>",
                &format!("// <rbs:migrations>\n            Box::new({module}::Migration),"),
            ),
        )
        .expect("lib.rs de migration écrivable");
    }

    /// Ajoute un test d'intégration à la crate `migration` du projet.
    ///
    /// `tokio` s'ajoute avec lui : une migration n'a pas besoin d'exécuteur, seul le test
    /// qui l'applique en réclame un.
    pub(crate) fn poser_test_de_migration(&self, nom: &str, contenu: &str) {
        let tests = self.racine.join("migration/tests");
        fs::create_dir_all(&tests).expect("répertoire de tests créable");
        fs::write(tests.join(format!("{nom}.rs")), contenu).expect("test écrivable");

        let manifeste = self.racine.join("migration/Cargo.toml");
        let source = fs::read_to_string(&manifeste).expect("manifeste de migration lisible");

        fs::write(
            &manifeste,
            format!(
                "{source}\n[dev-dependencies]\n\
                 tokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\n"
            ),
        )
        .expect("manifeste de migration écrivable");
    }

    /// Lance les tests de la crate `migration` contre `url`, et rapporte leur sortie.
    pub(crate) fn tester_migration(&self, url: &str) {
        let sortie = self.cargo(
            &["test", "-p", "migration", "--", "--nocapture"],
            &[("DATABASE_URL", url)],
        );

        assert!(
            sortie.status.success(),
            "les tests de migration échouent :\n{}\n{}",
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr)
        );
    }

    /// Compile le projet, et échoue en rapportant la sortie de `cargo` telle quelle.
    pub(crate) fn compiler(&self) {
        let sortie = self.cargo(&["build", "--workspace"], &[]);

        assert!(
            sortie.status.success(),
            "le projet ne compile pas :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
    }
}

/// Nombre de tests passés, tous les binaires de test du projet confondus.
fn tests_executes(journal: &str) -> u32 {
    journal
        .lines()
        .filter_map(|ligne| ligne.strip_prefix("test result: ok. "))
        .filter_map(|reste| reste.split_whitespace().next())
        .filter_map(|nombre| nombre.parse::<u32>().ok())
        .sum()
}

/// Passe `source` à rustfmt et rend le résultat.
///
/// Le code généré est écrit à la main dans des templates, sans que rien ne garantisse
/// qu'il porte déjà la mise en forme de rustfmt. Sans cette vérification, le premier
/// `cargo fmt` de l'utilisateur produirait un diff sur des fichiers qu'il n'a pas touchés.
pub(crate) fn formate(source: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;

    let mut rustfmt = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rustfmt doit être installé");

    rustfmt
        .stdin
        .take()
        .expect("entrée de rustfmt disponible")
        .write_all(source.as_bytes())
        .expect("source transmissible à rustfmt");

    let sortie = rustfmt
        .wait_with_output()
        .expect("rustfmt doit rendre la main");

    assert!(
        sortie.status.success(),
        "rustfmt refuse le rendu :\n{}",
        String::from_utf8_lossy(&sortie.stderr)
    );

    String::from_utf8(sortie.stdout).expect("rustfmt rend de l'UTF-8")
}
