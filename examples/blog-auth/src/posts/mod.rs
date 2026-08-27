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
        .route("/posts", get(controller::list).post(controller::create))
        .route(
            "/posts/{id}",
            get(controller::find)
                .put(controller::update)
                .delete(controller::delete),
        )
}
