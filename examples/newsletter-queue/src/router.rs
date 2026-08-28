use axum::Router;
use axum::middleware::from_fn;
use rbs_core::HasCoreState;

use crate::health;
use crate::openapi;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let docs = openapi::routes(state.core().config());

    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        .merge(crate::subscribers::routes())
        // </rbs:routes>
        .merge(docs)
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
