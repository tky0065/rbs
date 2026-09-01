use axum::routing::get;
use axum::{Json, Router};
use rbs_core::CommonResponses;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    modifiers(&CommonResponses),
    paths(
        crate::health::controller::health,
        // <rbs:openapi>
        crate::subscribers::controller::list,
        crate::subscribers::controller::filter,
        crate::subscribers::controller::create,
        crate::subscribers::controller::broadcast,
        crate::subscribers::controller::find,
        crate::subscribers::controller::update,
        crate::subscribers::controller::delete,
        // </rbs:openapi>
    )
)]
pub struct ApiDoc;

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

async fn document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
