//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Identity, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{CreateIncident, IncidentResponse, UpdateIncident};
use super::filter::IncidentFilter;
use super::service;
use crate::auth::guard::RequireRole;
use crate::auth::model::Role;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/incidents",
    tag = "incidents",
    operation_id = "incidents_list",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de incidents", body = Page<IncidentResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
) -> Result<Json<Page<IncidentResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
#[utoipa::path(
    post,
    path = "/incidents/filter",
    tag = "incidents",
    operation_id = "incidents_filter",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = IncidentFilter,
    responses(
        (status = 200, description = "page de incidents filtrés", body = Page<IncidentResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
    Json(filtre): Json<IncidentFilter>,
) -> Result<Json<Page<IncidentResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/incidents",
    tag = "incidents",
    operation_id = "incidents_create",
    security(("bearer" = [])),
    request_body = CreateIncident,
    responses(
        (status = 201, description = "incident créé", body = IncidentResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateIncident>,
) -> Result<(StatusCode, Json<IncidentResponse>)> {
    identite.require_role(Role::User)?;

    let incident = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(incident)))
}

#[utoipa::path(
    get,
    path = "/incidents/{id}",
    tag = "incidents",
    operation_id = "incidents_find",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de incident")),
    responses(
        (status = 200, description = "incident demandé", body = IncidentResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "incident introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<IncidentResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::find(state.core().db(), id).await?))
}

#[utoipa::path(
    patch,
    path = "/incidents/{id}",
    tag = "incidents",
    operation_id = "incidents_update",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de incident")),
    request_body = UpdateIncident,
    responses(
        (status = 200, description = "incident mis à jour", body = IncidentResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "incident introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateIncident>,
) -> Result<Json<IncidentResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/incidents/{id}",
    tag = "incidents",
    operation_id = "incidents_delete",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de incident")),
    responses(
        (status = 204, description = "incident supprimé"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "incident introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    identite.require_role(Role::User)?;

    service::delete(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
