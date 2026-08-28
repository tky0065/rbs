//! Création d'un projet complet : squelette rendu, arborescence écrite, dépôt initialisé.
//!
//! La commande suit la séquence du §4.4 de la spec, dans l'ordre où elle rend les échecs
//! inoffensifs : ce qui peut être vérifié l'est avant que le rendu commence, et le rendu
//! aboutit entièrement avant que le premier fichier soit écrit. Un nom refusé, une
//! feature indisponible ou une variable de template absente laissent donc le disque
//! exactement dans l'état où ils l'ont trouvé.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use minijinja::context;

use crate::template::Renderer;
use crate::templates::Source;

/// Features que les questions de création proposent, et que `rbs add` installe.
///
/// Elle double la liste que `add` tire des fragments embarqués : une feature absente
/// d'ici est refusée à la création comme si rbs ne la connaissait pas.
const FEATURES_CONNUES: &[&str] = &["docker", "ci", "auth", "redis", "storage", "mail"];

/// Ce qu'il faut savoir avant de créer un projet, questions et flags confondus.
pub struct Options {
    /// Nom du projet, qui est aussi celui du répertoire et du paquet Cargo.
    pub name: String,
    /// URL de connexion écrite dans le `.env` du projet.
    pub database_url: String,
    /// Features demandées à la création.
    pub features: Vec<String>,
    /// Noyau local à utiliser au lieu de la version publiée.
    pub core_path: Option<PathBuf>,
    /// Templates du disque remplaçant celles embarquées.
    pub template_dir: Option<PathBuf>,
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

    /// La feature existe, mais `--with` ne l'installe pas à la création.
    #[error(
        "`{feature}` ne s'installe pas à la création : créez le projet sans `--with`, \
         puis `rbs add {feature}`"
    )]
    FeatureAVenir {
        /// Feature demandée.
        feature: String,
    },

    /// La feature demandée n'existe pas.
    #[error("`{feature}` n'est pas une feature rbs — disponibles : {known}")]
    FeatureInconnue {
        /// Feature demandée.
        feature: String,
        /// Features que rbs connaît.
        known: String,
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
}

