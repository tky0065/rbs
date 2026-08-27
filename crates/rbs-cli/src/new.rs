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
    pub nom: String,
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
pub struct Projet {
    /// Racine du projet créé.
    pub racine: PathBuf,
    /// Nombre de fichiers écrits.
    pub fichiers: usize,
    /// `git init` a abouti. Faux n'invalide pas le projet.
    pub depot_git: bool,
}

/// Ce qui peut empêcher la création d'un projet.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Le nom ne peut être ni un paquet Cargo ni un répertoire.
    #[error(
        "`{nom}` n'est pas un nom de projet utilisable : lettres, chiffres, `-` et `_`, \
         en commençant par une lettre"
    )]
    NomInvalide {
        /// Nom refusé.
        nom: String,
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
    #[error("`{feature}` n'est pas une feature rbs — disponibles : {connues}")]
    FeatureInconnue {
        /// Feature demandée.
        feature: String,
        /// Features que rbs connaît.
        connues: String,
    },

    /// Le chemin visé est déjà pris.
    #[error("{chemin} existe déjà : choisissez un autre nom, ou retirez ce répertoire")]
    RepertoireOccupe {
        /// Chemin visé.
        chemin: String,
    },

    /// `--core-path` ne désigne pas un répertoire lisible.
    #[error("{chemin} est introuvable : `--core-path` désigne la crate `rbs-core` ({source})")]
    NoyauIntrouvable {
        /// Chemin donné.
        chemin: String,
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
    #[error("écriture impossible dans {chemin} : {source}")]
    Ecriture {
        /// Chemin en cause.
        chemin: String,
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
pub fn creer(options: &Options, parent: &Path) -> Result<Projet, Erreur> {
    valider_nom(&options.nom)?;
    valider_features(&options.features)?;

    let racine = parent.join(&options.nom);
    if racine.exists() {
        return Err(Erreur::RepertoireOccupe {
            chemin: racine.display().to_string(),
        });
    }

    let dependance = dependance_noyau(options.core_path.as_deref())?;
    let rendus = rendre(options, &dependance)?;

    ecrire(&racine, &rendus).map_err(|(chemin, source)| {
        // Le répertoire n'existait pas : le retirer entièrement ne peut rien emporter
        // qui préexistait à la commande.
        let _ = fs::remove_dir_all(&racine);
        Erreur::Ecriture { chemin, source }
    })?;

    Ok(Projet {
        depot_git: git_init(&racine),
        fichiers: rendus.len(),
        racine,
    })
}

/// Le nom devient un `name` de manifeste et un nom de répertoire : ce qui n'est pas
/// valide pour les deux est refusé avant que quoi que ce soit s'écrive.
fn valider_nom(nom: &str) -> Result<(), Erreur> {
    let utilisable = nom.starts_with(|premier: char| premier.is_ascii_alphabetic())
        && nom.chars().all(|caractere| {
            caractere.is_ascii_alphanumeric() || caractere == '-' || caractere == '_'
        });

    if utilisable {
        Ok(())
    } else {
        Err(Erreur::NomInvalide {
            nom: nom.to_owned(),
        })
    }
}

/// `--with` n'installe rien : il nomme des features que seul `rbs add` pose.
///
/// Les inscrire dans `[package.metadata.rbs]` sans rien poser rendrait leur installation
/// ultérieure impossible : l'idempotence du §4.2 porte sur ces métadonnées, pas sur la
/// présence des fichiers.
fn valider_features(features: &[String]) -> Result<(), Erreur> {
    match features.first() {
        None => Ok(()),
        Some(feature) if FEATURES_CONNUES.contains(&feature.as_str()) => {
            Err(Erreur::FeatureAVenir {
                feature: feature.clone(),
            })
        }
        Some(feature) => Err(Erreur::FeatureInconnue {
            feature: feature.clone(),
            connues: FEATURES_CONNUES.join(", "),
        }),
    }
}

/// Valeur de la dépendance à `rbs-core` dans le manifeste généré.
///
/// Le chemin est canonisé : Cargo le résout depuis le manifeste du projet créé, pas
/// depuis le répertoire où la commande a été lancée.
fn dependance_noyau(core_path: Option<&Path>) -> Result<String, Erreur> {
    let Some(chemin) = core_path else {
        return Ok(format!("\"{}\"", env!("CARGO_PKG_VERSION")));
    };

    let absolu = chemin
        .canonicalize()
        .map_err(|source| Erreur::NoyauIntrouvable {
            chemin: chemin.display().to_string(),
            source,
        })?;

    let valeur = toml_edit::Value::from(absolu.display().to_string());

    Ok(format!("{{ path = {} }}", valeur.to_string().trim()))
}

/// Rend toutes les templates. Aucun fichier n'est écrit tant que la dernière n'a pas
/// abouti : une variable oubliée ne doit pas laisser un projet à moitié généré.
fn rendre(options: &Options, dependance: &str) -> Result<Vec<(PathBuf, String)>, Erreur> {
    let fichiers = Source::nouvelle(options.template_dir.as_deref())
        .fichiers()
        .map_err(Erreur::Templates)?;

    let renderer = Renderer::new();
    let contexte = context! {
        nom_projet => options.nom.as_str(),
        nom_crate => nom_crate(&options.nom),
        rbs_core_dep => dependance,
        rbs_version => env!("CARGO_PKG_VERSION"),
        database_url => options.database_url.as_str(),
    };

    fichiers
        .into_iter()
        .map(|fichier| {
            let rendu = renderer
                .rendre(&fichier.source, &contexte)
                .map_err(|source| Erreur::Rendu {
                    template: fichier.destination.display().to_string(),
                    source,
                })?;

            Ok((fichier.destination, rendu))
        })
        .collect()
}

/// Écrit l'arborescence, en nommant le chemin qui a échoué.
fn ecrire(racine: &Path, rendus: &[(PathBuf, String)]) -> Result<(), (String, io::Error)> {
    for (destination, contenu) in rendus {
        let chemin = racine.join(destination);

        if let Some(parent) = chemin.parent() {
            fs::create_dir_all(parent).map_err(|erreur| (parent.display().to_string(), erreur))?;
        }

        fs::write(&chemin, contenu).map_err(|erreur| (chemin.display().to_string(), erreur))?;
    }

    Ok(())
}

/// Initialise le dépôt du projet créé.
///
/// L'échec n'est pas fatal : un projet sans dépôt reste un projet valide, et `git` peut
/// tout simplement ne pas être installé.
fn git_init(racine: &Path) -> bool {
    Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(racine)
        .status()
        .is_ok_and(|statut| statut.success())
}

/// Nom de la crate correspondant au nom du projet : un tiret n'est pas un caractère
/// d'identifiant Rust.
fn nom_crate(nom: &str) -> String {
    nom.replace('-', "_")
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

    fn options(nom: &str) -> Options {
        Options {
            nom: nom.to_owned(),
            database_url: "postgres://alice:s3cr3t@localhost:5432/api".to_owned(),
            features: Vec::new(),
            core_path: None,
            template_dir: Some(PathBuf::from(SQUELETTE)),
        }
    }

    fn parent() -> TempDir {
        TempDir::new().expect("répertoire temporaire créable")
    }

    fn lire(chemin: &Path) -> String {
        fs::read_to_string(chemin).unwrap_or_else(|erreur| {
            panic!("{} illisible : {erreur}", chemin.display());
        })
    }

    #[test]
    fn le_squelette_complet_est_ecrit_aux_chemins_attendus() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert_eq!(projet.racine, parent.path().join("mon-api"));
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
            "src/main.rs",
            "src/openapi.rs",
            "src/router.rs",
            "src/state.rs",
        ] {
            assert!(
                projet.racine.join(relatif).is_file(),
                "{relatif} absent du projet créé"
            );
        }
    }

    #[test]
    fn la_crate_migration_expose_un_binaire_pilotable_par_rbs_migrate() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifeste = lire(&projet.racine.join("migration/Cargo.toml"));
        assert!(
            manifeste.contains("[[bin]]"),
            "la crate migration n'expose aucun binaire à envelopper"
        );
        assert!(
            manifeste.contains("tokio"),
            "le binaire de migration n'a pas de runtime asynchrone"
        );
    }

    #[test]
    fn le_nom_du_projet_devient_celui_du_paquet_et_de_la_crate() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(
            lire(&projet.racine.join("Cargo.toml")).contains("name = \"mon-api\""),
            "le manifeste ne porte pas le nom du projet"
        );
        // Un tiret n'est pas un caractère d'identifiant Rust : les filtres de log visent
        // la crate, pas le paquet.
        assert!(
            lire(&projet.racine.join(".env")).contains("RUST_LOG=info,mon_api=debug"),
            "le filtre de log ne vise pas la crate"
        );
    }

    #[test]
    fn le_fichier_env_porte_l_url_choisie_quand_l_exemple_reste_generique() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(
            lire(&projet.racine.join(".env"))
                .contains("RBS_DATABASE__URL=postgres://alice:s3cr3t@localhost:5432/api"),
            "l'URL choisie n'est pas dans le `.env`"
        );
        let exemple = lire(&projet.racine.join(".env.example"));
        assert!(
            !exemple.contains("s3cr3t"),
            "le `.env.example`, versionné, porte le mot de passe de l'utilisateur :\n{exemple}"
        );
    }

    #[test]
    fn les_metadonnees_du_projet_cree_se_relisent() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let metadonnees =
            crate::metadata::lire(&projet.racine.join("Cargo.toml")).expect("métadonnées lisibles");
        assert_eq!(metadonnees.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(metadonnees.features, vec!["health".to_string()]);
    }

    /// Les tests que `rbs generate crud` pose traversent le routeur sans réseau et
    /// relisent du JSON : sans ces deux crates, le projet généré ne compilerait pas ses
    /// propres tests.
    #[test]
    fn le_manifeste_porte_les_dependances_de_developpement_des_tests_generes() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifeste = lire(&projet.racine.join("Cargo.toml"));
        assert!(
            manifeste.contains("[dev-dependencies]"),
            "section absente :\n{manifeste}"
        );
        for dependance in ["tower = ", "serde_json = ", "uuid = "] {
            assert!(
                manifeste.contains(dependance),
                "`{dependance}` absente :\n{manifeste}"
            );
        }
    }

    #[test]
    fn sans_core_path_le_manifeste_depend_de_la_version_publiee_du_noyau() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let manifeste = lire(&projet.racine.join("Cargo.toml"));
        let attendu = format!("rbs-core = \"{}\"", env!("CARGO_PKG_VERSION"));
        assert!(
            manifeste.contains(&attendu),
            "`{attendu}` absent du manifeste :\n{manifeste}"
        );
    }

    #[test]
    fn avec_core_path_le_manifeste_depend_du_noyau_local_par_un_chemin_absolu() {
        let parent = parent();
        let noyau = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../rbs-core"));
        let mut options = options("mon-api");
        options.core_path = Some(noyau.clone());

        let projet = creer(&options, parent.path()).expect("le projet doit se créer");

        let brut = lire(&projet.racine.join("Cargo.toml"));
        let manifeste: toml_edit::DocumentMut = brut
            .parse()
            .unwrap_or_else(|erreur| panic!("manifeste illisible comme TOML : {erreur}\n{brut}"));
        let absolu = noyau.canonicalize().expect("le noyau existe");

        // Le chemin se compare après analyse, jamais sur le texte du manifeste : un
        // chemin Windows y est inscrit avec ses antislashs échappés, et une comparaison
        // textuelle parlerait de l'échappement au lieu de parler du chemin.
        //
        // Cargo résout un chemin relatif depuis le manifeste du projet créé, pas depuis
        // le répertoire où la commande a été lancée : d'où l'absolu.
        let inscrit = manifeste["dependencies"]["rbs-core"]["path"]
            .as_str()
            .unwrap_or_else(|| panic!("la dépendance au noyau doit porter un `path` :\n{brut}"));
        assert_eq!(Path::new(inscrit), absolu);
    }

    #[test]
    fn un_core_path_introuvable_est_refuse_avant_toute_creation() {
        let parent = parent();
        let mut options = options("mon-api");
        options.core_path = Some(PathBuf::from("/introuvable/rbs-core"));

        let erreur = creer(&options, parent.path()).expect_err("un noyau absent doit être refusé");

        assert!(
            erreur.to_string().contains("/introuvable/rbs-core"),
            "le message ne nomme pas le chemin : {erreur}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "un projet a été créé malgré l'échec"
        );
    }

    #[test]
    fn une_feature_non_installable_est_refusee_avant_toute_creation() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["docker".to_owned()];

        let erreur =
            creer(&options, parent.path()).expect_err("`docker` n'est pas encore installable");

        let message = erreur.to_string();
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
    fn toute_feature_qu_add_installe_est_connue_a_la_creation() {
        // Les deux listes se sont désynchronisées une fois : `auth` livrée par `add`,
        // et refusée ici comme si elle n'existait pas.
        for feature in ["docker", "ci", "auth"] {
            let parent = parent();
            let mut options = options("mon-api");
            options.features = vec![feature.to_owned()];

            let erreur = creer(&options, parent.path())
                .expect_err("`--with` n'installe aucune feature à la création");
            let message = erreur.to_string();

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
    fn le_renvoi_vers_add_ne_pretend_pas_que_la_commande_manque() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["auth".to_owned()];

        let erreur = creer(&options, parent.path()).expect_err("`--with` n'installe rien");

        // `add` expose les trois features depuis le lot I. Le message qui annonçait le
        // contraire envoyait le lecteur attendre une commande déjà livrée.
        assert!(
            !erreur.to_string().contains("n'expose pas"),
            "le message dit encore qu'`add` n'expose pas la feature : {erreur}"
        );
    }

    #[test]
    fn une_feature_inconnue_ne_se_confond_pas_avec_une_feature_a_venir() {
        let parent = parent();
        let mut options = options("mon-api");
        options.features = vec!["kubernetes".to_owned()];

        let erreur = creer(&options, parent.path()).expect_err("`kubernetes` n'existe pas");

        let message = erreur.to_string();
        assert!(
            message.contains("kubernetes") && message.contains("docker"),
            "le message ne nomme pas les features existantes : {message}"
        );
    }

    #[test]
    fn un_nom_qui_n_est_pas_un_paquet_cargo_est_refuse() {
        let parent = parent();

        // Un nom traverse jusqu'au `name` du manifeste et jusqu'au chemin créé : un
        // espace produit un TOML invalide, `..` écrit hors du répertoire visé.
        for nom in ["mon api", "../evasion", "3volution", ""] {
            let resultat = creer(&options(nom), parent.path());

            assert!(
                resultat.is_err(),
                "`{nom}` a été accepté comme nom de projet"
            );
        }

        let restes: Vec<_> = fs::read_dir(parent.path())
            .expect("répertoire lisible")
            .map(|entree| entree.expect("entrée lisible").path())
            .collect();
        assert!(restes.is_empty(), "des fichiers ont été créés : {restes:?}");
    }

    #[test]
    fn un_repertoire_occupe_est_refuse_sans_rien_ecrire() {
        let parent = parent();
        let occupe = parent.path().join("mon-api");
        fs::create_dir(&occupe).expect("répertoire créable");
        fs::write(occupe.join("travail.rs"), "à ne pas perdre").expect("fichier écrit");

        let erreur =
            creer(&options("mon-api"), parent.path()).expect_err("un répertoire occupé est refusé");

        assert!(
            erreur.to_string().contains("mon-api"),
            "le message ne nomme pas le répertoire : {erreur}"
        );
        assert_eq!(lire(&occupe.join("travail.rs")), "à ne pas perdre");
        assert!(
            !occupe.join("Cargo.toml").exists(),
            "un fichier du squelette a été écrit dans le répertoire occupé"
        );
    }

    #[test]
    fn un_rendu_qui_echoue_ne_laisse_pas_de_projet_partiel() {
        let parent = parent();
        let templates = parent.path().join("templates");
        fs::create_dir(&templates).expect("répertoire créable");
        fs::write(
            templates.join("Cargo.toml.jinja"),
            "name = \"{@ nom_projet @}\"",
        )
        .expect("template écrite");
        fs::write(templates.join("src.rs.jinja"), "{@ variable_absente @}")
            .expect("template écrite");
        let mut options = options("mon-api");
        options.template_dir = Some(templates);

        let erreur = creer(&options, parent.path())
            .expect_err("une variable absente doit arrêter la génération");

        assert!(
            erreur.to_string().contains("src.rs"),
            "le message ne nomme pas la template fautive : {erreur}"
        );
        assert!(
            !parent.path().join("mon-api").exists(),
            "le projet à moitié écrit n'a pas été retiré"
        );
    }

    #[test]
    fn le_projet_cree_est_un_depot_git() {
        let parent = parent();

        let projet = creer(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert!(projet.depot_git, "`git init` non signalé dans le rapport");
        assert!(
            projet.racine.join(".git").exists(),
            "le projet créé n'est pas un dépôt"
        );
    }
}
