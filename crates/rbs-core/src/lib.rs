//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

//! # Feature flags
//!
//! Quatre extensions sont prévues. Seul `auth` porte du code ; les trois autres sont
//! **déclarés mais vides**, et servent seulement à réserver leur nom.
//!
//! | Flag | Ce qu'il active |
//! |---|---|
//! | `auth` | hachage Argon2, JWT, jetons opaques, extracteur d'identité |
//! | `redis` | client Redis partagé par l'état applicatif |
//! | `mail` | envoi de courriels et rendu de gabarits |
//! | `storage` | stockage de fichiers, local ou compatible S3 |

#![warn(missing_docs)]

/// Chargement et validation de la configuration de l'application.
pub mod config;
/// Ouverture du pool de connexions à la base.
pub mod db;
/// Erreurs du runtime et alias `Result` associé.
pub mod error;
/// Extracteurs de requête du runtime.
pub mod extract;
/// Hachage et vérification des mots de passe.
#[cfg(feature = "auth")]
pub mod hash;
/// Route de santé de l'application.
pub mod health;
/// Formateurs de logs du runtime.
pub mod logs;
/// Déclaration unique des réponses d'erreur du document OpenAPI.
pub mod openapi;
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
pub use openapi::{ProblemDetails, ReponsesCommunes};
pub use pagination::{Page, Pagination};
pub use state::{CoreState, HasCoreState};
