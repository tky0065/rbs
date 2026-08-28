use axum::extract::State;
use axum::response::Response;

use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "l'application et ses dépendances répondent"))
)]
pub async fn health(state: State<AppState>) -> Response {
    rbs_core::health::handler(state).await
}
