//! `rbs migrate` : pilotage des migrations d'un projet généré.
//!
//! `up`, `down` et `status` enveloppent le binaire de la crate `migration` du projet :
//! le moteur de SeaORM n'est pas réimplémenté, seulement rendu lisible. `new` n'a besoin
//! de personne — ni de cargo, ni d'une base démarrée.

pub mod fresh;
pub mod render;
pub mod state;

use std::io;
use std::path::Path;

use crate::generate::migration::current_timestamp;
use crate::{cargo, dotenv, metadata};

/// La variable qui porte l'URL de la base, telle que le projet la nomme.
///
/// C'est celle de la configuration du noyau — `RBS_DATABASE__URL` alimente
/// `database.url` — et non un `DATABASE_URL` que rbs serait seul à connaître.
pub(crate) const URL: &str = "RBS_DATABASE__URL";

/// Ce que `rbs migrate` peut faire.
#[derive(Debug)]
pub(crate) enum Action {
    /// Applique les migrations en attente.
    Up,
    /// Annule la dernière migration appliquée.
    Down,
    /// Inventorie les migrations et leur état.
    Status,
    /// Crée un fichier de migration vide.
    Fresh(String),
}

/// Ce qu'une action a produit, à afficher.
#[derive(Debug)]
pub(crate) enum Output {
    /// Les migrations en attente ont été appliquées.
    Appliquees,
    /// La dernière migration appliquée a été annulée.
    Annulee,
    /// L'inventaire, déjà mis en forme.
    Inventaire(String),
    /// La migration créée.
    Creee(fresh::Fresh),
}

/// Ce qui peut empêcher de piloter les migrations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,

    /// Le `.env` du projet est absent ou illisible.
    #[error("{0}")]
    Env(#[from] dotenv::Error),

    /// Le `.env` ne dit pas quelle base viser.
    #[error("{URL} est absente du .env : rbs ne sait pas quelle base migrer")]
    SansUrl,

    /// `cargo` n'a pas pu être lancé.
    #[error("cargo n'a pas pu être lancé : {0}")]
    Cargo(#[source] io::Error),

    /// Le binaire de migration a échoué.
    #[error("la crate migration a échoué (code {code})")]
    Migration {
        /// Code de sortie du sous-processus.
        code: i32,
    },

    /// La sortie du binaire de migration n'a pas pu être analysée.
    #[error("{0}")]
    State(#[from] state::Error),

    /// La migration n'a pas pu être créée.
    #[error("{0}")]
    Fresh(#[from] fresh::Error),

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

/// Exécute `action` dans le projet qui contient `directory`.
pub(crate) fn run(action: Action, directory: &Path) -> Result<Output, Error> {
    let root = metadata::project_root(directory)?;

    if let Action::Fresh(name) = action {
        return Ok(Output::Creee(fresh::run(
            &root,
            &name,
            &current_timestamp(),
        )?));
    }

    let variables = project_variables(&root)?;

    match action {
        Action::Up => {
            launch(&root, "up", &variables, false)?;
            Ok(Output::Appliquees)
        }
        Action::Down => {
            launch(&root, "down", &variables, false)?;
            Ok(Output::Annulee)
        }
        Action::Status => {
            let output = launch(&root, "status", &variables, true)?;
            Ok(Output::Inventaire(render::status(&state::parse(&output)?)))
        }
        Action::Fresh(_) => unreachable!("traitée avant la lecture du .env"),
    }
}

/// Lit le `.env` du projet et en tire ce qu'il faut transmettre au sous-processus.
pub(crate) fn project_variables(root: &Path) -> Result<Vec<(String, String)>, Error> {
    let paires = dotenv::read(&root.join(".env"))?;
    prepare(paires, |key| std::env::var_os(key).is_some())
}

/// Retient du `.env` ce que le sous-processus n'a pas déjà, et exige de savoir quelle
/// base viser.
///
/// L'environnement de l'appelant l'emporte : `RBS_DATABASE__URL=… rbs migrate up` doit
/// pouvoir viser une autre base sans toucher au fichier du projet.
fn prepare(
    paires: Vec<(String, String)>,
    definie: impl Fn(&str) -> bool,
) -> Result<Vec<(String, String)>, Error> {
    if !definie(URL) && dotenv::value(&paires, URL).is_none() {
        return Err(Error::SansUrl);
    }

    Ok(variables(paires, definie))
}

fn variables(
    paires: Vec<(String, String)>,
    definie: impl Fn(&str) -> bool,
) -> Vec<(String, String)> {
    paires
        .into_iter()
        .filter(|(key, _)| !definie(key))
        .collect()
}

/// Lance le binaire de la crate `migration` du projet.
pub(crate) fn launch(
    root: &Path,
    command: &str,
    variables: &[(String, String)],
    capturer: bool,
) -> Result<String, Error> {
    cargo::run(
        root,
        &["run", "-p", "migration", "--", command],
        variables,
        capturer,
    )
    .map_err(|error| match error {
        cargo::Error::Lancement(source) => Error::Cargo(source),
        cargo::Error::Statut(code) => Error::Migration { code },
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::fixtures::project;

    #[test]
    fn outside_an_rbs_project_nothing_is_launched() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(Action::Status, ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(error, Error::PasUnProjet));
    }

    #[test]
    fn the_expected_variable_is_the_one_a_fresh_project_writes_in_its_env() {
        let (_parent, root) = project();

        let paires = dotenv::read(&root.join(".env")).expect("le .env est lisible");

        assert!(
            dotenv::value(&paires, URL).is_some(),
            "migrate cherche {URL}, absente du .env généré"
        );
    }

    #[test]
    fn a_missing_env_is_reported_with_its_path() {
        let (_parent, root) = project();
        std::fs::remove_file(root.join(".env")).expect("le .env existe");

        let error = run(Action::Status, &root).expect_err("le .env manque");

        assert!(error.to_string().contains(".env"));
    }

    #[test]
    fn with_no_url_anywhere_the_targeted_database_is_unknown() {
        let paires = vec![("RUST_LOG".to_string(), "info".to_string())];

        let error = prepare(paires, |_| false).expect_err("aucune URL n'est connue");

        assert!(error.to_string().contains(URL));
    }

    #[test]
    fn a_url_inherited_from_the_environment_is_enough() {
        let paires = vec![("RUST_LOG".to_string(), "info".to_string())];

        prepare(paires, |key| key == URL).expect("l'appelant fournit l'URL");
    }

    #[test]
    fn a_migration_created_from_a_subdirectory_targets_the_project_root() {
        let (_parent, root) = project();

        let output = run(
            Action::Fresh("ajout_index".to_string()),
            &root.join("migration/src"),
        )
        .expect("la migration se crée");

        let Output::Creee(fresh) = output else {
            panic!("une création rend la migration créée");
        };
        assert!(root.join(&fresh.file).is_file());
    }

    #[test]
    fn an_already_defined_variable_wins_over_the_file_one() {
        let paires = vec![
            (URL.to_string(), "postgres://du-fichier".to_string()),
            ("RUST_LOG".to_string(), "info".to_string()),
        ];

        let transmises = variables(paires, |key| key == URL);

        assert_eq!(
            transmises,
            vec![("RUST_LOG".to_string(), "info".to_string())],
            "l'environnement de l'appelant l'emporte sur le .env"
        );
    }
}
