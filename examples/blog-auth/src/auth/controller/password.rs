use axum::extract::State;
use axum::http::StatusCode;
use rbs_core::{HasAuth, HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};

use super::super::dto::{ChangePasswordRequest, EmailRequest, ResetPasswordRequest, TokenPair};
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
    let id = identite.user_uuid()?;

    service::password::change(state.core().db(), state.auth(), id, input).await
}

// region: forgot_password
// Le statut est 202 quoi qu'il arrive, et seule la lecture du compte est attendue :
// l'émission du jeton et l'envoi partent détachés. Les attendre rendrait par le temps
// de réponse ce que le code de statut refuse de dire — une adresse inconnue n'écrit
// rien et n'envoie rien.
#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = "auth",
    operation_id = "auth_forgot_password",
    request_body = EmailRequest,
    responses(
        (status = 202, description = "demande acceptée, que l'adresse soit inscrite ou non"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<EmailRequest>,
) -> Result<StatusCode> {
    service::password::send_reset_link(
        state.core().db(),
        state.mail(),
        state.flows(),
        &input.email,
    )
    .await?;

    Ok(StatusCode::ACCEPTED)
}
// endregion: forgot_password

// region: reset_password
#[utoipa::path(
    post,
    path = "/auth/reset-password",
    tag = "auth",
    operation_id = "auth_reset_password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "mot de passe réinitialisé, sessions révoquées"),
        (status = 401, description = "jeton inconnu, périmé ou déjà consommé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<ResetPasswordRequest>,
) -> Result<StatusCode> {
    service::password::reset(state.core().db(), input).await?;

    Ok(StatusCode::NO_CONTENT)
}
// endregion: reset_password
