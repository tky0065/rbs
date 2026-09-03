pub mod controller;
pub mod dto;
pub mod filter;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post, put};

use crate::state::AppState;

/// Taille maximale d'un contenu déposé. Relevez-la si vos fichiers sont plus gros ; elle
/// ne vaut que pour la route de dépôt, les routes JSON gardant la limite d'axum.
const TAILLE_MAX: usize = 10 * 1024 * 1024;

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
            put(controller::put_content)
                .get(controller::get_content)
                .head(controller::head_content)
                .layer(DefaultBodyLimit::max(TAILLE_MAX)),
        )
    // endregion: route_contenu
}
