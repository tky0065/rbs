use std::path::{Path, PathBuf};
use std::sync::Arc;

use minijinja::{Environment, path_loader};
use rbs_core::{Error, Result};
use serde::Serialize;

/// Les gabarits de messages, lus dans un répertoire du projet.
///
/// L'environnement est en `Arc` parce qu'`AppState` se clone à chaque requête : les
/// gabarits sont chargés une fois, pas une fois par appel.
#[derive(Debug, Clone)]
pub struct Gabarits {
    racine: PathBuf,
    environnement: Arc<Environment<'static>>,
}

impl Gabarits {
    pub fn nouveaux(racine: impl AsRef<Path>) -> Self {
        let racine = racine.as_ref().to_path_buf();
        let mut environnement = Environment::new();
        // `path_loader` refuse un nom absolu ou remontant : un nom de gabarit venu d'une
        // entrée utilisateur ne peut pas sortir du répertoire.
        environnement.set_loader(path_loader(&racine));

        Self {
            racine,
            environnement: Arc::new(environnement),
        }
    }

    pub fn rendre<S: Serialize>(&self, nom: &str, contexte: S) -> Result<String> {
        let gabarit = self
            .environnement
            .get_template(nom)
            .map_err(|source| self.erreur(nom, &source))?;

        gabarit
            .render(contexte)
            .map_err(|source| self.erreur(nom, &source))
    }

    /// minijinja ne connaît que le nom du gabarit : c'est ici que le chemin s'ajoute,
    /// sans quoi « absent.html » n'oriente vers aucun répertoire.
    fn erreur(&self, nom: &str, source: &minijinja::Error) -> Error {
        Error::Internal(anyhow::anyhow!(
            "gabarit {} : {source}",
            self.racine.join(nom).display()
        ))
    }
}
