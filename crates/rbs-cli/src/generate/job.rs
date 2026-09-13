//! `rbs generate job` : un job de la file, et son échéance sous `--every`.
//!
//! La séquence est celle de `generate crud` : le nom, les features requises, l'expression
//! et l'état des fichiers visés sont jugés avant le rendu, et rien n'est écrit tant que le
//! plan n'est pas entier. Un job n'est pas une feature : le manifeste n'en garde pas trace.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::anchors;
use crate::cron;
use crate::git;
use crate::metadata;
use crate::plan;
use crate::template::Renderer;

use super::fields::to_pascal_case;
use super::{format, name};

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/job/job.rs.jinja"
));

/// Modules que le fragment `jobs` pose dans `src/modules/jobs/` : un job de ce nom
/// écraserait leur fichier, ou doublerait leur déclaration.
const MODULES_DE_LA_FILE: [&str; 6] = ["config", "demo", "model", "queue", "worker", "tests"];

/// Crates que nomment `src/modules/jobs/mod.rs` et la template d'un job, plus `core` et
/// `alloc` : déclaré dans ce fichier, un module de ce nom y masque la crate. Mesuré sur un
/// projet engendré, `std`, `serde`, `serde_json`, `anyhow` et `async_trait` y cassent la
/// compilation ; les trois autres ne la cassent pas encore, mais le premier chemin qui les
/// nommerait dans ce fichier le ferait.
const CRATES: [&str; 8] = [
    "alloc",
    "anyhow",
    "async_trait",
    "core",
    "serde",
    "serde_json",
    "std",
    "tracing",
];

/// Ce qu'il faut savoir pour engendrer un job.
pub(crate) struct Options {
    /// Nom du job, en snake_case : celui de son module et de son `KIND`.
    pub name: String,
    /// Expression cron de son échéance, telle que `--every` la donne.
    pub every: Option<String>,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Engendre même si le projet porte des modifications non commitées.
    pub force: bool,
}

/// Ce que la génération fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Chemin du fichier du job, relatif à la racine du projet.
    pub fichier: String,
    /// Ce que rustfmt n'a pas pu faire sur le rendu, s'il y a lieu.
    pub avertissement: Option<format::Avertissement>,
    /// L'expression de l'échéance, telle qu'elle a été saisie.
    pub echeance: Option<String>,
}

/// Ce qui peut empêcher d'engendrer un job.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Un fichier du projet n'a pu être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le projet porte des modifications non commitées, qu'une génération rendrait
    /// indiscernables des siennes.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    /// Le nom ne fait pas un module Rust.
    #[error("{0}")]
    Nom(name::NameError),

    /// Le nom est celui d'un module que la file porte déjà.
    #[error(
        "« {nom} » est un module de la file : src/modules/jobs/ le porte déjà — choisissez un \
         autre nom"
    )]
    ModuleDeLaFile {
        /// Nom demandé.
        nom: String,
    },

    /// Le nom est celui du dossier qui contiendrait le job.
    #[error(
        "« jobs » nommerait le module comme le dossier qui le contient : `pub mod jobs;` \
         dans src/modules/jobs/mod.rs déclenche `clippy::module_inception` — choisissez un \
         autre nom"
    )]
    NomDuDossier,

    /// Le nom est celui d'une crate : déclaré dans `src/modules/jobs/mod.rs`, le module la
    /// masquerait.
    #[error(
        "« {nom} » est aussi le nom d'une crate : `pub mod {nom};` la masquerait dans \
         src/modules/jobs/mod.rs, où un chemin `{nom}::…` ne la trouverait plus — choisissez \
         un autre nom"
    )]
    NomDeCrate {
        /// Nom demandé.
        nom: String,
    },

    /// La feature est installée, mais hors de `src/modules/` : le projet l'a reçue avant la
    /// 1.3.0, et `rbs upgrade` ne déplace aucun module.
    #[error(
        "`rbs generate job` écrit dans src/modules/{feature}/mod.rs, qui manque : ce projet a \
         reçu `{feature}` avant la 1.3.0, en `src/{feature}/`. Déplacez ce répertoire sous \
         `src/modules/` et corrigez ses `use`, puis relancez la commande"
    )]
    HorsModules {
        /// La feature restée à la racine de `src/`.
        feature: &'static str,
    },

    /// Le projet n'a pas la file où le job s'inscrit.
    #[error(
        "`rbs generate job` exige la feature `jobs`, absente de ce projet : lancez \
         `rbs add jobs`, puis relancez la commande"
    )]
    SansJobs,

    /// `--every` sur un projet sans le calendrier où l'échéance s'inscrit.
    #[error(
        "`--every` exige la feature `scheduler`, absente de ce projet : lancez \
         `rbs add scheduler`, puis relancez la commande"
    )]
    SansScheduler,

    /// L'expression de `--every` ne passerait pas le démarrage du projet.
    #[error("{0}")]
    Cron(cron::Erreur),

    /// Le module du job est déjà déclaré, hors de l'ancre où la commande le déclare.
    #[error(
        "src/modules/jobs/mod.rs porte déjà `{ligne}` hors de `// <rbs:job_modules>` : le \
         plan le déclarerait une seconde fois — choisissez un autre nom"
    )]
    ModuleDejaDeclare {
        /// La déclaration trouvée, telle qu'elle est écrite.
        ligne: String,
    },

    /// Le fichier du job existe, et ce n'est pas un job de ce nom.
    #[error(
        "{path} existe et ne définit pas `impl Job for {type_}` : rbs ne réécrit pas un \
         fichier qu'il ne reconnaît pas — choisissez un autre nom, ou déplacez ce fichier"
    )]
    FichierEtranger {
        /// Chemin du fichier, relatif à la racine.
        path: String,
        /// Type que le job aurait porté.
        type_: String,
    },

    /// Un autre job porte déjà ce `KIND`.
    #[error(
        "`KIND = \"{kind}\"` est déjà pris par {path} : deux jobs de même KIND se disputent \
         leur entrée du registre — choisissez un autre nom"
    )]
    KindPris {
        /// Le `KIND` demandé.
        kind: String,
        /// Fichier qui le porte, relatif à la racine.
        path: String,
    },

    /// Le job est déjà planifié, sous une autre expression.
    #[error(
        "src/modules/scheduler/mod.rs planifie déjà `{nom}` sous « {existante} » : le \
         calendrier se clé par KIND, et l'échéance « {demandee} » écraserait la première — \
         modifiez celle-là à la main"
    )]
    EcheanceExistante {
        /// Nom du job.
        nom: String,
        /// L'expression que le calendrier porte.
        existante: String,
        /// L'expression que `--every` demande.
        demandee: String,
    },

    /// La template du job ne s'est pas rendue.
    #[error("{file} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        file: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// Le plan de la génération n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

