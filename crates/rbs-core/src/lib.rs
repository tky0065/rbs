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
/// Extracteurs de requête du runtime.
pub mod extract;
/// Formateurs de logs du runtime.
pub mod logs;
/// Pagination des listes.
pub mod pagination;
/// Identifiant de corrélation de la requête courante.
pub mod request_id;
/// État partagé du runtime.
pub mod state;
/// Trace d'une requête HTTP.
pub mod trace;

pub use config::Config;
pub use error::{Error, Result};
pub use extract::ValidatedJson;
pub use pagination::{Page, Pagination};
pub use state::{CoreState, HasCoreState};
