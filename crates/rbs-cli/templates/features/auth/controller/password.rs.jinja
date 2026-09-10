use axum::extract::State;
use rbs_core::{HasAuth, HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, TokenPair};
use super::super::service;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/auth/change-password",
    tag = "auth",
    operation_id = "auth_change_password",
    security(("bearer" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "mot de passe changé, paire neuve", body = TokenPair),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "mot de passe courant refusé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<ChangePasswordRequest>,
) -> Result<TokenPair> {
    // `sub` porte l'identifiant sous forme de chaîne : le jeton est signé, mais rien ne
    // garantit que celui-ci a été émis par une version du service qui y mettait un UUID.
    let id = Uuid::parse_str(&identite.user_id).map_err(|_| rbs_core::Error::Unauthorized)?;

    service::password::change(state.core().db(), state.auth(), id, input).await
}