/// Calcule ce que la génération du job de `options` ferait au projet, sans rien écrire.
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    let metadata::Cible { root, metadonnees } = metadata::cible::<Error>(&options.directory)?;

    if !options.force {
        git::garde(&root)?;
    }

    let nom = options.name.as_str();
    name::validate_identifier(nom).map_err(Error::Nom)?;

    if MODULES_DE_LA_FILE.contains(&nom) {
        return Err(Error::ModuleDeLaFile {
            nom: nom.to_string(),
        });
    }

    if nom == "jobs" {
        return Err(Error::NomDuDossier);
    }

    if CRATES.contains(&nom) {
        return Err(Error::NomDeCrate {
            nom: nom.to_string(),
        });
    }

    let installee = |feature: &str| metadonnees.features.iter().any(|f| f == feature);
    if !installee("jobs") {
        return Err(Error::SansJobs);
    }
    if options.every.is_some() && !installee("scheduler") {
        return Err(Error::SansScheduler);
    }

    // Le manifeste ne suffit pas : un projet d'avant 1.3.0 déclare `jobs` et le porte en
    // `src/jobs/`. Planifié, le job naîtrait dans un `src/modules/jobs/` que rien ne compile
    // — et qui ferait basculer le contrôle `disposition` de `doctor` — et chacune de ses
    // insertions sauterait.
    if !root.join(&*anchors::JOBS.file).exists() {
        return Err(Error::HorsModules { feature: "jobs" });
    }
    if options.every.is_some() && !root.join(&*anchors::SCHEDULES.file).exists() {
        return Err(Error::HorsModules {
            feature: "scheduler",
        });
    }

    // Jugée ici plutôt qu'au démarrage du projet, qu'elle arrêterait : le CLI peut encore la
    // refuser sans avoir rien écrit.
    if let Some(expression) = &options.every {
        cron::valider(expression).map_err(Error::Cron)?;
    }

    let type_ = to_pascal_case(nom);
    let fichier = format!("src/modules/jobs/{nom}.rs");

    let modules = lire(&root, &anchors::JOB_MODULES.file)?.unwrap_or_default();
    if let Some(ligne) = declaration_hors_ancre(&modules, nom) {
        return Err(Error::ModuleDejaDeclare { ligne });
    }

    let existant = lire(&root, &fichier)?;
    if let Some(source) = &existant
        && !definit_le_job(source, &type_)
    {
        return Err(Error::FichierEtranger {
            path: fichier,
            type_,
        });
    }

    if let Some(path) = kind_pris(&root, nom, &fichier)? {
        return Err(Error::KindPris {
            kind: nom.to_string(),
            path,
        });
    }

    if let Some(demandee) = &options.every {
        let calendrier = lire(&root, &anchors::SCHEDULES.file)?.unwrap_or_default();
        if let Some(existante) = echeance_de(&calendrier, nom, &type_)
            && format!("\"{existante}\"") != format!("{demandee:?}")
        {
            return Err(Error::EcheanceExistante {
                nom: nom.to_string(),
                existante,
                demandee: demandee.clone(),
            });
        }
    }

    let mut avertissement = None;
    let contenu = match existant {
        // Reconnu, le fichier appartient au développeur : le plan le reprend tel quel, pour
        // l'annoncer inchangé plutôt que de le rendre à sa template — `--force` compris.
        Some(source) => source,
        None => {
            let mut rendu = render(nom).map_err(|source| Error::Rendu {
                file: fichier.clone(),
                source,
            })?;
            avertissement = format::format_batch(std::iter::once(&mut rendu));
            rendu
        }
    };

    let inscription = instructions(
        format!("registre = registre.register::<{nom}::{type_}>();"),
        &mut avertissement,
    );

    let mut builder = plan::Builder::new(root);
    builder.create(&fichier, &contenu)?;
    builder.insert_ou_sauter(anchors::JOB_MODULES, &[format!("pub mod {nom};")])?;
    builder.insert_ou_sauter(anchors::JOBS, &inscription)?;

    if let Some(expression) = &options.every {
        // `{expression:?}` : l'expression saisie devient un littéral Rust sûr, guillemets et
        // barres obliques inverses échappés.
        let chemin = format!("crate::modules::jobs::{nom}::{type_}");
        let echeance = instructions(
            format!(
                "calendrier.push(Schedule::every::<{chemin}>({expression:?}, || {chemin} {{}}));"
            ),
            &mut avertissement,
        );
        builder.insert_ou_sauter(anchors::SCHEDULES, &echeance)?;
    }

    Ok(Planned {
        plan: builder.finir(),
        fichier,
        avertissement,
        echeance: options.every.clone(),
    })
}

