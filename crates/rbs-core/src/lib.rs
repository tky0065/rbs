//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

//! # Feature flags
//!
//! Outre le pilote (`postgres`, `mysql`, `sqlite`), deux features portent du code. Redis,
//! le mail et le stockage n'en ont pas : ce sont des fragments que `rbs add` engendre
//! dans le projet.
//!
//! | Flag | Ce qu'il active |
//! |---|---|
//! | `auth` | hachage Argon2, JWT, jetons opaques, extracteur d'identité |
//! | `observability` | export OTLP des spans, greffé sur l'abonné que `logs::init()` pose |

#![warn(missing_docs)]

/// Chargement et validation de la configuration de l'application.
pub mod config;
/// Ouverture du pool de connexions à la base.
pub mod db;
/// Erreurs du runtime et alias `Result` associé.
pub mod error;
/// Extracteurs de requête du runtime.
pub mod extract;

/// Opérateurs de filtrage et de tri des listes.
pub mod filter;
/// Hachage et vérification des mots de passe.
#[cfg(feature = "auth")]
pub mod hash;
/// Route de santé de l'application.
pub mod health;
/// Signature et vérification des jetons d'accès.
#[cfg(feature = "auth")]
pub mod jwt;
/// Langue des réponses HTTP du projet.
pub mod lang;
/// Formateurs de logs du runtime.
pub mod logs;
/// Déclaration unique des réponses d'erreur du document OpenAPI.
pub mod openapi;
/// Pagination des listes.
pub mod pagination;
/// Identifiant de corrélation de la requête courante.
pub mod request_id;
/// Arrêt gracieux du processus : le signal, et les tâches de fond qu'il doit attendre.
pub mod shutdown;
/// État partagé du runtime.
pub mod state;
/// Tirage et empreinte des jetons opaques.
#[cfg(feature = "auth")]
pub mod token;
/// Trace d'une requête HTTP.
pub mod trace;

pub use config::Config;
pub use error::{Error, Result};
#[cfg(feature = "auth")]
pub use extract::Identity;
pub use extract::ValidatedJson;
pub use filter::schema::{
    BoolComparisonSchema, ComparisonSchema, DateTimeComparisonSchema, FloatComparisonSchema,
    IntComparisonSchema, TextMatchSchema, UuidComparisonSchema,
};
pub use filter::{Comparison, Sort, SortKey, TextMatch};
pub use lang::Lang;
pub use openapi::{CommonResponses, ProblemDetails};
pub use pagination::{Cursor, CursorPage, Page, Pagination};
#[cfg(feature = "auth")]
pub use state::HasAuth;
pub use state::{CoreState, HasCoreState};
