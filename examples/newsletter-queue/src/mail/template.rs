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
pub struct Templates {
    root: PathBuf,
    environnement: Arc<Environment<'static>>,
}

impl Templates {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let mut environnement = Environment::new();
        // `path_loader` refuse un nom absolu ou remontant : un nom de gabarit venu d'une
        // entrée utilisateur ne peut pas sortir du répertoire.
        environnement.set_loader(path_loader(&root));

        Self {
            root,
            environnement: Arc::new(environnement),
        }
    }

    pub fn render<S: Serialize>(&self, name: &str, context: S) -> Result<String> {
        let template = self
            .environnement
            .get_template(name)
            .map_err(|source| self.error(name, &source))?;

        template
            .render(context)
            .map_err(|source| self.error(name, &source))
    }

    /// minijinja ne connaît que le nom du gabarit : c'est ici que le chemin s'ajoute,
    /// sans quoi « absent.html » n'oriente vers aucun répertoire.
    fn error(&self, name: &str, source: &minijinja::Error) -> Error {
        Error::Internal(anyhow::anyhow!(
            "gabarit {} : {source}",
            self.root.join(name).display()
        ))
    }
}
