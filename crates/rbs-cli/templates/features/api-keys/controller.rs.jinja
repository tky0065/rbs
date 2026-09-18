use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{Error, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::ActiveEnum;
use sea_orm::prelude::Uuid;

use super::dto::{ApiKeyCreated, ApiKeyResponse, CreateApiKey};
use super::service;
use crate::auth::model::Role;
use crate::state::AppState;

// Les quatre routes n'exigent aucun rôle : chacun administre ses propres clés, comme
// chacun administre ses propres sessions. Ce qu'une clé créée ici pourra faire est borné
// par le rôle de son créateur, que `service::create` vérifie.
//
// Une clé peut en créer une autre : le plafond interdit l'escalade, et l'interdire
// casserait le provisionnement automatisé, qui est la raison d'être du fragment.

#[utoipa::path(
    post,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_create",
    security(("bearer" = []), ("api_key" = [])),
    request_body = CreateApiKey,
    responses(
        (status = 201, description = "clé tirée, rendue cette seule fois", body = ApiKeyCreated),
        (status = 400, description = "rôle inconnu", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle demandé supérieur à celui de l'appelant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateApiKey>,
) -> Result<(StatusCode, ApiKeyCreated)> {
    let porteur = identite.user_uuid()?;
    // Le rôle de l'appelant tel qu'il est porté par son justificatif : c'est déjà le
    // moindre du rôle de la clé et de celui du compte quand l'appel vient d'une clé.
    let sien = Role::try_from_value(&identite.role).map_err(|_| Error::Forbidden)?;

    let creee = service::create(&state, porteur, sien, input).await?;

    Ok((StatusCode::CREATED, creee))
}

#[utoipa::path(
    get,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_list",
    security(("bearer" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "les clés de l'appelant, révoquées comprises, sans de quoi les présenter", body = Vec<ApiKeyResponse>),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
) -> Result<Json<Vec<ApiKeyResponse>>> {
    let porteur = identite.user_uuid()?;

    Ok(Json(service::list(&state, porteur).await?))
}

#[utoipa::path(
    delete,
    path = "/api-keys/{id}",
    tag = "api-keys",
    operation_id = "api_keys_revoke",
    security(("bearer" = []), ("api_key" = [])),
    params(("id" = Uuid, Path, description = "identifiant de la clé")),
    responses(
        (status = 204, description = "clé révoquée"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "aucune clé vivante de l'appelant sous cet identifiant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let porteur = identite.user_uuid()?;

    // 404 et non 403 : un 403 confirmerait que cette clé existe chez quelqu'un d'autre.
    if service::revoke(&state, id, porteur).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound("clé d'API"))
    }
}

#[utoipa::path(
    delete,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_revoke_all",
    security(("bearer" = []), ("api_key" = [])),
    responses(
        (status = 204, description = "toutes les clés de l'appelant révoquées, celle qui appelle comprise"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke_all(State(state): State<AppState>, identite: Identity) -> Result<StatusCode> {
    let porteur = identite.user_uuid()?;

    service::revoke_all(&state, porteur).await?;

    Ok(StatusCode::NO_CONTENT)
}
