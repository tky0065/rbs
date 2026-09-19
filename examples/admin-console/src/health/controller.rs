use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use rbs_core::HasCoreState;

use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    operation_id = "health",
    responses((status = 200, description = "l'application et ses dépendances répondent"))
)]
pub async fn health(State(state): State<AppState>) -> Response {
    rbs_core::health::report(
        state.core().db(),
        vec![
            // <rbs:health_probes>
            // </rbs:health_probes>
        ],
    )
    .await
}

// Ne prend pas l'état : une sonde de vie qui interrogerait la base ferait redémarrer l'API
// en boucle le jour où c'est la base qui tombe. Cette question-là est celle de `/health`.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    operation_id = "health_live",
    responses((status = 200, description = "le processus répond, sans rien interroger"))
)]
pub async fn live() -> StatusCode {
    StatusCode::OK
}