/// Rend le fichier du job `nom`, avant rustfmt.
fn render(nom: &str) -> Result<String, minijinja::Error> {
    let type_ = to_pascal_case(nom);

    Renderer::new().render(
        TEMPLATE,
        BTreeMap::from([("nom", nom), ("type", type_.as_str())]),
    )
}

/// Les lignes que rustfmt écrirait pour `source`, ou `source` telle quelle en disant
/// pourquoi — l'avertissement reste unique pour toute la commande, comme `format_batch`.
fn instructions(source: String, avertissement: &mut Option<format::Avertissement>) -> Vec<String> {
    match format::instructions(&source) {
        Ok(lignes) => lignes,
        Err(raison) => {
            avertissement.get_or_insert(raison);
            vec![source]
        }
    }
}

/// Le contenu de `path`, ou `None` s'il n'existe pas.
fn lire(root: &Path, path: &str) -> Result<Option<String>, Error> {
    match fs::read_to_string(root.join(path)) {
        Ok(contenu) => Ok(Some(contenu)),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(crate::errors::Acces::new(Path::new(path), source).into()),
    }
}

/// La ligne qui déclare déjà le module `nom` hors de `<rbs:job_modules>`.
///
/// Dans l'ancre, la déclaration vient d'une génération précédente et l'insertion la
/// reconnaîtra ; ailleurs, celle que le plan ajouterait ferait un module déclaré deux fois.
fn declaration_hors_ancre(modules: &str, nom: &str) -> Option<String> {
    let (ouvrante, fermante) = (
        anchors::JOB_MODULES.opening(),
        anchors::JOB_MODULES.closing(),
    );
    let module = format!("{nom};");
    let mut dans_l_ancre = false;

    for ligne in modules.lines().map(str::trim) {
        if ligne == ouvrante || ligne == fermante {
            dans_l_ancre = ligne == ouvrante;
            continue;
        }

        let mots: Vec<&str> = ligne.split_whitespace().collect();
        if !dans_l_ancre
            && mots
                .windows(2)
                .any(|paire| paire == ["mod", module.as_str()])
        {
            return Some(ligne.to_string());
        }
    }

    None
}

/// `source` définit-elle `impl Job for <type_>` ?
///
/// C'est à cette ligne, et à rien d'autre, que la commande reconnaît un fichier qu'elle a
/// pu écrire : tout le reste a pu être réécrit par le développeur.
fn definit_le_job(source: &str, type_: &str) -> bool {
    source.lines().any(|ligne| {
        ligne
            .trim()
            .strip_prefix("impl Job for ")
            .is_some_and(|reste| {
                reste
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    == Some(type_)
            })
    })
}

