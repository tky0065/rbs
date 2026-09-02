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

use crate::anchors::{self, Anchor};

use super::mount::{self, Mount};

/// PostgreSQL en conteneur, et l'URL de connexion qui y mène.
///
/// La version démarrée est celle que `test_postgres::image` désigne : la 18 livrée par
/// défaut, ou le plancher 14 quand `RBS_TEST_PG` le demande.
pub(crate) struct TestDatabase {
    _conteneur: Container<GenericImage>,
    url: String,
}

impl TestDatabase {
    pub(crate) fn start() -> Self {
        // Le message d'ouverture paraît deux fois : le serveur temporaire de l'initdb
        // l'écrit avant que la base définitive n'existe. S'y connecter à la première
        // occurrence donne un refus, ou pire, une base qui disparaît sous le test.
        let opening = || WaitFor::message_on_stderr("ready to accept connections");

        let (nom, version) = crate::test_postgres::image();
        let container = GenericImage::new(nom, version.clone())
            .with_wait_for(opening())
            .with_wait_for(opening())
            .with_env_var("POSTGRES_USER", "rbs")
            .with_env_var("POSTGRES_PASSWORD", "rbs")
            .with_env_var("POSTGRES_DB", "demo_api")
            .start()
            .unwrap_or_else(|erreur| {
                panic!("PostgreSQL {version} doit démarrer — Docker requis : {erreur}")
            });
        let port = container
            .get_host_port_ipv4(5432.tcp())
            .expect("port de la base exposé");

        Self {
            url: format!("postgres://rbs:rbs@127.0.0.1:{port}/demo_api"),
            _conteneur: container,
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
pub(crate) fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

/// Un projet neuf, créé par le binaire livré, prêt à recevoir une feature.
pub(crate) struct Project {
    _parent: TempDir,
    root: PathBuf,
}

impl Project {
    pub(crate) fn fresh() -> Self {
        Self::fresh_on("postgres://rbs:rbs@localhost:5432/demo_api")
    }

    /// Un projet neuf dont le `.env` vise `url`.
    ///
    /// Ce que le projet lit de sa base passe par sa configuration : un test qui exerce
    /// l'application montée doit donc pointer le `.env` sur le conteneur, et non fournir
    /// l'URL à l'exécution.
    pub(crate) fn fresh_on(url: &str) -> Self {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let core = repo().join("crates/rbs-core");

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(parent.path())
            .args([
                "new",
                "demo-api",
                "--database-url",
                url,
                "--core-path",
                core.to_str().expect("chemin du noyau représentable"),
                "--yes",
            ])
            .assert()
            .success();

        let root = parent.path().join("demo-api");

        Self {
            _parent: parent,
            root,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Écrit `src/<module>/` avec les fichiers donnés, et déclare le module.
    pub(crate) fn write_feature(&self, module: &str, files: &[(&str, &str)]) {
        let directory = self.root.join("src").join(module);
        fs::create_dir_all(&directory).expect("répertoire de feature créable");

        // Un `mod.rs` fourni l'emporte : dès que la feature porte son propre `routes()`,
        // la liste de déclarations déduite des noms de fichiers ne suffit plus.
        let declarations = files
            .iter()
            .find(|(name, _)| *name == "mod.rs")
            .map_or_else(
                || {
                    files
                        .iter()
                        .map(|(name, _)| {
                            let module = name.trim_end_matches(".rs");
                            format!("pub mod {module};\n")
                        })
                        .collect()
                },
                |(_, content)| (*content).to_string(),
            );

        fs::write(directory.join("mod.rs"), declarations).expect("mod.rs écrivable");

        for (name, content) in files.iter().filter(|(name, _)| *name != "mod.rs") {
            fs::write(directory.join(name), content).expect("fichier de feature écrivable");
        }

        let features = anchors::resolve_features(&self.root);
        self.mount(&mount::pour(module, features.clone()), &[features]);
    }

    /// Monte les routes de `module` et ses handlers dans le document OpenAPI.
    pub(crate) fn mount_feature(&self, module: &str) {
        let features = anchors::resolve_features(&self.root);
        self.mount(
            &mount::pour(module, features),
            &[anchors::ROUTES, anchors::OPENAPI],
        );
    }

    /// Écrit dans les ancres `visees` par le moteur du CLI, et non à la main.
    ///
    /// Ce que le banc simulerait ici est précisément ce qu'il doit éprouver : la seule
    /// preuve que le moteur d'ancres produit du Rust valide est un projet qui compile.
    fn mount(&self, montages: &[Mount], visees: &[Anchor]) {
        for mount in montages.iter().filter(|m| visees.contains(&m.anchor)) {
            let path = self.root.join(mount.anchor.file.as_ref());
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} illisible : {error}", path.display()));

            let rendered = anchors::insert(&source, mount.anchor.clone(), &mount.lines)
                .unwrap_or_else(|error| panic!("{error}"));

            fs::write(&path, rendered)
                .unwrap_or_else(|error| panic!("{} non écrit : {error}", path.display()));
        }
    }

    /// Ajoute un module de test au binaire du projet.
    ///
    /// Le projet généré est un binaire : un test d'intégration ne peut pas atteindre ses
    /// modules. Ce qui doit inspecter le projet de l'intérieur passe donc par ici.
    pub(crate) fn write_unit_test(&self, name: &str, content: &str) {
        let sources = self.root.join("src");
        fs::write(sources.join(format!("{name}.rs")), content).expect("module de test écrivable");

        let main = sources.join("main.rs");
        let source = fs::read_to_string(&main).expect("main.rs lisible");

        fs::write(&main, format!("#[cfg(test)]\nmod {name};\n{source}"))
            .expect("main.rs écrivable");
    }

    /// Recopie le projet sous `target/workshop/` et rend son chemin.
    ///
    /// Le répertoire temporaire disparaît avec le test ; les critères qui demandent une
    /// revue à l'œil — Swagger UI, la mise en page des logs — ont besoin d'un projet qui
    /// survit, qu'on démarre et qu'on regarde.
    pub(crate) fn keep(&self) -> PathBuf {
        let destination = repo().join("target/atelier");
        let _ = fs::remove_dir_all(&destination);
        fs::create_dir_all(&destination).expect("répertoire d'atelier créable");

        let output = std::process::Command::new("cp")
            .arg("-R")
            .arg(self.root.join("."))
            .arg(&destination)
            .output()
            .expect("copie lançable");

        assert!(
            output.status.success(),
            "copie du projet impossible :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        destination
    }

    /// Applique les migrations du projet contre `url`, puis retire de quoi les appliquer.
    ///
    /// La crate `migration` n'a pas de binaire et `rbs migrate` n'existe pas encore : la
    /// montée passe par un test jetable, effacé aussitôt pour que `test_of()` ne trouve
    /// dans le projet que du code généré.
    pub(crate) fn migrate(&self, url: &str) {
        const MONTEE: &str = r#"use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::Database;

#[tokio::test]
async fn apply() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL doit être fournie");
    let db = Database::connect(&url).await.expect("connexion à la base");

    Migrator::up(&db, None).await.expect("montée des migrations");
}
"#;

        self.write_migration_test("montee", MONTEE);
        self.test_migration(url);
        fs::remove_file(self.root.join("migration/tests/montee.rs")).expect("test jetable effacé");
    }

    /// Lance cargo sur le projet, un seul appel à la fois.
    fn cargo(&self, arguments: &[&str], variables: &[(&str, &str)]) -> Output {
        let _exclusivite = CARGO.lock().unwrap_or_else(PoisonError::into_inner);

        std::process::Command::new("cargo")
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", repo().join("target/rbs-integration"))
            .envs(variables.iter().copied())
            .args(arguments)
            .output()
            .expect("cargo doit être lançable")
    }

    /// Lance le binaire `rbs` dans le projet, et rapporte sa sortie.
    ///
    /// Sous le même verrou que `cargo` : `rbs migrate` et `rbs seed` en lancent un, et il
    /// doit écrire dans la cible partagée comme les autres.
    pub(crate) fn rbs(&self, arguments: &[&str]) -> Output {
        let _exclusivite = CARGO.lock().unwrap_or_else(PoisonError::into_inner);

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", repo().join("target/rbs-integration"))
            .args(arguments)
            .output()
            .expect("rbs doit être lançable")
    }

    /// Lance `rbs` en exigeant qu'il aboutisse.
    pub(crate) fn rbs_ok(&self, arguments: &[&str]) {
        let output = self.rbs(arguments);

        assert!(
            output.status.success(),
            "`rbs {}` a échoué :\n{}\n{}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Lance les tests du projet dont le nom porte `filtre`, et rapporte leur sortie.
    pub(crate) fn test_matching(&self, filtre: &str) {
        let output = self.cargo(&["test", filtre], &[]);
        let journal = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "les tests du projet échouent :\n{journal}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            tests_run(&journal) > 0,
            "aucun test n'a été exécuté :\n{journal}"
        );
    }

    /// Lance les tests du projet, et rapporte leur sortie.
    ///
    /// `--include-ignored` plutôt que `test` seul : les tests engendrés joignent la base
    /// du projet et sont `#[ignore]` pour cette raison. Sans lui, la commande sortirait
    /// verte sans avoir rien exécuté, et le garde-fou ci-dessous serait le seul à le voir.
    pub(crate) fn test_of(&self) {
        let output = self.cargo(&["test", "--", "--include-ignored"], &[]);
        let journal = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "les tests du projet échouent :\n{journal}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Un `cargo test` qui ne trouve aucun test sort vert : sans ce garde-fou, une
        // feature dont les tests ne seraient pas compilés passerait pour vérifiée.
        assert!(
            tests_run(&journal) > 0,
            "aucun test n'a été exécuté :\n{journal}"
        );
    }

    /// Ajoute une migration au projet, la déclare et l'inscrit dans le `Migrator`.
    pub(crate) fn write_migration(&self, module: &str, content: &str) {
        let sources = self.root.join("migration/src");
        fs::write(sources.join(format!("{module}.rs")), content).expect("migration écrivable");

        self.mount(
            &mount::for_migration(module),
            &[anchors::MIGRATION_MODULES, anchors::MIGRATIONS],
        );
    }

    /// Ajoute un test d'intégration à la crate `migration` du projet.
    ///
    /// `tokio` s'ajoute avec lui : une migration n'a pas besoin d'exécuteur, seul le test
    /// qui l'applique en réclame un.
    pub(crate) fn write_migration_test(&self, name: &str, content: &str) {
        let tests = self.root.join("migration/tests");
        fs::create_dir_all(&tests).expect("répertoire de tests créable");
        fs::write(tests.join(format!("{name}.rs")), content).expect("test écrivable");

        let manifest = self.root.join("migration/Cargo.toml");
        let source = fs::read_to_string(&manifest).expect("manifeste de migration lisible");

        fs::write(
            &manifest,
            format!(
                "{source}\n[dev-dependencies]\n\
                 tokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\n"
            ),
        )
        .expect("manifeste de migration écrivable");
    }

    /// Lance les tests de la crate `migration` contre `url`, et rapporte leur sortie.
    pub(crate) fn test_migration(&self, url: &str) {
        let output = self.cargo(
            &["test", "-p", "migration", "--", "--nocapture"],
            &[("DATABASE_URL", url)],
        );

        assert!(
            output.status.success(),
            "les tests de migration échouent :\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Passe le projet au niveau qu'exige la CI que `rbs add ci` y pose.
    ///
    /// Un warning laissé dans du code généré rendrait rouge, dès le premier push, du code
    /// que l'utilisateur n'a pas écrit.
    pub(crate) fn clippy(&self) {
        let output = self.cargo(
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            &[],
        );

        assert!(
            output.status.success(),
            "clippy refuse le projet :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Compile le projet, et échoue en rapportant la sortie de `cargo` telle quelle.
    pub(crate) fn compile(&self) {
        let output = self.cargo(&["build", "--workspace"], &[]);

        assert!(
            output.status.success(),
            "le projet ne compile pas :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Nombre de tests passés, tous les binaires de test du projet confondus.
fn tests_run(journal: &str) -> u32 {
    journal
        .lines()
        .filter_map(|line| line.strip_prefix("test result: ok. "))
        .filter_map(|reste| reste.split_whitespace().next())
        .filter_map(|nombre| nombre.parse::<u32>().ok())
        .sum()
}

/// Passe `source` à rustfmt et rend le résultat.
///
/// Le code généré est écrit à la main dans des templates, sans que rien ne garantisse
/// qu'il porte déjà la mise en forme de rustfmt. Sans cette vérification, le premier
/// `cargo fmt` de l'utilisateur produirait un diff sur des fichiers qu'il n'a pas touchés.
///
/// `newline_style` est forcé pour la même raison qu'en `format::formatted` : son défaut,
/// « Auto », retombe sur le style de la plateforme, et les gardes compareraient un rendu LF
/// à une sortie CRLF.
pub(crate) fn formatted(source: &str) -> String {
    use std::io::Write;
    use std::process::Stdio;

    let mut rustfmt = std::process::Command::new("rustfmt")
        .args([
            "--edition",
            "2024",
            "--emit",
            "stdout",
            "--quiet",
            "--config",
            "newline_style=Unix",
        ])
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

    let output = rustfmt
        .wait_with_output()
        .expect("rustfmt doit rendre la main");

    assert!(
        output.status.success(),
        "rustfmt refuse le rendu :\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("rustfmt rend de l'UTF-8")
}
