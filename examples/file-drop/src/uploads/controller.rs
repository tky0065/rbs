use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{CreateUpload, UpdateUpload, UploadResponse};
use super::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/uploads",
    tag = "uploads",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses((status = 200, description = "page de uploads", body = Page<UploadResponse>))
)]
pub async fn list(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Page<UploadResponse>>> {
    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

#[utoipa::path(
    post,
    path = "/uploads",
    tag = "uploads",
    request_body = CreateUpload,
    responses(
        (status = 201, description = "upload créé", body = UploadResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(entree): ValidatedJson<CreateUpload>,
) -> Result<(StatusCode, Json<UploadResponse>)> {
    let upload = service::create(state.core().db(), entree).await?;

    Ok((StatusCode::CREATED, Json(upload)))
}

#[utoipa::path(
    get,
    path = "/uploads/{id}",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    responses(
        (status = 200, description = "upload demandé", body = UploadResponse),
        (status = 404, description = "upload introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UploadResponse>> {
    Ok(Json(service::find(state.core().db(), id).await?))
}

// Un champ absent du corps garde sa valeur : la mise à jour est une fusion, non un
// remplacement, malgré le verbe `PUT` qu'attend un client de CRUD.
#[utoipa::path(
    put,
    path = "/uploads/{id}",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    request_body = UpdateUpload,
    responses(
        (status = 200, description = "upload mis à jour", body = UploadResponse),
        (status = 404, description = "upload introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(entree): ValidatedJson<UpdateUpload>,
) -> Result<Json<UploadResponse>> {
    Ok(Json(service::update(state.core().db(), id, entree).await?))
}

#[utoipa::path(
    delete,
    path = "/uploads/{id}",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    responses(
        (status = 204, description = "upload supprimé"),
        (status = 404, description = "upload introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode> {
    service::delete(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