/// Le premier fichier de `src/`, par ordre de chemin, qui porte déjà `KIND = "<nom>"`.
///
/// Le fichier du job lui-même est écarté : sans quoi la relance de la même commande se
/// refuserait.
fn kind_pris(root: &Path, nom: &str, fichier: &str) -> Result<Option<String>, Error> {
    let cherche = format!("const KIND: &'static str = \"{nom}\";");
    let relatif = |path: &Path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let acces = |path: &Path, source| crate::errors::Acces::new(Path::new(&relatif(path)), source);

    let mut a_parcourir = vec![root.join("src")];
    let mut porteurs = Vec::new();

    while let Some(repertoire) = a_parcourir.pop() {
        for entree in fs::read_dir(&repertoire).map_err(|source| acces(&repertoire, source))? {
            let path = entree.map_err(|source| acces(&repertoire, source))?.path();

            if path.is_dir() {
                a_parcourir.push(path);
                continue;
            }

            let chemin = relatif(&path);
            if path.extension().is_none_or(|extension| extension != "rs") || chemin == fichier {
                continue;
            }

            let source = fs::read_to_string(&path).map_err(|source| acces(&path, source))?;
            if source.contains(&cherche) {
                porteurs.push(chemin);
            }
        }
    }

    porteurs.sort();

    Ok(porteurs.into_iter().next())
}

