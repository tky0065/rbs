//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

#![warn(missing_docs)]

/// Erreurs du runtime et alias `Result` associé.
pub mod error;

pub use error::{Error, Result};
