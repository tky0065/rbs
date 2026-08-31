//! Le projet neuf sur lequel presque tous les tests de la crate s'appuient.
//!
//! Dix-huit modules en portaient leur copie : une option ajoutée à `rbs new` demandait de
//! les visiter tous, et deux copies avaient déjà divergé de ce que `rbs new` produit.
//!
//! Le constructeur est en chaîne pour que chaque appelant ne nomme que ce qui le
//! concerne : un test qui choisit son moteur n'a pas à répéter l'URL, et un test qui
//! choisit son URL n'a pas à répéter le moteur.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::database::Database;
use crate::lang::Lang;
use crate::new;

/// Un projet à créer, et ce qui le distingue du projet par défaut.
pub(crate) struct Project {
    options: new::Options,
}

impl Project {
    /// Le projet que la plupart des tests attendent : `demo-api`, PostgreSQL, sans
    /// feature.
    pub(crate) fn new() -> Self {
        Self {
            options: new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Database::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: Lang::Fr,
            },
        }
    }

    /// Le moteur du projet. L'URL ne suit pas : les deux se choisissent séparément.
    pub(crate) fn database(mut self, database: Database) -> Self {
        self.options.database = database;
        self
    }

    /// L'URL que le `.env` du projet portera.
    pub(crate) fn url(mut self, url: &str) -> Self {
        self.options.database_url = url.to_string();
        self
    }

    /// Les features à installer à la création.
    pub(crate) fn features(mut self, features: &[&str]) -> Self {
        self.options.features = features.iter().map(|f| (*f).to_string()).collect();
        self
    }

    /// Le chemin du noyau, quand le test a besoin d'une dépendance locale.
    pub(crate) fn core_path(mut self, core_path: Option<PathBuf>) -> Self {
        self.options.core_path = core_path;
        self
    }

    /// Crée le projet dans un répertoire temporaire, rendu avec lui : le laisser tomber
    /// efface le projet.
    pub(crate) fn create(self) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = new::create(&self.options, parent.path()).expect("le projet doit se créer");

        (parent, project.root)
    }
}

/// Le projet par défaut, sans rien à préciser.
pub(crate) fn project() -> (TempDir, PathBuf) {
    Project::new().create()
}
