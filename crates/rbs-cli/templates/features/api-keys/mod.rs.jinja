pub mod controller;
pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{delete, post};

use crate::state::AppState;

// Les deux méthodes du même chemin se déclarent en une fois : axum refuse — et le dit par
// une panique au démarrage — deux `route()` sur un chemin identique.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api-keys",
            post(controller::create)
                .get(controller::list)
                .delete(controller::revoke_all),
        )
        .route("/api-keys/{id}", delete(controller::revoke))
}