/// Crée le projet décrit par `options` dans `parent`.
///
/// # Erreurs
///
/// Échoue si le nom, les features ou le chemin visé sont inutilisables, si une template
/// ne se rend pas, ou si l'écriture échoue. Dans tous les cas, rien de ce que la commande
/// a créé ne subsiste.
pub fn create(options: &Options, parent: &Path) -> Result<Project, Error> {
    validate_name(&options.name)?;
    validate_features(&options.features)?;

    let root = parent.join(&options.name);
    if root.exists() {
        return Err(Error::RepertoireOccupe {
            path: root.display().to_string(),
        });
    }

    let dependency = core_dependency(options.core_path.as_deref())?;
    let rendus = render(options, &dependency)?;

    write(&root, &rendus).map_err(|(path, source)| {
        // Le répertoire n'existait pas : le retirer entièrement ne peut rien emporter
        // qui préexistait à la commande.
        let _ = fs::remove_dir_all(&root);
        Error::Ecriture { path, source }
    })?;

    Ok(Project {
        depot_git: git_init(&root),
        files: rendus.len(),
        root,
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

/// `--with` n'installe rien : il nomme des features que seul `rbs add` pose.
///
/// Les inscrire dans `[package.metadata.rbs]` sans rien poser rendrait leur installation
/// ultérieure impossible : l'idempotence du §4.2 porte sur ces métadonnées, pas sur la
/// présence des fichiers.
fn validate_features(features: &[String]) -> Result<(), Error> {
    match features.first() {
        None => Ok(()),
        Some(feature) if FEATURES_CONNUES.contains(&feature.as_str()) => {
            Err(Error::FeatureAVenir {
                feature: feature.clone(),
            })
        }
        Some(feature) => Err(Error::FeatureInconnue {
            feature: feature.clone(),
            known: FEATURES_CONNUES.join(", "),
        }),
    }
}

/// Valeur de la dépendance à `rbs-core` dans le manifeste généré.
///
/// Le chemin est canonisé : Cargo le résout depuis le manifeste du projet créé, pas
/// depuis le répertoire où la commande a été lancée.
fn core_dependency(core_path: Option<&Path>) -> Result<String, Error> {
    let Some(path) = core_path else {
        return Ok(format!("\"{}\"", env!("CARGO_PKG_VERSION")));
    };

    let absolu = path
        .canonicalize()
        .map_err(|source| Error::NoyauIntrouvable {
            path: path.display().to_string(),
            source,
        })?;

    let value = toml_edit::Value::from(absolu.display().to_string());

    Ok(format!("{{ path = {} }}", value.to_string().trim()))
}

/// Rend toutes les templates. Aucun fichier n'est écrit tant que la dernière n'a pas
/// abouti : une variable oubliée ne doit pas laisser un projet à moitié généré.
fn render(options: &Options, dependency: &str) -> Result<Vec<(PathBuf, String)>, Error> {
    let files = Source::fresh(options.template_dir.as_deref())
        .files()
        .map_err(Error::Templates)?;

    let renderer = Renderer::new();
    let context = context! {
        project_name => options.name.as_str(),
        crate_name => crate_name(&options.name),
        rbs_core_dep => dependency,
        rbs_version => env!("CARGO_PKG_VERSION"),
        database_url => options.database_url.as_str(),
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
            features: Vec::new(),
            core_path: None,
            template_dir: Some(PathBuf::from(SQUELETTE)),
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
            "migration/Cargo.toml",
            "migration/src/lib.rs",
            "migration/src/main.rs",
            "src/health/controller.rs",
            "src/health/mod.rs",
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

    #[test]
    fn without_core_path_the_manifest_depends_on_the_published_core_version() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifest = read(&project.root.join("Cargo.toml"));
        let expected = format!("rbs-core = \"{}\"", env!("CARGO_PKG_VERSION"));
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
    fn a_non_installable_feature_is_rejected_before_any_creation() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["docker".to_owned()];

        let error =
            create(&options, parent.path()).expect_err("`docker` n'est pas encore installable");

        let message = error.to_string();
        assert!(
            message.contains("docker") && message.contains("rbs add"),
            "le message ne dit pas comment obtenir la feature : {message}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "un projet a été créé malgré l'échec"
        );
    }

    #[test]
    fn every_feature_add_installs_is_known_at_creation() {
        // Les deux listes se sont désynchronisées une fois : `auth` livrée par `add`,
        // et refusée ici comme si elle n'existait pas.
        for feature in ["docker", "ci", "auth"] {
            let parent = parent();
            let mut options = options("mon-api");
            options.features = vec![feature.to_owned()];

            let error = create(&options, parent.path())
                .expect_err("`--with` n'installe aucune feature à la création");
            let message = error.to_string();

            assert!(
                message.contains(&format!("rbs add {feature}")),
                "le message ne renvoie pas vers `rbs add {feature}` : {message}"
            );
            assert!(
                !message.contains("n'est pas une feature rbs"),
                "`{feature}` est traitée comme inconnue : {message}"
            );
        }
    }

    #[test]
    fn the_pointer_to_add_does_not_pretend_the_command_is_missing() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["auth".to_owned()];

        let error = create(&options, parent.path()).expect_err("`--with` n'installe rien");

        // `add` expose les trois features depuis le lot I. Le message qui annonçait le
        // contraire envoyait le lecteur attendre une commande déjà livrée.
        assert!(
            !error.to_string().contains("n'expose pas"),
            "le message dit encore qu'`add` n'expose pas la feature : {error}"
        );
    }

    #[test]
    fn an_unknown_feature_is_not_confused_with_an_upcoming_one() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["kubernetes".to_owned()];

        let error = create(&options, parent.path()).expect_err("`kubernetes` n'existe pas");

        let message = error.to_string();
        assert!(
            message.contains("kubernetes") && message.contains("docker"),
            "le message ne nomme pas les features existantes : {message}"
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
}
