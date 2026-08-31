//! Création d'un projet complet : squelette rendu, arborescence écrite, dépôt initialisé.
//!
//! La commande suit la séquence du §5.2 de la spec, dans l'ordre où elle rend les échecs
//! inoffensifs : ce qui peut être vérifié l'est avant que le rendu commence, et le rendu
//! aboutit entièrement avant que le premier fichier soit écrit. Un nom refusé, une
//! feature indisponible ou une variable de template absente laissent donc le disque
//! exactement dans l'état où ils l'ont trouvé.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use minijinja::context;

use crate::database::Database;
use crate::template::Renderer;
use crate::templates::Source;

/// Nom du compose à la racine du projet, tel que la template le rend et que `rbs dev` le
/// cherche.
const COMPOSE: &str = "docker-compose.yml";

/// Ce qu'il faut savoir avant de créer un projet, questions et flags confondus.
pub struct Options {
    /// Nom du projet, qui est aussi celui du répertoire et du paquet Cargo.
    pub name: String,
    /// URL de connexion écrite dans le `.env` du projet.
    pub database_url: String,
    /// Moteur de base sur lequel le projet tournera.
    pub database: Database,
    /// Features demandées à la création.
    pub features: Vec<String>,
    /// Noyau local à utiliser au lieu de la version publiée.
    pub core_path: Option<PathBuf>,
    /// Templates du disque remplaçant celles embarquées.
    pub template_dir: Option<PathBuf>,
    /// Langue du guide `AGENTS.md`.
    pub lang: crate::lang::Lang,
}

/// Ce qu'un projet créé rapporte à son appelant.
#[derive(Debug)]
pub struct Project {
    /// Racine du projet créé.
    pub root: PathBuf,
    /// Nombre de fichiers écrits.
    pub files: usize,
    /// `git init` a abouti. Faux n'invalide pas le projet.
    pub depot_git: bool,
    /// Les features que `--with` (ou la question) a demandées, et qui se sont installées.
    pub installed: Vec<InstalledFeature>,
}

/// Une feature posée par la création, et ce qu'elle a écrit.
#[derive(Debug)]
pub struct InstalledFeature {
    /// Nom de la feature, tel que `rbs add` l'accepte.
    pub name: String,
    /// Nombre de fichiers que le fragment a déposés.
    pub files: usize,
    /// Le fragment a posé une migration.
    pub migration: bool,
}

