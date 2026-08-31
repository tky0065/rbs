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
        .merge(crate::auth::routes())
        .merge(crate::posts::routes())
        // </rbs:routes>
        .merge(docs)
        // Un `.layer()` enveloppe ceux qui le précèdent : posée ici, une couche ajoutée
        // par une feature s'exécute *après* `request_id` et `trace`, jamais avant. C'est
        // la seule position qui lui donne l'identifiant de la requête et qui fait entrer
        // ses propres réponses — un 429, un préflight refusé — dans le journal.
        // <rbs:layers>
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::rate_limit::middleware,
        ))
        // </rbs:layers>
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
