//! `rbs seed` : les données de démonstration d'un projet généré.
//!
//! La commande enveloppe le binaire `seed` du projet, sur le motif de `rbs migrate` : le
//! CLI ne parle jamais à la base et ne gagne aucun client SQL.
//!
//! Le refus d'insérer en production vit ici, et non dans le code généré. Un seed est fait
//! pour être modifié : un garde-fou posé dans le projet pourrait être retiré par mégarde,
//! celui-ci non.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{anchors, cargo, dotenv, metadata, migrate};

/// Racine du binaire des seeds dans le projet.
const BINAIRE: &str = "src/seeds/main.rs";

/// La variable qui nomme l'environnement, telle que la configuration du noyau la lit.
const ENV: &str = "RBS_ENV";

/// L'environnement où insérer des données de démonstration se refuse.
const PRODUCTION: &str = "production";

/// Ce qu'il faut savoir pour insérer les seeds.
pub(crate) struct Options {
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Insère malgré un environnement de production.
    pub force: bool,
}

/// Ce que la commande a fait, à afficher.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Output {
    /// Le binaire du projet a tourné.
    Insere,
    /// Aucun seed n'est déclaré : il n'y avait rien à insérer.
    Rien,
}

/// Ce qui peut empêcher d'insérer les seeds.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error(
        "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici"
    )]
    PasUnProjet,

    /// L'environnement visé est la production.
    #[error(
        "{ENV}={PRODUCTION} : les seeds sont des données de démonstration, et rbs refuse de \
         les insérer en production — relancez avec --force si c'est bien ce que vous voulez"
    )]
    Production,

    /// Le projet ne porte pas de binaire de seeds.
    #[error("ce projet n'a pas de {BINAIRE} : `rbs seed` n'a aucun binaire à lancer")]
    SansSeeds,

    /// Le binaire des seeds n'a pas pu être lu.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// Le `.env` du projet est absent ou illisible.
    #[error("{0}")]
    Env(#[from] dotenv::Error),

    /// Le `.env` ne dit pas quelle base viser.
    #[error(
        "{} est absente du .env : rbs ne sait pas quelle base peupler",
        migrate::URL
    )]
    SansUrl,

    /// `cargo` n'a pas pu être lancé.
    #[error("cargo n'a pas pu être lancé : {0}")]
    Cargo(#[source] io::Error),

    /// Le binaire des seeds a échoué.
    #[error("le binaire seed du projet a échoué (code {code})")]
    Seeds {
        /// Code de sortie du sous-processus.
        code: i32,
    },

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),
}

/// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
impl From<metadata::RootError> for Error {
    fn from(faute: metadata::RootError) -> Self {
        match faute {
            metadata::RootError::Absent => Self::PasUnProjet,
            metadata::RootError::Illisible(faute) => Self::Metadata(faute),
        }
    }
}

impl Error {
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Un projet créé avant les seeds n'a pas de binaire à lancer, et cela se répare en
    /// deux gestes plutôt que par une décision : le remède les donne.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Error::SansSeeds => Some(format!(
                "créez {BINAIRE}, puis déclarez-le dans Cargo.toml :\n\n\
                 [[bin]]\nname = \"seed\"\npath = \"{BINAIRE}\"\n\n\
                 un projet créé par `rbs new` le porte déjà."
            )),
            _ => None,
        }
    }
}

/// Insère les seeds du projet qui contient `options.directory`.
pub(crate) fn run(options: &Options) -> Result<Output, Error> {
    let root = metadata::project_root(&options.directory)?;

    execute(&root, options.force, |key| std::env::var(key).ok(), launch)
}

/// Le corps de la commande, l'environnement et le lancement rendus injectables.
///
/// Les deux sont des paramètres pour que « le binaire du projet n'a pas été lancé » soit
/// une assertion, et non une déduction sur la durée d'un test.
fn execute(
    root: &Path,
    force: bool,
    env: impl Fn(&str) -> Option<String>,
    launch: impl FnOnce(&Path) -> Result<(), Error>,
) -> Result<Output, Error> {
    // Avant toute lecture : le refus doit tenir même sur un projet dont il ne reste rien.
    if !force && production(root, &env) {
        return Err(Error::Production);
    }

    let source = read_binary(root)?;

    // Une ancre absente ne bloque pas : un binaire de seeds écrit à la main reste
    // lançable. Seule une ancre présente et vide dit qu'il n'y a rien à insérer, et rien
    // à insérer ne vaut pas la compilation d'un projet entier.
    if anchors::body(&source, anchors::SEEDS).is_some_and(|body| body.trim().is_empty()) {
        return Ok(Output::Rien);
    }

    launch(root)?;

    Ok(Output::Insere)
}

