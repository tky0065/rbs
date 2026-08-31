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
        .route("/uploads", get(controller::list).post(controller::create))
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
