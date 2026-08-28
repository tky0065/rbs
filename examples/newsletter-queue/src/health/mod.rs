pub mod controller;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(controller::health))
}
