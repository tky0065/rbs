//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

//! # Feature flags
//!
//! Quatre extensions sont prévues pour la v0.2. Leurs flags sont **déclarés mais vides** :
//! les activer ne change rien en v0.1, et sert seulement à réserver leur nom.
//!
//! | Flag | Ce qu'il activera |
//! |---|---|
//! | `auth` | hachage Argon2, JWT, extracteur d'identité |
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
