//! Bibliothèque du projet, partagée par le binaire principal et par `rbs seed`.
//!
//! Deux racines de crate ne s'atteignent pas l'une l'autre : c'est ici que vit tout ce
//! qu'elles doivent lire en commun, `AppState` comme le modèle de chaque feature.

mod health;
mod openapi;
pub mod router;
pub mod state;
// <rbs:features>
pub mod jobs;
pub mod mail;
pub mod subscribers;
// </rbs:features>
