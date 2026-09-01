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
        crate::uploads::controller::list,
        crate::uploads::controller::filter,
        crate::uploads::controller::create,
        crate::uploads::controller::find,
        crate::uploads::controller::update,
        crate::uploads::controller::delete,
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