/// L'expression sous laquelle `schedules()` planifie déjà ce job, dans la forme que cette
/// commande écrit : `Schedule::every::<crate::modules::jobs::<nom>::<Type>>("<expression>"`.
///
/// Les blancs entre les jetons sont ignorés : rustfmt coupe l'appel autrement selon la
/// longueur du nom, jusqu'à ouvrir la liste des génériques et y laisser une virgule.
fn echeance_de(calendrier: &str, nom: &str, type_: &str) -> Option<String> {
    let chemin = format!("crate::modules::jobs::{nom}::{type_}");
    let consomme = |texte: &'_ str, jeton: &str| -> Option<usize> {
        let blancs = texte.len() - texte.trim_start().len();
        texte[blancs..]
            .starts_with(jeton)
            .then_some(blancs + jeton.len())
    };

    calendrier
        .match_indices("Schedule::every::<")
        .find_map(|(rang, appel)| {
            let mut reste = &calendrier[rang + appel.len()..];
            reste = &reste[consomme(reste, &chemin)?..];
            if let Some(virgule) = consomme(reste, ",") {
                reste = &reste[virgule..];
            }
            for jeton in [">", "(", "\""] {
                reste = &reste[consomme(reste, jeton)?..];
            }

            reste.find('"').map(|fin| reste[..fin].to_string())
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::fixtures::Project;
    use crate::plan::{CauseSautee, Sautee, Status};

    const MODULES: &str = "src/modules/jobs/mod.rs";
    const CALENDRIER: &str = "src/modules/scheduler/mod.rs";

    /// Le projet que ces tests attendent : la file et son calendrier installés.
    fn project() -> (TempDir, PathBuf) {
        Project::new().features(&["jobs", "scheduler"]).create()
    }

    fn options(root: &Path, name: &str, every: Option<&str>) -> Options {
        Options {
            name: name.to_string(),
            every: every.map(str::to_string),
            directory: root.to_path_buf(),
            force: false,
        }
    }

    /// Planifie puis applique, comme la commande le fait.
    fn run(options: &Options) -> Result<Planned, Error> {
        let planned = plan_for(options)?;
        crate::plan::application::apply(&planned.plan, options.force)?;

        Ok(planned)
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} illisible : {error}", path.display()))
    }

    /// Fait du projet un dépôt dont tout est commité.
    fn commit(root: &Path) {
        for arguments in [
            vec!["config", "user.email", "rbs@example.test"],
            vec!["config", "user.name", "rbs"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "projet neuf"],
        ] {
            let output = std::process::Command::new("git")
                .args(&arguments)
                .current_dir(root)
                .output()
                .expect("git doit être lançable");

            assert!(
                output.status.success(),
                "git {arguments:?} a échoué :\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Le fichier d'un plan, par son chemin.
    fn file<'a>(planned: &'a Planned, path: &str) -> &'a crate::plan::File {
        planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("`{path}` absent du plan : {:?}", planned.plan.files()))
    }

    #[test]
    fn a_pascal_case_name_is_refused_with_its_snake_case_form() {
        let (_parent, root) = project();

        let error = plan_for(&options(&root, "PurgeSessions", None))
            .expect_err("un nom en PascalCase doit être refusé");

        assert!(matches!(error, Error::Nom(_)), "{error}");
        assert!(error.to_string().contains("purge_sessions"), "{error}");
    }

    #[test]
    fn a_rust_keyword_is_refused() {
        let (_parent, root) = project();

        let error = plan_for(&options(&root, "match", None)).expect_err("`match` est un mot-clé");

        assert!(error.to_string().contains("mot-clé Rust"), "{error}");
    }

    /// `src/modules/jobs/` porte déjà ces modules : un job de ce nom écraserait le fichier
    /// ou doublerait sa déclaration.
    #[test]
    fn a_module_of_the_queue_is_refused_by_naming_it() {
        let (_parent, root) = project();

        for name in ["config", "demo", "model", "queue", "worker", "tests"] {
            let error = plan_for(&options(&root, name, None))
                .expect_err("un module de la file doit être refusé");

            assert!(matches!(error, Error::ModuleDeLaFile { .. }), "{error}");
            assert!(error.to_string().contains(name), "{error}");
        }
    }

    /// `pub mod jobs;` dans `src/modules/jobs/mod.rs` nommerait le module comme son propre
    /// dossier : mesuré sur une crate jetable, `clippy::module_inception` le refuse, et un
    /// projet engendré tomberait sous son propre `clippy -D warnings`.
    #[test]
    fn a_job_named_after_its_containing_directory_is_refused() {
        let (_parent, root) = project();

        let error = plan_for(&options(&root, "jobs", None)).expect_err("`jobs` doit être refusé");

        assert!(matches!(error, Error::NomDuDossier), "{error}");
        assert!(error.to_string().contains("module_inception"), "{error}");
        assert!(!root.join("src/modules/jobs/jobs.rs").exists());
    }

    #[test]
    fn a_project_without_jobs_is_refused_and_names_the_command_that_installs_it() {
        let (_parent, root) = crate::fixtures::project();

        let error = run(&options(&root, "purge", None)).expect_err("la file manque");

        assert!(matches!(error, Error::SansJobs), "{error}");
        assert!(
            error.to_string().contains("rbs add jobs"),
            "le message doit nommer la commande qui installe la feature : {error}"
        );
        assert!(!root.join("src/modules/jobs/purge.rs").exists());
    }

    #[test]
    fn every_without_scheduler_is_refused_and_names_the_command_that_installs_it() {
        let (_parent, root) = Project::new().features(&["jobs"]).create();

        let error =
            run(&options(&root, "purge", Some("0 4 * * *"))).expect_err("le calendrier manque");

        assert!(matches!(error, Error::SansScheduler), "{error}");
        assert!(
            error.to_string().contains("rbs add scheduler"),
            "le message doit nommer la commande qui installe la feature : {error}"
        );
        assert!(!root.join("src/modules/jobs/purge.rs").exists());

        run(&options(&root, "purge", None)).expect("sans --every, la file suffit");
    }

    /// Refusée ici, l'expression n'arrête pas le démarrage du projet : c'est tout l'intérêt
    /// de la juger avant le plan.
    #[test]
    fn an_unreadable_expression_is_refused_before_anything_is_written() {
        let (_parent, root) = project();
        let modules = read(&root.join(MODULES));
        let calendrier = read(&root.join(CALENDRIER));

        let error =
            run(&options(&root, "purge", Some("0 99 * * *"))).expect_err("99 n'est pas une heure");

        assert!(matches!(error, Error::Cron(_)), "{error}");
        assert!(error.to_string().contains("0 99 * * *"), "{error}");
        assert!(!root.join("src/modules/jobs/purge.rs").exists());
        assert_eq!(read(&root.join(MODULES)), modules);
        assert_eq!(read(&root.join(CALENDRIER)), calendrier);
    }

    #[test]
    fn the_plan_creates_the_job_and_fills_its_three_anchors() {
        let (_parent, root) = project();
        let manifeste = read(&root.join("Cargo.toml"));

        let planned = plan_for(&options(&root, "purge", Some("0 4 * * *")))
            .expect("la planification aboutit");

        assert_eq!(planned.fichier, "src/modules/jobs/purge.rs");
        assert_eq!(planned.echeance.as_deref(), Some("0 4 * * *"));
        assert!(
            planned.plan.sautees().is_empty(),
            "{:?}",
            planned.plan.sautees()
        );

        let chemins: Vec<(&str, bool, Status)> = planned
            .plan
            .files()
            .iter()
            .map(|file| (file.path.as_str(), file.before.is_some(), file.statut))
            .collect();
        assert_eq!(
            chemins,
            [
                ("src/modules/jobs/purge.rs", false, Status::AFaire),
                (MODULES, true, Status::AFaire),
                (CALENDRIER, true, Status::AFaire),
            ]
        );

        let job = &file(&planned, "src/modules/jobs/purge.rs").after;
        assert!(job.contains("pub struct Purge {}"), "{job}");
        assert!(job.contains("impl Job for Purge {"), "{job}");
        assert!(
            job.contains("const KIND: &'static str = \"purge\";"),
            "{job}"
        );

        let modules = &file(&planned, MODULES).after;
        assert!(
            modules.contains(
                "pub mod worker;\n// <rbs:job_modules>\npub mod purge;\n// </rbs:job_modules>\n"
            ),
            "{modules}"
        );
        assert!(
            modules.contains(
                "    registre = registre.register::<purge::Purge>();\n    // </rbs:jobs>\n"
            ),
            "{modules}"
        );

        let calendrier = &file(&planned, CALENDRIER).after;
        assert!(
            calendrier.contains(
                "    // <rbs:schedules>\n    \
                 calendrier.push(Schedule::every::<crate::modules::jobs::purge::Purge>(\n        \
                 \"0 4 * * *\",\n        \
                 || crate::modules::jobs::purge::Purge {},\n    \
                 ));\n    \
                 // </rbs:schedules>\n"
            ),
            "{calendrier}"
        );

        crate::plan::application::apply(&planned.plan, false).expect("l'écriture aboutit");
        assert_eq!(
            read(&root.join("Cargo.toml")),
            manifeste,
            "un job n'est pas une feature : le manifeste ne doit pas bouger"
        );
    }

    /// Sans `--every`, le calendrier n'est pas visité : le plan ne le nomme pas.
    #[test]
    fn without_every_the_calendar_is_left_out_of_the_plan() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "purge", None)).expect("la planification aboutit");

        assert_eq!(planned.echeance, None);
        assert!(
            planned
                .plan
                .files()
                .iter()
                .all(|file| file.path != CALENDRIER),
            "{:?}",
            planned.plan.files()
        );
    }

    /// La même commande relancée ne change rien — le `KIND` du job, présent dans son
    /// propre fichier, ne compte pas comme déjà pris.
    #[test]
    fn a_second_identical_run_finds_everything_unchanged() {
        let (_parent, root) = project();
        run(&options(&root, "purge", Some("0 4 * * *"))).expect("la première génération aboutit");

        let second =
            plan_for(&options(&root, "purge", Some("0 4 * * *"))).expect("la relance se planifie");

        assert_eq!(second.plan.files().len(), 3, "{:?}", second.plan.files());
        for file in second.plan.files() {
            assert_eq!(file.statut, Status::DejaFait, "{} réécrit", file.path);
        }
        assert!(
            second.plan.sautees().is_empty(),
            "{:?}",
            second.plan.sautees()
        );
    }

    /// Le fichier du job appartient au développeur dès qu'il est écrit : ni la relance ni
    /// `--force` ne le rendent à sa template.
    #[test]
    fn a_job_edited_by_hand_is_announced_unchanged_and_left_intact() {
        let (_parent, root) = project();
        run(&options(&root, "purge", None)).expect("la première génération aboutit");
        let chemin = root.join("src/modules/jobs/purge.rs");
        let edite = read(&chemin).replace("Ok(())", "purger(_state).await");
        fs::write(&chemin, &edite).expect("le job se réécrit");

        let planned = run(&Options {
            force: true,
            ..options(&root, "purge", None)
        })
        .expect("un job reconnu se replanifie");

        let job = file(&planned, "src/modules/jobs/purge.rs");
        assert_eq!(job.statut, Status::DejaFait);
        assert_eq!(read(&chemin), edite);
    }

    /// Un projet engendré avant l'ancre des modules : la déclaration est à reporter, et le
    /// reste s'écrit — le job, et son inscription au registre.
    #[test]
    fn a_project_without_the_modules_anchor_skips_the_declaration_and_writes_the_rest() {
        let (_parent, root) = project();
        let chemin = root.join(MODULES);
        let ampute = read(&chemin)
            .replace("// <rbs:job_modules>\n", "")
            .replace("// </rbs:job_modules>\n", "");
        fs::write(&chemin, ampute).expect("le module se réécrit");

        let planned = run(&options(&root, "purge", None)).expect("l'ancre manquante se saute");

        assert_eq!(
            planned.plan.sautees(),
            [Sautee {
                anchor: crate::anchors::JOB_MODULES,
                lines: vec!["pub mod purge;".to_string()],
                cause: CauseSautee::AncreAbsente,
            }]
        );
        assert!(root.join("src/modules/jobs/purge.rs").exists());
        assert!(read(&chemin).contains("registre = registre.register::<purge::Purge>();"));
    }

    #[test]
    fn a_module_declared_outside_the_anchor_is_refused() {
        let (_parent, root) = project();
        let chemin = root.join(MODULES);
        let source = read(&chemin).replace("pub mod worker;\n", "pub mod log;\npub mod worker;\n");
        fs::write(&chemin, source).expect("le module se réécrit");

        let error = plan_for(&options(&root, "log", None)).expect_err("`log` est déjà déclaré");

        assert!(matches!(error, Error::ModuleDejaDeclare { .. }), "{error}");
        assert!(error.to_string().contains("pub mod log;"), "{error}");
    }

    /// Deux jobs de même `KIND` se disputent la même entrée du registre : le second
    /// inscrit remplace le premier, sans un mot.
    #[test]
    fn a_kind_already_taken_is_refused_and_names_its_file() {
        let (_parent, root) = project();

        let error = plan_for(&options(&root, "log", None)).expect_err("`log` est le KIND de demo");

        assert!(matches!(error, Error::KindPris { .. }), "{error}");
        assert!(
            error.to_string().contains("src/modules/jobs/demo.rs"),
            "{error}"
        );
    }

    #[test]
    fn a_foreign_file_under_the_job_name_is_refused_and_left_intact() {
        let (_parent, root) = project();
        let chemin = root.join("src/modules/jobs/purge.rs");
        fs::write(&chemin, "pub fn purge() {}\n").expect("le fichier s'écrit");

        let error = run(&options(&root, "purge", None)).expect_err("le fichier n'est pas un job");

        assert!(matches!(error, Error::FichierEtranger { .. }), "{error}");
        assert!(error.to_string().contains("impl Job for Purge"), "{error}");
        assert_eq!(read(&chemin), "pub fn purge() {}\n");
    }

    /// Le calendrier se clé par `kind` : une seconde échéance du même job écraserait la
    /// première dans la table.
    #[test]
    fn a_second_schedule_with_another_expression_is_refused() {
        let (_parent, root) = project();
        run(&options(&root, "purge", Some("0 4 * * *"))).expect("la première génération aboutit");

        let error = plan_for(&options(&root, "purge", Some("*/5 * * * *")))
            .expect_err("purge est déjà planifié");

        assert!(matches!(error, Error::EcheanceExistante { .. }), "{error}");
        let message = error.to_string();
        assert!(message.contains("0 4 * * *"), "{message}");
        assert!(message.contains("*/5 * * * *"), "{message}");
    }

    /// Un nom long fait couper rustfmt dans la liste des génériques, virgule finale
    /// comprise : l'échéance doit s'y lire comme sous la forme courte.
    #[test]
    fn a_schedule_is_read_back_whichever_way_rustfmt_broke_the_call() {
        let courte = "    calendrier.push(Schedule::every::<crate::modules::jobs::purge::Purge>(\n        \"0 4 * * *\",\n        || crate::modules::jobs::purge::Purge {},\n    ));\n";
        let longue = "    calendrier.push(Schedule::every::<\n        crate::modules::jobs::purger_les_sessions::PurgerLesSessions,\n    >(\"*/5 * * * *\", || {\n        crate::modules::jobs::purger_les_sessions::PurgerLesSessions {}\n    }));\n";

        assert_eq!(
            echeance_de(courte, "purge", "Purge").as_deref(),
            Some("0 4 * * *")
        );
        assert_eq!(
            echeance_de(longue, "purger_les_sessions", "PurgerLesSessions").as_deref(),
            Some("*/5 * * * *")
        );
        assert_eq!(echeance_de(courte, "purger", "Purger"), None);
    }

    /// `<rbs:job_modules>` se garde trié, comme rustfmt le rendrait ; `<rbs:schedules>`
    /// garde l'ordre d'arrivée.
    #[test]
    fn two_jobs_keep_their_modules_sorted_and_their_schedules_in_arrival_order() {
        let (_parent, root) = project();
        run(&options(&root, "zeta", Some("0 1 * * *"))).expect("zeta s'engendre");
        run(&options(&root, "alpha", Some("0 2 * * *"))).expect("alpha s'engendre");

        let modules = read(&root.join(MODULES));
        assert!(
            modules.contains(
                "// <rbs:job_modules>\npub mod alpha;\npub mod zeta;\n// </rbs:job_modules>"
            ),
            "{modules}"
        );

        let calendrier = read(&root.join(CALENDRIER));
        let zeta = calendrier.find("jobs::zeta::Zeta>").expect("zeta planifié");
        let alpha = calendrier
            .find("jobs::alpha::Alpha>")
            .expect("alpha planifié");
        assert!(zeta < alpha, "{calendrier}");
    }

    #[test]
    fn a_dirty_project_refuses_the_job_unless_forced() {
        let (_parent, root) = project();
        commit(&root);
        let main = root.join("src/main.rs");
        fs::write(&main, format!("{}\n// en cours\n", read(&main))).expect("main.rs réécrivable");

        let error = run(&options(&root, "purge", None)).expect_err("l'arbre est sale");
        assert!(matches!(error, Error::WorkingTreeSale(_)), "{error}");
        assert!(!root.join("src/modules/jobs/purge.rs").exists());

        run(&Options {
            force: true,
            ..options(&root, "purge", None)
        })
        .expect("`--force` passe outre");
        assert!(root.join("src/modules/jobs/purge.rs").exists());
    }

    /// Un projet qui a reçu `jobs` avant la 1.3.0 le porte en `src/jobs/`, où `rbs upgrade`
    /// l'a laissé. Planifié quand même, le job naîtrait dans un `src/modules/jobs/` que rien
    /// ne compile, et ses insertions sauteraient toutes.
    #[test]
    fn a_queue_of_the_layout_before_1_3_0_is_refused_before_anything_is_written() {
        let (_parent, root) = project();
        fs::rename(root.join("src/modules/jobs"), root.join("src/jobs"))
            .expect("la file se déplace à l'ancienne place");

        let error = run(&options(&root, "purge", None)).expect_err("la file est hors de modules");

        let message = error.to_string();
        assert!(message.contains("src/modules/jobs/mod.rs"), "{message}");
        assert!(message.contains("1.3.0"), "{message}");
        assert!(message.contains("`src/jobs/`"), "{message}");
        assert!(!root.join("src/modules/jobs").exists());
    }

    #[test]
    fn every_on_a_calendar_of_the_layout_before_1_3_0_is_refused() {
        let (_parent, root) = project();
        fs::rename(
            root.join("src/modules/scheduler"),
            root.join("src/scheduler"),
        )
        .expect("le calendrier se déplace à l'ancienne place");

        let error = run(&options(&root, "purge", Some("0 4 * * *")))
            .expect_err("le calendrier est hors de modules");

        let message = error.to_string();
        assert!(
            message.contains("src/modules/scheduler/mod.rs"),
            "{message}"
        );
        assert!(message.contains("`src/scheduler/`"), "{message}");
        assert!(!root.join("src/modules/jobs/purge.rs").exists());

        run(&options(&root, "purge", None)).expect("sans --every, le calendrier n'est pas visé");
    }

    /// Déclaré dans `src/modules/jobs/mod.rs`, un module nommé comme une crate que ce fichier
    /// emploie la masque : mesuré sur `examples/event-hub`, `pub mod serde;` y casse `use
    /// serde::Serialize`, et de même pour `std`, `serde_json`, `anyhow` et `async_trait`.
    #[test]
    fn a_job_named_after_a_crate_is_refused_before_anything_is_written() {
        let (_parent, root) = project();
        let modules = read(&root.join(MODULES));

        for nom in [
            "alloc",
            "anyhow",
            "async_trait",
            "core",
            "serde",
            "serde_json",
            "std",
            "tracing",
        ] {
            let error = run(&options(&root, nom, None)).expect_err("le nom masquerait une crate");

            let message = error.to_string();
            assert!(message.contains(&format!("« {nom} »")), "{message}");
            assert!(message.contains("crate"), "{message}");
            assert!(!root.join(format!("src/modules/jobs/{nom}.rs")).exists());
        }
        assert_eq!(read(&root.join(MODULES)), modules);
    }

    /// Les racines de chemin de `source` qui peuvent nommer une crate : l'identifiant en
    /// minuscules qui ouvre un `a::b`, hors commentaires, hors `crate`, `super` et `self`, et
    /// hors les modules que la file porte elle-même.
    fn racines(source: &str) -> std::collections::BTreeSet<String> {
        let mut racines = std::collections::BTreeSet::new();

        for ligne in source
            .lines()
            .map(str::trim)
            .filter(|ligne| !ligne.starts_with("//"))
        {
            for (rang, _) in ligne.match_indices("::") {
                let avant = &ligne[..rang];
                let debut = avant
                    .char_indices()
                    .rev()
                    .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
                    .map_or(0, |(i, c)| i + c.len_utf8());
                let racine = &avant[debut..];
                // `a::b::c` n'a qu'une racine, et `x.register::<J>()` n'en a pas.
                let precedent = avant[..debut].chars().next_back();

                if racine.starts_with(|c: char| c.is_ascii_lowercase())
                    && !matches!(precedent, Some(':' | '.'))
                    && !["crate", "super", "self"].contains(&racine)
                    && !MODULES_DE_LA_FILE.contains(&racine)
                {
                    racines.insert(racine.to_string());
                }
            }
        }

        racines
    }

    /// Les noms refusés sont les crates que nomment la file et la template d'un job, plus
    /// `core` et `alloc` : une crate ajoutée à l'une d'elles sans l'être à `CRATES`
    /// rouvrirait le trou, une crate retirée laisserait un refus sans raison.
    #[test]
    fn the_refused_crate_names_are_the_path_roots_of_the_queue_templates() {
        let file = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/jobs/mod.rs.jinja"
        ));
        let mut trouvees = racines(file);
        trouvees.extend(racines(TEMPLATE));

        let attendues: std::collections::BTreeSet<String> = CRATES
            .iter()
            .filter(|nom| !["core", "alloc"].contains(nom))
            .map(|nom| nom.to_string())
            .collect();
        assert_eq!(trouvees, attendues);
    }

    /// Le fichier rendu est déjà ce que rustfmt écrirait : `format_batch` n'est qu'un
    /// filet, et un nom long ne doit rien y changer.
    #[test]
    fn the_rendered_job_is_already_what_rustfmt_would_write() {
        for nom in [
            "purge",
            "purger_les_sessions_expirees_depuis_plus_de_trente_jours",
        ] {
            let rendu = render(nom).expect("la template se rend");
            let mut formate = [rendu.clone()];

            assert_eq!(format::format_batch(formate.iter_mut()), None);
            assert_eq!(formate[0], rendu, "rustfmt réécrit le job `{nom}`");
        }
    }
}
