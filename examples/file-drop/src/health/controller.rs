use axum::extract::State;
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
            rbs_core::health::Probe::new("cache", state.cache().ping()),
            rbs_core::health::Probe::new("storage", crate::storage::probe(state.storage())),
            // </rbs:health_probes>
        ],
    )
    .await
}
