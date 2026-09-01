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
        .route("/uploads", get(controller::list).post(controller::create))
        // Avant `/uploads/{id}`, sans quoi `filter` serait lu comme un identifiant
        // et rendrait un 400 sur un chemin pourtant monté.
        .route("/uploads/filter", post(controller::filter))
        .route(
            "/uploads/{id}",
            get(controller::find)
                .patch(controller::update)
                .delete(controller::delete),
        )
        // region: route_contenu
        .route(
            "/uploads/{id}/content",
            get(controller::get_content)
                .put(controller::put_content)
                .head(controller::head_content),
        )
    // endregion: route_contenu
}