/// Ce qui peut empêcher la création d'un projet.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Le nom ne peut être ni un paquet Cargo ni un répertoire.
    #[error(
        "`{name}` n'est pas un name de project utilisable : lettres, chiffres, `-` et `_`, \
         en commençant par une lettre"
    )]
    NomInvalide {
        /// Nom refusé.
        name: String,
    },

    /// Une feature demandée n'a pas pu être installée.
    #[error("`{feature}` n'a pas pu être installée : {source}")]
    Installation {
        /// Feature en cause.
        feature: String,
        /// Cause remontée par l'installation.
        source: Box<crate::add::Error>,
    },

    /// La feature demandée n'existe pas.
    #[error("`{feature}` n'est pas une feature rbs — disponibles : {known}")]
    FeatureInconnue {
        /// Feature demandée.
        feature: String,
        /// Features que rbs connaît.
        known: String,
    },

    /// Le moteur choisi et l'URL donnée ne désignent pas la même base.
    #[error(
        "`--database {database}` n'accepte pas cette URL : attendu `{expected}://`, \
         trouvé {found} — choisissez l'un ou l'autre"
    )]
    UrlEtrangereAuMoteur {
        /// Moteur demandé au flag.
        database: Database,
        /// Schéma que le moteur attendait.
        expected: &'static str,
        /// Ce que l'URL porte, cité tel quel.
        found: String,
    },

    /// Le chemin visé est déjà pris.
    #[error("{path} existe déjà : choisissez un autre nom, ou retirez ce répertoire")]
    RepertoireOccupe {
        /// Chemin visé.
        path: String,
    },

    /// `--core-path` ne désigne pas un répertoire lisible.
    #[error("{path} est introuvable : `--core-path` désigne la crate `rbs-core` ({source})")]
    NoyauIntrouvable {
        /// Chemin donné.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// Les templates n'ont pas pu être lues.
    #[error("templates illisibles : {0}")]
    Templates(#[source] io::Error),

    /// Une template ne s'est pas rendue.
    #[error("{template} ne se rend pas : {source}")]
    Rendu {
        /// Destination de la template fautive.
        template: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// L'arborescence n'a pas pu être écrite.
    #[error("écriture impossible dans {path} : {source}")]
    Ecriture {
        /// Chemin en cause.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// Le guide de l'agent n'a pas pu être rendu.
    #[error("AGENTS.md n'a pas pu être écrit : {0}")]
    Agents(#[from] crate::agents::Error),
}

/// Crée le projet décrit par `options` dans `parent`.
///
/// # Erreurs
///
/// Échoue si le nom, les features ou le chemin visé sont inutilisables, si une template
/// ne se rend pas, ou si l'écriture échoue. Dans tous les cas, rien de ce que la commande
/// a créé ne subsiste.
pub fn create(options: &Options, parent: &Path) -> Result<Project, Error> {
    let disponibles = crate::templates::feature_names(options.template_dir.as_deref());

    validate_name(&options.name)?;
    validate_features(&options.features, &disponibles)?;
    validate_database(options.database, &options.database_url)?;

    let root = parent.join(&options.name);
    if root.exists() {
        return Err(Error::RepertoireOccupe {
            path: root.display().to_string(),
        });
    }

    let dependency = core_dependency(options.core_path.as_deref(), options.database)?;
    let rendus = render(options, &dependency)?;

    write(&root, &rendus).map_err(|(path, source)| {
        // Le répertoire n'existait pas : le retirer entièrement ne peut rien emporter
        // qui préexistait à la commande.
        let _ = fs::remove_dir_all(&root);
        Error::Ecriture { path, source }
    })?;

    // L'ordre est celui de la liste dérivée, non celui de la frappe : les insertions dans
    // le Migrator et dans le compose suivent l'ordre d'installation, et deux `--with`
    // équivalents doivent rendre deux projets identiques.
    let demandees: Vec<&String> = disponibles
        .iter()
        .filter(|feature| options.features.contains(feature))
        .collect();

    let mut installed = Vec::new();
    for feature in demandees {
        match install(&root, feature, options.template_dir.as_deref()) {
            Ok(pose) => installed.push(pose),
            Err(source) => {
                // Le répertoire n'existait pas avant la commande : le retirer entièrement
                // ne peut rien emporter qui lui préexistait.
                let _ = fs::remove_dir_all(&root);
                return Err(Error::Installation {
                    feature: feature.clone(),
                    source: Box::new(source),
                });
            }
        }
    }

    // Après les features, non avant : l'inventaire lit le manifeste que chaque
    // installation complète. Pour un projet neuf, la version qui écrit le guide est celle
    // du CLI lui-même.
    let agents =
        crate::agents::document(&root, options.lang, &options.name, crate::agents::VERSION)
            .inspect_err(|_| {
                // Le répertoire n'existait pas avant la commande : le retirer entièrement
                // ne peut rien emporter qui lui préexistait.
                let _ = fs::remove_dir_all(&root);
            })?;

    fs::write(root.join(crate::agents::FICHIER), agents).map_err(|source| {
        let path = root.join(crate::agents::FICHIER).display().to_string();
        let _ = fs::remove_dir_all(&root);
        Error::Ecriture { path, source }
    })?;

    Ok(Project {
        depot_git: git_init(&root),
        files: rendus.len() + 1,
        installed,
        root,
    })
}

/// Pose une feature dans le projet tout juste créé, par le pipeline de `rbs add`.
fn install(
    root: &Path,
    feature: &str,
    template_dir: Option<&Path>,
) -> Result<InstalledFeature, crate::add::Error> {
    let planned = crate::add::plan_for(&crate::add::Options {
        directory: root.to_path_buf(),
        feature: feature.to_string(),
        force: false,
        template_dir: template_dir.map(Path::to_path_buf),
    })?;

    let migration = planned
        .files
        .iter()
        .any(|file| file.starts_with("migration/src/"));
    let files = planned.files.len();

    crate::plan::application::apply(&planned.plan, false)?;

    Ok(InstalledFeature {
        name: feature.to_string(),
        files,
        migration,
    })
}

/// Le nom devient un `name` de manifeste et un nom de répertoire : ce qui n'est pas
/// valide pour les deux est refusé avant que quoi que ce soit s'écrive.
fn validate_name(name: &str) -> Result<(), Error> {
    let utilisable = name.starts_with(|premier: char| premier.is_ascii_alphabetic())
        && name.chars().all(|caractere| {
            caractere.is_ascii_alphanumeric() || caractere == '-' || caractere == '_'
        });

    if utilisable {
        Ok(())
    } else {
        Err(Error::NomInvalide {
            name: name.to_owned(),
        })
    }
}

/// L'URL doit désigner le moteur choisi, faute de quoi le projet compilerait avec un
/// pilote et échouerait à la connexion — la faute que `doctor` rattrape après coup.
fn validate_database(database: Database, url: &str) -> Result<(), Error> {
    if database.accepte(url) {
        return Ok(());
    }

    Err(Error::UrlEtrangereAuMoteur {
        database,
        expected: database.schemes()[0],
        found: match crate::database::scheme_of(url) {
            Some(scheme) => format!("`{scheme}://`"),
            None => "aucun schéma".to_owned(),
        },
    })
}

/// Une feature que rbs ne connaît pas est refusée avant qu'un fichier soit écrit — comme
/// le nom du projet et l'URL le sont déjà.
fn validate_features(features: &[String], disponibles: &[String]) -> Result<(), Error> {
    for feature in features {
        if !disponibles.contains(feature) {
            return Err(Error::FeatureInconnue {
                feature: feature.clone(),
                known: disponibles.join(", "),
            });
        }
    }

    Ok(())
}

/// Valeur de la dépendance à `rbs-core` dans le manifeste généré.
///
/// Le chemin est canonisé : Cargo le résout depuis le manifeste du projet créé, pas
/// depuis le répertoire où la commande a été lancée.
///
/// Le pilote se choisit ici, et les défauts du noyau sont coupés : les laisser actifs
/// ferait compiler PostgreSQL à un projet MySQL, les features de Cargo s'unifiant sur
/// toute la dépendance.
fn core_dependency(core_path: Option<&Path>, database: Database) -> Result<String, Error> {
    let provenance = match core_path {
        None => format!("version = \"{}\"", env!("CARGO_PKG_VERSION")),
        Some(path) => {
            let absolu = path
                .canonicalize()
                .map_err(|source| Error::NoyauIntrouvable {
                    path: path.display().to_string(),
                    source,
                })?;

            let value = toml_edit::Value::from(absolu.display().to_string());

            format!("path = {}", value.to_string().trim())
        }
    };

    Ok(format!(
        "{{ {provenance}, default-features = false, features = [\"{database}\"] }}"
    ))
}

/// Rend toutes les templates. Aucun fichier n'est écrit tant que la dernière n'a pas
/// abouti : une variable oubliée ne doit pas laisser un projet à moitié généré.
fn render(options: &Options, dependency: &str) -> Result<Vec<(PathBuf, String)>, Error> {
    let mut files = Source::fresh(options.template_dir.as_deref())
        .files()
        .map_err(Error::Templates)?;

    let connexion = crate::url::parse(&options.database_url);
    if !compose_utile(options, connexion.as_ref()) {
        files.retain(|file| file.destination != Path::new(COMPOSE));
    }

    let renderer = Renderer::new();
    let context = context! {
        project_name => options.name.as_str(),
        crate_name => crate_name(&options.name),
        rbs_core_dep => dependency,
        rbs_version => env!("CARGO_PKG_VERSION"),
        database_url => options.database_url.as_str(),
        database => options.database.name(),
        sea_orm_feature => options.database.sea_orm_feature(),
        database_url_par_defaut => options.database.default_url(&crate_name(&options.name)),
        database_user => connexion.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
        database_password => connexion.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        // Une URL sans chemin rend un nom de base vide, que le repli ne rattraperait pas
        // s'il ne guettait que `None` : le compose porterait un `POSTGRES_DB:` vide, et
        // le service ne deviendrait jamais sain.
        database_name => connexion
            .as_ref()
            .map(|c| c.database.clone())
            .filter(|base| !base.is_empty())
            .unwrap_or_else(|| crate_name(&options.name)),
        database_port => connexion.as_ref().map(|c| c.port).unwrap_or_default(),
        lang => options.lang.name(),
    };

    files
        .into_iter()
        .map(|file| {
            let rendered =
                renderer
                    .render(&file.source, &context)
                    .map_err(|source| Error::Rendu {
                        template: file.destination.display().to_string(),
                        source,
                    })?;

            Ok((file.destination, rendered))
        })
        .collect()
}

/// Écrit l'arborescence, en nommant le chemin qui a échoué.
fn write(root: &Path, rendus: &[(PathBuf, String)]) -> Result<(), (String, io::Error)> {
    for (destination, content) in rendus {
        let path = root.join(destination);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| (parent.display().to_string(), error))?;
        }

        fs::write(&path, content).map_err(|error| (path.display().to_string(), error))?;
    }

    Ok(())
}

/// Initialise le dépôt du projet créé.
///
/// L'échec n'est pas fatal : un projet sans dépôt reste un projet valide, et `git` peut
/// tout simplement ne pas être installé.
fn git_init(root: &Path) -> bool {
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .is_ok_and(|statut| statut.success())
}

/// Le compose n'est écrit que s'il éviterait un `docker run` tapé à la main.
///
/// SQLite n'a rien à monter. Une base distante non plus : engendrer un service local que
/// `rbs dev` monterait pendant que l'application en interroge un autre serait pire que de
/// ne rien écrire. Une URL sans identifiants — valide, `parse` l'accepte — vaut la même
/// abstention : l'image PostgreSQL officielle refuse de s'initialiser sans mot de passe,
/// et un compose qui ne peut pas démarrer est pire qu'un compose absent.
fn compose_utile(options: &Options, connexion: Option<&crate::url::Connection>) -> bool {
    options.database.a_un_serveur()
        && connexion.is_some_and(|connexion| {
            connexion.est_locale() && !connexion.user.is_empty() && !connexion.password.is_empty()
        })
}

/// Nom de la crate correspondant au nom du projet : un tiret n'est pas un caractère
/// d'identifiant Rust.
fn crate_name(name: &str) -> String {
    name.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;

    /// Les templates du dépôt, pour que les tests portent sur le squelette réel plutôt
    /// que sur une copie embarquée au moment de leur compilation.
    const SQUELETTE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/project");

    fn options(name: &str) -> Options {
        Options {
            name: name.to_owned(),
            database_url: "postgres://alice:s3cr3t@localhost:5432/api".to_owned(),
            database: Database::Postgres,
            features: Vec::new(),
            core_path: None,
            template_dir: Some(PathBuf::from(SQUELETTE)),
            lang: crate::lang::Lang::Fr,
        }
    }

    fn parent() -> TempDir {
        TempDir::new().expect("répertoire temporaire créable")
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("{} illisible : {error}", path.display());
        })
    }

    // Le refus vit dans la phase de vérification : rien n'est rendu, donc rien n'est
    // écrit. C'est ce que la seconde assertion mesure, et non le seul message.
    #[test]
    fn an_url_foreign_to_the_engine_is_refused_before_anything_is_written() {
        let parent = parent();
        let mut options = options("mon-api");
        options.database = Database::Mysql;

        let error = create(&options, parent.path()).expect_err("l'URL n'est pas une URL MySQL");

        let message = error.to_string();
        assert!(
            message.contains("mysql") && message.contains("postgres"),
            "le message ne nomme pas les deux côtés de l'écart : {message}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "un projet a été écrit malgré le refus"
        );
    }

    #[test]
    fn an_url_of_the_chosen_engine_is_accepted() {
        let parent = parent();
        let mut options = options("mon-api");
        options.database = Database::Sqlite;
        options.database_url = "sqlite://mon_api.db?mode=rwc".to_owned();

        create(&options, parent.path()).expect("l'URL désigne bien le moteur choisi");
    }

    // Le troisième critère de S1 : sans le flag, aucun projet existant ne change.
    #[test]
    fn without_the_flag_the_manifest_stays_on_postgres() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifeste = read(&project.root.join("Cargo.toml"));
        assert!(
            manifeste.contains("sqlx-postgres"),
            "le manifeste ne porte pas le pilote PostgreSQL :\n{manifeste}"
        );
        assert!(
            manifeste.contains("database = \"postgres\""),
            "le manifeste n'inscrit pas le moteur :\n{manifeste}"
        );
    }

    // Le noyau n'épingle plus de pilote : sans cette sélection, un projet MySQL
    // compilerait aussi celui de PostgreSQL, par unification des features de Cargo.
    #[test]
    fn the_core_dependency_selects_the_driver_of_the_chosen_engine() {
        for engine in Database::TOUS {
            let parent = parent();
            let mut options = options("mon-api");
            options.database = engine;
            options.database_url = engine.default_url("mon_api");

            let project = create(&options, parent.path()).expect("le projet doit se créer");

            let manifeste = read(&project.root.join("Cargo.toml"));
            let ligne = manifeste
                .lines()
                .find(|ligne| ligne.starts_with("rbs-core = "))
                .unwrap_or_else(|| panic!("`rbs-core` absente du manifeste :\n{manifeste}"));

            assert!(
                ligne.contains(&format!("\"{engine}\"")),
                "la dépendance au noyau ne choisit pas {engine} : {ligne}"
            );
            assert!(
                ligne.contains("default-features = false"),
                "les défauts du noyau rameneraient PostgreSQL : {ligne}"
            );
        }
    }

    #[test]
    fn each_engine_writes_its_own_driver_in_both_manifests() {
        for engine in Database::TOUS {
            let parent = parent();
            let mut options = options("mon-api");
            options.database = engine;
            options.database_url = engine.default_url("mon_api");

            let project = create(&options, parent.path()).expect("le projet doit se créer");

            for manifeste in ["Cargo.toml", "migration/Cargo.toml"] {
                let text = read(&project.root.join(manifeste));
                assert!(
                    text.contains(engine.sea_orm_feature()),
                    "{manifeste} ne porte pas `{}` pour {engine} :\n{text}",
                    engine.sea_orm_feature()
                );
            }

            let racine = read(&project.root.join("Cargo.toml"));
            assert!(
                racine.contains(&format!("database = \"{engine}\"")),
                "le manifeste n'inscrit pas `{engine}` :\n{racine}"
            );
        }
    }

    // L'URL du `.env` est celle qui sert à se connecter : un `.env` PostgreSQL dans un
    // projet SQLite se verrait au premier `rbs migrate up`, pas à la création.
    #[test]
    fn the_dotenv_carries_the_url_of_the_chosen_engine() {
        let parent = parent();
        let mut options = options("mon-api");
        options.database = Database::Sqlite;
        options.database_url = "sqlite://mon_api.db?mode=rwc".to_owned();

        let project = create(&options, parent.path()).expect("le projet doit se créer");

        for fichier in [".env", ".env.example"] {
            let text = read(&project.root.join(fichier));
            assert!(
                text.contains("sqlite://"),
                "{fichier} ne porte pas l'URL du moteur choisi :\n{text}"
            );
        }
    }

    #[test]
    fn the_full_skeleton_is_written_to_the_expected_paths() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert_eq!(project.root, parent.path().join("mon-api"));
        for relatif in [
            ".env",
            ".env.example",
            ".gitignore",
            "Cargo.toml",
            "config/default.toml",
            "config/development.toml",
            "config/production.toml",
            "migration/Cargo.toml",
            "migration/src/lib.rs",
            "migration/src/main.rs",
            "src/health/controller.rs",
            "src/health/mod.rs",
            "src/lib.rs",
            "src/seeds/main.rs",
            "src/main.rs",
            "src/openapi.rs",
            "src/router.rs",
            "src/state.rs",
        ] {
            assert!(
                project.root.join(relatif).is_file(),
                "{relatif} absent du projet créé"
            );
        }
    }

    #[test]
    fn the_migration_crate_exposes_a_binary_drivable_by_rbs_migrate() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifest = read(&project.root.join("migration/Cargo.toml"));
        assert!(
            manifest.contains("[[bin]]"),
            "la crate migration n'expose aucun binaire à envelopper"
        );
        assert!(
            manifest.contains("tokio"),
            "le binaire de migration n'a pas de runtime asynchrone"
        );
    }

    #[test]
    fn the_project_name_becomes_the_package_and_crate_name() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(
            read(&project.root.join("Cargo.toml")).contains("name = \"mon-api\""),
            "le manifeste ne porte pas le nom du projet"
        );
        // Un tiret n'est pas un caractère d'identifiant Rust : les filtres de log visent
        // la crate, pas le paquet.
        assert!(
            read(&project.root.join(".env")).contains("RUST_LOG=info,mon_api=debug"),
            "le filtre de log ne vise pas la crate"
        );
    }

    #[test]
    fn the_env_file_carries_the_chosen_url_while_the_example_stays_generic() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(
            read(&project.root.join(".env"))
                .contains("RBS_DATABASE__URL=postgres://alice:s3cr3t@localhost:5432/api"),
            "l'URL choisie n'est pas dans le `.env`"
        );
        let exemple = read(&project.root.join(".env.example"));
        assert!(
            !exemple.contains("s3cr3t"),
            "le `.env.example`, versionné, porte le mot de passe de l'utilisateur :\n{exemple}"
        );
    }

    #[test]
    fn the_metadata_of_the_created_project_reads_back() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let metadonnees =
            crate::metadata::read(&project.root.join("Cargo.toml")).expect("métadonnées lisibles");
        assert_eq!(metadonnees.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(metadonnees.features, vec!["health".to_string()]);
    }

    /// Les tests que `rbs generate crud` pose traversent le routeur sans réseau et
    /// relisent du JSON : sans ces deux crates, le projet généré ne compilerait pas ses
    /// propres tests.
    #[test]
    fn the_manifest_carries_the_dev_dependencies_of_the_generated_tests() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifest = read(&project.root.join("Cargo.toml"));
        assert!(
            manifest.contains("[dev-dependencies]"),
            "section absente :\n{manifest}"
        );
        for dependency in ["tower = ", "serde_json = ", "uuid = "] {
            assert!(
                manifest.contains(dependency),
                "`{dependency}` absente :\n{manifest}"
            );
        }
    }

    // L'identifiant est désormais posé par le modèle : `uuid` sert au code de production
    // et non plus aux seuls tests, et c'est `v7` qui le fournit.
    #[test]
    fn uuid_is_a_production_dependency_carrying_v7() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifest = read(&project.root.join("Cargo.toml"));
        let (production, dev) = manifest
            .split_once("[dev-dependencies]")
            .expect("le manifeste porte ses deux sections");

        assert!(
            production.contains("uuid = "),
            "`uuid` n'est pas une dépendance de production :\n{manifest}"
        );
        assert!(
            !dev.contains("uuid = "),
            "`uuid` est déclarée deux fois :\n{manifest}"
        );
        assert!(
            production.contains("\"v7\""),
            "la feature `v7` manque :\n{manifest}"
        );
        // Les tests générés tirent leurs valeurs aléatoires avec `new_v4`.
        assert!(
            production.contains("\"v4\""),
            "la feature `v4` manque aux tests générés :\n{manifest}"
        );
    }

    #[test]
    fn without_core_path_the_manifest_depends_on_the_published_core_version() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifest = read(&project.root.join("Cargo.toml"));
        let expected = format!("version = \"{}\"", env!("CARGO_PKG_VERSION"));
        assert!(
            manifest.contains(&expected),
            "`{expected}` absent du manifeste :\n{manifest}"
        );
    }

    #[test]
    fn with_core_path_the_manifest_depends_on_the_local_core_by_an_absolute_path() {
        let parent = parent();
        let core = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../rbs-core"));
        let mut options = options("mon-api");
        options.core_path = Some(core.clone());

        let project = create(&options, parent.path()).expect("le projet doit se créer");

        let brut = read(&project.root.join("Cargo.toml"));
        let manifest: toml_edit::DocumentMut = brut
            .parse()
            .unwrap_or_else(|error| panic!("manifeste illisible comme TOML : {error}\n{brut}"));
        let absolu = core.canonicalize().expect("le noyau existe");

        // Le chemin se compare après analyse, jamais sur le texte du manifeste : un
        // chemin Windows y est inscrit avec ses antislashs échappés, et une comparaison
        // textuelle parlerait de l'échappement au lieu de parler du chemin.
        //
        // Cargo résout un chemin relatif depuis le manifeste du projet créé, pas depuis
        // le répertoire où la commande a été lancée : d'où l'absolu.
        let inscrit = manifest["dependencies"]["rbs-core"]["path"]
            .as_str()
            .unwrap_or_else(|| panic!("la dépendance au noyau doit porter un `path` :\n{brut}"));
        assert_eq!(Path::new(inscrit), absolu);
    }

    #[test]
    fn an_unfindable_core_path_is_rejected_before_any_creation() {
        let parent = parent();
        let mut options = options("mon-api");
        options.core_path = Some(PathBuf::from("/introuvable/rbs-core"));

        let error = create(&options, parent.path()).expect_err("un noyau absent doit être refusé");

        assert!(
            error.to_string().contains("/introuvable/rbs-core"),
            "le message ne nomme pas le chemin : {error}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "un projet a été créé malgré l'échec"
        );
    }

    #[test]
    fn an_unknown_feature_is_refused_before_anything_is_written() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let error = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["graphql".to_string()],
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect_err("`graphql` n'est pas une feature");

        let message = error.to_string();
        assert!(message.contains("graphql"), "{message}");
        assert!(
            message.contains("jobs"),
            "la liste doit être complète : {message}"
        );
        assert!(
            !parent.path().join("demo").exists(),
            "rien ne doit être écrit"
        );
    }

    /// `jobs` était refusé par une liste qui l'avait oublié, alors que `rbs add jobs`
    /// fonctionnait.
    #[test]
    fn every_embedded_feature_is_accepted_by_name() {
        for feature in crate::templates::feature_names(None) {
            let parent = TempDir::new().expect("répertoire temporaire créable");

            create(
                &Options {
                    name: "demo".to_string(),
                    database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                    database: Database::Postgres,
                    features: vec![feature.clone()],
                    core_path: None,
                    template_dir: None,
                    lang: crate::lang::Lang::Fr,
                },
                parent.path(),
            )
            .unwrap_or_else(|error| panic!("`{feature}` doit s'installer : {error}"));
        }
    }

    #[test]
    fn a_requested_feature_is_actually_installed() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["auth".to_string()],
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(project.root.join("src/auth/service.rs").is_file());

        let lib = fs::read_to_string(project.root.join("src/lib.rs")).expect("lib lisible");
        assert!(lib.contains("pub mod auth;"), "{lib}");

        let manifest = fs::read_to_string(project.root.join("Cargo.toml")).expect("manifeste");
        assert!(manifest.contains("\"auth\""), "{manifest}");

        assert_eq!(project.installed.len(), 1);
        assert_eq!(project.installed[0].name, "auth");
        assert!(project.installed[0].migration);
    }

    /// L'ordre de frappe ne doit pas décider du contenu : deux `--with` équivalents
    /// produisent deux projets identiques.
    #[test]
    fn the_install_order_does_not_depend_on_the_typing_order() {
        let rendu = |features: Vec<String>| {
            let parent = TempDir::new().expect("répertoire temporaire créable");
            let project = create(
                &Options {
                    name: "demo".to_string(),
                    database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                    database: Database::Postgres,
                    features,
                    core_path: None,
                    template_dir: None,
                    lang: crate::lang::Lang::Fr,
                },
                parent.path(),
            )
            .expect("le projet doit se créer");

            let lib = fs::read_to_string(project.root.join("src/lib.rs")).expect("lib");
            let compose =
                fs::read_to_string(project.root.join("docker-compose.yml")).expect("compose");
            (lib, compose)
        };

        assert_eq!(
            rendu(vec!["redis".into(), "mail".into()]),
            rendu(vec!["mail".into(), "redis".into()])
        );
    }

    #[test]
    fn a_failed_installation_leaves_no_project_behind() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let directory = TempDir::new().expect("répertoire temporaire créable");
        // Un fragment vide : son manifeste est illisible, l'installation échoue.
        fs::create_dir(directory.path().join("cassee")).expect("répertoire créable");
        fs::write(
            directory.path().join("cassee/feature.toml"),
            "pas du toml [",
        )
        .expect("écriture possible");

        let error = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["cassee".to_string()],
                core_path: None,
                template_dir: Some(directory.path().to_path_buf()),
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect_err("le fragment est cassé");

        assert!(error.to_string().contains("cassee"), "{error}");
        assert!(
            !parent.path().join("demo").exists(),
            "le projet à moitié installé ne doit pas subsister"
        );
    }

    #[test]
    fn a_name_that_is_not_a_cargo_package_is_rejected() {
        let parent = parent();

        // Un nom traverse jusqu'au `name` du manifeste et jusqu'au chemin créé : un
        // espace produit un TOML invalide, `..` écrit hors du répertoire visé.
        for name in ["mon api", "../evasion", "3volution", ""] {
            let resultat = create(&options(name), parent.path());

            assert!(
                resultat.is_err(),
                "`{name}` a été accepté comme nom de projet"
            );
        }

        let restes: Vec<_> = fs::read_dir(parent.path())
            .expect("répertoire lisible")
            .map(|input| input.expect("entrée lisible").path())
            .collect();
        assert!(restes.is_empty(), "des fichiers ont été créés : {restes:?}");
    }

    #[test]
    fn an_occupied_directory_is_rejected_without_writing_anything() {
        let parent = parent();
        let occupe = parent.path().join("mon-api");
        fs::create_dir(&occupe).expect("répertoire créable");
        fs::write(occupe.join("travail.rs"), "à ne pas perdre").expect("fichier écrit");

        let error = create(&options("mon-api"), parent.path())
            .expect_err("un répertoire occupé est refusé");

        assert!(
            error.to_string().contains("mon-api"),
            "le message ne nomme pas le répertoire : {error}"
        );
        assert_eq!(read(&occupe.join("travail.rs")), "à ne pas perdre");
        assert!(
            !occupe.join("Cargo.toml").exists(),
            "un fichier du squelette a été écrit dans le répertoire occupé"
        );
    }

    #[test]
    fn a_failing_render_leaves_no_partial_project() {
        let parent = parent();
        let templates = parent.path().join("templates");
        fs::create_dir(&templates).expect("répertoire créable");
        fs::write(
            templates.join("Cargo.toml.jinja"),
            "name = \"{@ project_name @}\"",
        )
        .expect("template écrite");
        fs::write(templates.join("src.rs.jinja"), "{@ variable_absente @}")
            .expect("template écrite");
        let mut options = options("mon-api");
        options.template_dir = Some(templates);

        let error = create(&options, parent.path())
            .expect_err("une variable absente doit arrêter la génération");

        assert!(
            error.to_string().contains("src.rs"),
            "le message ne nomme pas la template fautive : {error}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "le projet à moitié écrit n'a pas été retiré"
        );
    }

    #[test]
    fn the_created_project_is_a_git_repository() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(project.depot_git, "`git init` non signalé dans le rapport");
        assert!(
            project.root.join(".git").exists(),
            "le projet créé n'est pas un dépôt"
        );
    }

    /// Le compose n'est utile que s'il évite un `docker run` tapé à la main : c'est le
    /// seul critère qui décide de son écriture.
    #[test]
    fn a_local_postgres_project_gets_a_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:secret@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
            .expect("le compose doit être écrit");

        assert!(compose.contains("POSTGRES_USER: rbs"), "{compose}");
        assert!(compose.contains("POSTGRES_PASSWORD: secret"), "{compose}");
        assert!(compose.contains("POSTGRES_DB: demo"), "{compose}");
        assert!(compose.contains("- \"5432:5432\""), "{compose}");
        assert!(compose.contains("# <rbs:services>"), "{compose}");
        assert!(compose.contains("# </rbs:services>"), "{compose}");
        assert_eq!(project.files, 20);
    }

    /// Le port publié est celui du .env, non 5432 en dur : sans quoi `cargo run` sur
    /// l'hôte joindrait un port que le conteneur n'expose pas.
    #[test]
    fn the_published_port_is_the_one_the_project_will_dial() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:15432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
            .expect("le compose doit être écrit");

        assert!(compose.contains("- \"15432:5432\""), "{compose}");
    }

    /// Une URL sans nom de base laisse `POSTGRES_DB:` vide, donc un service que le
    /// healthcheck ne déclare jamais sain : le nom du projet prend le relais.
    #[test]
    fn a_url_without_a_database_names_the_one_the_project_will_open() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "mon-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
            .expect("le compose doit être écrit");

        assert!(compose.contains("POSTGRES_DB: mon_api"), "{compose}");
    }

    #[test]
    fn a_sqlite_project_gets_no_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "sqlite://demo.db?mode=rwc".to_string(),
                database: Database::Sqlite,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(!project.root.join("docker-compose.yml").exists());
        assert_eq!(project.files, 19);
    }

    #[test]
    fn a_remote_database_gets_no_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@db.prod.exemple:5432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(!project.root.join("docker-compose.yml").exists());
        assert_eq!(project.files, 19);
    }

    /// Une URL sans identifiants est valide et acceptée par `parse` : sans cette
    /// abstention, le compose porterait `POSTGRES_USER:` et `POSTGRES_PASSWORD:` vides,
    /// rendus `null` par `docker compose config` — l'image officielle refuse alors de
    /// s'initialiser. Écrire un compose qui ne peut pas démarrer est pire que ne rien
    /// écrire, la même raison que pour SQLite et l'hôte distant.
    #[test]
    fn a_database_url_without_credentials_gets_no_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(!project.root.join("docker-compose.yml").exists());
        assert_eq!(project.files, 19);
    }

    #[test]
    fn a_new_project_carries_its_agents_file() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let project = create(
            &Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let agents =
            std::fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md est écrit");

        assert!(agents.contains("<!-- rbs:guide"), "{agents}");
        assert!(agents.contains("rbs generate crud"), "{agents}");
    }

    /// L'inventaire lit le manifeste, que l'installation des features complète : écrit
    /// avant elle, il annoncerait un projet sans `auth` sur un projet qui vient de
    /// l'installer.
    #[test]
    fn the_inventory_of_a_new_project_names_the_features_installed_at_creation() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let project = create(
            &Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: vec!["redis".to_string()],
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let agents =
            std::fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md est écrit");

        assert!(agents.contains("redis"), "{agents}");
    }

    #[test]
    fn an_english_project_carries_an_english_guide() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let project = create(
            &Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::En,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let agents =
            std::fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md est écrit");

        assert!(agents.contains("## CLI first"), "{agents}");
    }

    /// `--template-dir` ne remplace que le squelette de projet : les guides `AGENTS.md`
    /// n'en font pas partie, et rien n'oblige un répertoire de substitution à les fournir.
    /// Sans ce test, une régression qui referait lire `template_dir` par
    /// `crate::templates::Source::agents` casserait tout `rbs new --template-dir`
    /// pointant sur un squelette seul — exactement le cas documenté — sans qu'aucun test
    /// existant ne s'en aperçoive.
    #[test]
    fn a_template_dir_holding_only_the_skeleton_still_gets_its_agents_file() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let project = create(
            &Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: Some(PathBuf::from(SQUELETTE)),
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("un `--template-dir` qui ne porte que le squelette doit suffire à créer le projet");

        let agents = std::fs::read_to_string(project.root.join("AGENTS.md"))
            .expect("AGENTS.md est écrit même quand `--template-dir` ne porte pas les guides");

        assert!(agents.contains("<!-- rbs:guide"), "{agents}");
    }
}
