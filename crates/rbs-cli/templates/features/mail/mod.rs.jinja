//! Envoi de courriels par SMTP.
//!
//! Le transport est bâti une fois au démarrage et partagé par l'état : `lettre` tient son
//! propre pool de connexions, qu'un transport par message rendrait inutile.
//!
//! Aucune route n'est montée ici. Le module est la brique ; l'usage appartient au projet.

pub mod config;
mod service;
pub mod template;

#[cfg(test)]
mod tests;

pub use service::Mailer;

use crate::state::AppState;

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart
// avec elle.
impl AppState {
    /// Le transport partagé, tel qu'un handler le lit depuis l'état.
    pub fn mail(&self) -> &Mailer {
        &self.mail
    }
}
