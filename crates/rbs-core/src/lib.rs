//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

#![warn(missing_docs)]

/// Chargement et validation de la configuration de l'application.
pub mod config;
/// Ouverture du pool de connexions à la base.
pub mod db;
/// Erreurs du runtime et alias `Result` associé.
pub mod error;
/// Formateurs de logs du runtime.
pub mod logs;
/// Identifiant de corrélation de la requête courante.
pub mod request_id;
/// État partagé du runtime.
pub mod state;

pub use config::Config;
pub use error::{Error, Result};
pub use state::{CoreState, HasCoreState};
