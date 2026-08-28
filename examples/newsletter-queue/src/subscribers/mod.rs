pub mod controller;
pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/subscribers",
            get(controller::list).post(controller::create),
        )
        // Avant `/subscribers/{id}`, sans quoi `broadcast` serait lu comme un
        // identifiant et rendrait un 400 sur un chemin pourtant monté.
        .route(
            "/subscribers/broadcast",
            axum::routing::post(controller::broadcast),
        )
        .route(
            "/subscribers/{id}",
            get(controller::find)
                .put(controller::update)
                .delete(controller::delete),
        )
}
