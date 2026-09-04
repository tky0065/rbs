use axum::routing::get;
use axum::{Json, Router};
use rbs_core::CommonResponses;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

// region: document
#[derive(OpenApi)]
#[openapi(
    modifiers(&CommonResponses),
    paths(
        crate::health::controller::health,
        // <rbs:openapi>
        crate::articles::controller::list,
        crate::articles::controller::filter,
        crate::articles::controller::create,
        crate::articles::controller::find,
        crate::articles::controller::update,
        crate::articles::controller::delete,
        // </rbs:openapi>
    )
)]
pub struct ApiDoc;
// endregion: document

// region: exposition
pub fn routes(config: &rbs_core::Config) -> Router<AppState> {
    match (config.docs.swagger_ui, config.docs.openapi_json) {
        // Swagger UI charge le document par HTTP et monte lui-même sa route : l'afficher
        // implique de l'exposer, et le router une seconde fois ferait paniquer Axum au
        // démarrage. Pour n'exposer que le document, coupez `docs.swagger_ui`.
        (true, _) => Router::new()
            .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi())),
        (false, true) => Router::new().route("/api-docs/openapi.json", get(document)),
        (false, false) => Router::new(),
    }
}
// endregion: exposition

async fn document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