/// La source du binaire des seeds. Son absence se dit ici plutôt que dans la sortie de
/// cargo, où elle ne dirait rien de ce qu'il faut faire.
fn read_binary(root: &Path) -> Result<String, Error> {
    match fs::read_to_string(root.join(BINAIRE)) {
        Ok(source) => Ok(source),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Err(Error::SansSeeds),
        Err(source) => Err(Error::Acces {
            path: BINAIRE.to_string(),
            source,
        }),
    }
}

/// L'environnement visé est-il la production ?
///
/// L'environnement de l'appelant l'emporte sur le `.env` du projet, comme pour
/// `rbs migrate` : `RBS_ENV=development rbs seed` doit pouvoir viser un projet dont le
/// fichier dit autre chose. Un `.env` illisible ne vaut pas production — le refus se
/// prononce sur une valeur lue, jamais sur un doute.
fn production(root: &Path, env: impl Fn(&str) -> Option<String>) -> bool {
    let declare = env(ENV).or_else(|| {
        let paires = dotenv::read(&root.join(".env")).ok()?;
        dotenv::value(&paires, ENV).map(str::to_string)
    });

    declare.as_deref() == Some(PRODUCTION)
}

/// Lance le binaire `seed` du projet, le `.env` transmis.
fn launch(root: &Path) -> Result<(), Error> {
    let variables = migrate::project_variables(root).map_err(|error| match error {
        migrate::Error::Env(source) => Error::Env(source),
        _ => Error::SansUrl,
    })?;

    cargo::run(root, &["run", "--bin", "seed"], &variables, false)
        .map(|_| ())
        .map_err(|error| match error {
            cargo::Error::Lancement(source) => Error::Cargo(source),
            cargo::Error::Statut(code) => Error::Seeds { code },
        })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use tempfile::TempDir;

    use super::*;
    use crate::fixtures::project;

    /// Le même projet, un seed déclaré dans son ancre : il y a quelque chose à insérer.
    fn seeded() -> (TempDir, PathBuf) {
        let (parent, root) = project();
        let path = root.join(BINAIRE);
        let source = fs::read_to_string(&path).expect("binaire de seeds lisible");

        fs::write(
            &path,
            anchors::insert(&source, anchors::SEEDS, &["articles,".to_string()])
                .expect("l'ancre est présente"),
        )
        .expect("binaire de seeds réécrivable");

        (parent, root)
    }

    /// L'environnement du processus, remplacé par une table.
    fn env(paires: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| {
            paires
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_string())
        }
    }

    fn sans_env(_: &str) -> Option<String> {
        None
    }

    /// Lance `execute` en notant si le binaire du projet a été lancé.
    fn run_noting(
        root: &Path,
        force: bool,
        env: impl Fn(&str) -> Option<String>,
    ) -> (Result<Output, Error>, bool) {
        let lance = Cell::new(false);

        let output = execute(root, force, env, |_| {
            lance.set(true);
            Ok(())
        });

        (output, lance.get())
    }

    /// Remplace `RBS_ENV` dans le `.env` du projet.
    fn declare(root: &Path, environnement: &str) {
        let dotenv = root.join(".env");
        let source = fs::read_to_string(&dotenv).expect(".env lisible");

        fs::write(
            &dotenv,
            source.replace("RBS_ENV=development", &format!("RBS_ENV={environnement}")),
        )
        .expect(".env réécrivable");
    }

    /// Le premier critère du lot : la production refuse, et rien ne part.
    #[test]
    fn under_production_the_command_refuses_naming_force_and_launches_nothing() {
        let (_parent, root) = seeded();

        let (output, lance) = run_noting(&root, false, env(&[(ENV, PRODUCTION)]));

        let error = output.expect_err("la production doit être refusée");
        assert!(matches!(error, Error::Production), "{error}");
        assert!(
            error.to_string().contains("--force"),
            "le refus doit nommer l'échappatoire : {error}"
        );
        assert!(!lance, "le binaire du projet a été lancé malgré le refus");
    }

    #[test]
    fn force_lifts_the_production_refusal() {
        let (_parent, root) = seeded();

        let (output, lance) = run_noting(&root, true, env(&[(ENV, PRODUCTION)]));

        output.expect("`--force` passe outre");
        assert!(lance, "le binaire du projet aurait dû être lancé");
    }

    /// Le `.env` d'un projet déployé porte l'environnement : le refus doit l'y voir.
    #[test]
    fn a_dotenv_declaring_production_is_enough_to_refuse() {
        let (_parent, root) = seeded();
        declare(&root, PRODUCTION);

        let (output, lance) = run_noting(&root, false, sans_env);

        assert!(
            matches!(output, Err(Error::Production)),
            "la production déclarée dans le .env doit refuser"
        );
        assert!(!lance, "le binaire du projet a été lancé malgré le refus");
    }

    /// Un `.env` annoté à la main reste un `.env` : le commentaire ne doit pas désarmer
    /// le refus en rendant la valeur différente de `production`.
    #[test]
    fn a_commented_production_line_still_refuses() {
        let (_parent, root) = project();
        declare(&root, "production # ne jamais semer");

        assert!(production(&root, sans_env));
    }

    /// L'environnement de l'appelant l'emporte, comme pour `rbs migrate`.
    #[test]
    fn the_callers_environment_overrides_the_projects_dotenv() {
        let (_parent, root) = seeded();
        declare(&root, PRODUCTION);

        let (output, lance) = run_noting(&root, false, env(&[(ENV, "development")]));

        output.expect("l'appelant vise le développement");
        assert!(lance, "le binaire du projet aurait dû être lancé");
    }

    /// Le second critère du lot : dire comment créer le binaire, non buter sur cargo.
    #[test]
    fn a_project_without_seeds_says_how_to_create_one() {
        let (_parent, root) = project();
        let _ = fs::remove_dir_all(root.join("src/seeds"));

        let (output, lance) = run_noting(&root, false, sans_env);

        let error = output.expect_err("il n'y a pas de binaire de seeds");
        assert!(matches!(error, Error::SansSeeds), "{error}");
        assert!(error.to_string().contains(BINAIRE), "{error}");

        let remedy = error.remedy().expect("un binaire absent se crée");
        assert!(remedy.contains("[[bin]]"), "{remedy}");
        assert!(remedy.contains("name = \"seed\""), "{remedy}");
        assert!(!lance, "cargo n'a rien à faire ici");
    }

    /// Le critère du squelette : un projet vierge n'est pas une panne, et ne compile rien.
    #[test]
    fn an_empty_anchor_reports_nothing_to_insert_without_launching_cargo() {
        let (_parent, root) = project();

        let (output, lance) = run_noting(&root, false, sans_env);

        assert_eq!(
            output.expect("un projet vierge n'est pas une erreur"),
            Output::Rien
        );
        assert!(!lance, "il n'y avait rien à insérer");
    }

    #[test]
    fn a_declared_seed_launches_the_project_binary() {
        let (_parent, root) = seeded();

        let (output, lance) = run_noting(&root, false, sans_env);

        assert_eq!(output.expect("le lancement aboutit"), Output::Insere);
        assert!(lance, "le binaire du projet aurait dû être lancé");
    }

    /// Un binaire de seeds écrit à la main n'a aucune ancre à porter : rbs le lance.
    #[test]
    fn a_binary_without_the_anchor_is_launched_all_the_same() {
        let (_parent, root) = project();
        fs::write(root.join(BINAIRE), "fn main() {}\n").expect("binaire de seeds réécrivable");

        let (output, lance) = run_noting(&root, false, sans_env);

        assert_eq!(output.expect("le lancement aboutit"), Output::Insere);
        assert!(lance, "le binaire du projet aurait dû être lancé");
    }

    #[test]
    fn an_error_without_a_known_remedy_does_not_invent_one() {
        assert_eq!(Error::PasUnProjet.remedy(), None);
    }

    #[test]
    fn outside_a_project_nothing_is_launched() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(&Options {
            directory: ailleurs.path().to_path_buf(),
            force: false,
        })
        .expect_err("il n'y a pas de projet ici");

        assert!(matches!(error, Error::PasUnProjet), "{error}");
    }
}
