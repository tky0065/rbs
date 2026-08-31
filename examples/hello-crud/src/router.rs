use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::middleware::from_fn;
use rbs_core::HasCoreState;
use tower_http::timeout::TimeoutLayer;

use crate::health;
use crate::openapi;
use crate::state::AppState;

// region: montage
pub fn router(state: AppState) -> Router {
    let docs = openapi::routes(state.core().config());
    let timeout = Duration::from_secs(state.core().config().server.timeout_secs);

    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        .merge(crate::articles::routes())
        // </rbs:routes>
        .merge(docs)
        // Un `.layer()` enveloppe ceux qui le précèdent : posée ici, une couche ajoutée
        // par une feature s'exécute *après* `request_id` et `trace`, jamais avant. C'est
        // la seule position qui lui donne l'identifiant de la requête et qui fait entrer
        // ses propres réponses — un 429, un préflight refusé — dans le journal.
        // <rbs:layers>
        // La borne vient de `server.timeout_secs` : au-delà, la requête rend un 408 et
        // rend la connexion que la suivante attend.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            timeout,
        ))
        // </rbs:layers>
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
// endregion: montage
