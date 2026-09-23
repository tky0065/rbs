use axum::extract::State;
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};

use super::super::dto::EmailRequest;
use super::super::service;
use crate::state::AppState;

// L'adresse et rien d'autre : `EmailRequest` ne porte que ce champ, et le rôle comme la
// date de vérification n'ont aucun chemin d'écriture depuis le client.
//
// Un 202 sans corps, que l'adresse soit libre ou déjà prise — la règle de
// `/auth/register`, et pour la même raison : un statut ou un corps qui différerait dirait
// à qui en essaie plusieurs lesquelles sont inscrites.
#[utoipa::path(
    patch,
    path = "/auth/me",
    tag = "auth",
    operation_id = "auth_update_me",
    security(("bearer" = [])),
    request_body = EmailRequest,
    responses(
        (status = 202, description = "changement reçu, que l'adresse soit libre ou non"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<EmailRequest>,
) -> Result<StatusCode> {
    let id = identite.user_uuid()?;

    service::change_email(state.core().db(), state.mail(), state.flows(), id, input).await?;

    Ok(StatusCode::ACCEPTED)
}
