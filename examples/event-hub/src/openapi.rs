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
        crate::health::controller::live,
        crate::health::controller::health,
        // <rbs:openapi>
        crate::auth::controller::register,
        crate::auth::controller::login,
        crate::auth::controller::refresh,
        crate::auth::controller::logout,
        crate::auth::controller::me,
        crate::auth::controller::password::change_password,
        crate::auth::controller::password::forgot_password,
        crate::auth::controller::password::reset_password,
        crate::auth::controller::verification::verify_email,
        crate::auth::controller::verification::resend_verification,
        crate::auth::controller::list_sessions,
        crate::auth::controller::revoke_session,
        crate::auth::controller::revoke_sessions,
        crate::modules::webhooks::controller::subscribe,
        crate::modules::webhooks::controller::list,
        crate::modules::webhooks::controller::revoke,
        crate::orders::controller::list,
        crate::orders::controller::filter,
        crate::orders::controller::create,
        crate::orders::controller::find,
        crate::orders::controller::update,
        crate::orders::controller::delete,
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
