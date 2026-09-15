use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasAuth, HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::super::dto::{
    LoginRequest, RefreshRequest, RegisterRequest, SessionResponse, TokenPair, UserResponse,
};
use super::super::service;
use crate::state::AppState;

// Un 202 sans corps, que l'adresse soit neuve ou déjà prise : un 409, ou un profil rendu
// à la seule adresse neuve, dirait à qui essaie plusieurs adresses lesquelles sont
// inscrites.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    operation_id = "auth_register",
    request_body = RegisterRequest,
    responses(
        (status = 202, description = "inscription reçue, que l'adresse soit neuve ou non"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<RegisterRequest>,
) -> Result<StatusCode> {
    service::register(state.core().db(), state.mail(), state.flows(), input).await?;

    Ok(StatusCode::ACCEPTED)
}

// Un mot de passe erroné et un email inconnu rendent la même réponse : distinguer les
// deux dirait à un attaquant quels emails sont inscrits.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    operation_id = "auth_login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "paire de jetons", body = TokenPair),
        (status = 401, description = "identifiants refusés", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<LoginRequest>,
) -> Result<TokenPair> {
    service::login(state.core().db(), state.auth(), input).await
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    operation_id = "auth_refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "nouvelle paire de jetons", body = TokenPair),
        (status = 401, description = "jeton absent, expiré ou déjà consommé", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<RefreshRequest>,
) -> Result<TokenPair> {
    service::refresh(state.core().db(), state.auth(), input).await
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    operation_id = "auth_logout",
    request_body = RefreshRequest,
    responses(
        (status = 204, description = "session révoquée"),
        (status = 401, description = "jeton inconnu", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<RefreshRequest>,
) -> Result<StatusCode> {
    service::logout(state.core().db(), input).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    operation_id = "auth_me",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "profil de l'appelant", body = UserResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn me(State(state): State<AppState>, identite: Identity) -> Result<Json<UserResponse>> {
    let id = identite.user_uuid()?;

    Ok(Json(service::me(state.core().db(), id).await?))
}

#[utoipa::path(
    get,
    path = "/auth/sessions",
    tag = "auth",
    operation_id = "auth_list_sessions",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "sessions ouvertes de l'appelant", body = Vec<SessionResponse>),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    identite: Identity,
) -> Result<Json<Vec<SessionResponse>>> {
    let id = identite.user_uuid()?;

    Ok(Json(service::sessions(state.core().db(), id).await?))
}

#[utoipa::path(
    delete,
    path = "/auth/sessions/{id}",
    tag = "auth",
    operation_id = "auth_revoke_session",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de la session")),
    responses(
        (status = 204, description = "session révoquée"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "aucune session de l'appelant sous cet identifiant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
// region: revoke_session
pub async fn revoke_session(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let user_id = identite.user_uuid()?;

    service::revoke_session(state.core().db(), id, user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
// endregion: revoke_session

#[utoipa::path(
    delete,
    path = "/auth/sessions",
    tag = "auth",
    operation_id = "auth_revoke_sessions",
    security(("bearer" = [])),
    responses(
        (status = 204, description = "toutes les sessions de l'appelant révoquées, la sienne comprise"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke_sessions(
    State(state): State<AppState>,
    identite: Identity,
) -> Result<StatusCode> {
    let id = identite.user_uuid()?;

    service::revoke_sessions(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
