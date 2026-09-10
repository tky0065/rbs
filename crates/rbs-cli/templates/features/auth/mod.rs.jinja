pub mod config;
pub mod controller;
pub mod dto;
pub mod guard;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{get, post};
use rbs_core::HasAuth;

use crate::state::AppState;

// `HasAuth` a un corps par défaut, mais n'est pas implémenté pour tout état : c'est ce
// qui laisse un projet tirer son secret d'ailleurs qu'un fichier de configuration.
// L'implémentation vit ici plutôt que dans `state.rs` — elle arrive avec la feature, et
// repart avec elle.
impl HasAuth for AppState {}

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart avec
// elle.
impl AppState {
    /// Les réglages des parcours de réinitialisation et de vérification.
    pub fn flows(&self) -> &config::FlowConfig {
        &self.flows
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(controller::register))
        .route("/auth/login", post(controller::login))
        .route("/auth/refresh", post(controller::refresh))
        .route("/auth/logout", post(controller::logout))
        .route("/auth/me", get(controller::me))
        .route(
            "/auth/change-password",
            post(controller::password::change_password),
        )
        .route(
            "/auth/forgot-password",
            post(controller::password::forgot_password),
        )
        .route(
            "/auth/reset-password",
            post(controller::password::reset_password),
        )
}
