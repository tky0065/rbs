pub mod controller;
pub mod dto;
pub mod filter;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/posts", get(controller::list).post(controller::create))
        // Avant `/posts/{id}`, sans quoi `filter` serait lu comme un identifiant
        // et rendrait un 400 sur un chemin pourtant monté.
        .route("/posts/filter", post(controller::filter))
        .route(
            "/posts/{id}",
            get(controller::find)
                .patch(controller::update)
                .delete(controller::delete),
        )
}
