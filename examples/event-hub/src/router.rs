use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::middleware::from_fn;
use rbs_core::HasCoreState;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tower_http::timeout::TimeoutLayer;

use crate::health;
use crate::openapi;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let docs = openapi::routes(state.core().config());
    let timeout = Duration::from_secs(state.core().config().server.timeout_secs);
    // Le document OpenAPI dépasse vite la centaine de Ko. Le prédicat par défaut épargne
    // les petits corps, les images et les flux SSE, que la compression ralentirait sans
    // rien gagner ; un binaire s'y ajoute : servi tel quel, il garde son `content-length`,
    // et une archive déjà compressée ne l'est pas une seconde fois.
    let compression =
        DefaultPredicate::new().and(NotForContentType::const_new("application/octet-stream"));

    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        .merge(crate::auth::routes())
        .merge(crate::modules::webhooks::routes())
        .merge(crate::orders::routes())
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
        .layer(CompressionLayer::new().compress_when(compression))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::modules::rate_limit::middleware,
        ))
        .layer(crate::modules::cors::layer())
        // </rbs:layers>
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
