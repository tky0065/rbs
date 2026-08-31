use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
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
    Ok(Json(
        service::list(state.core().db(), state.cache(), &pagination).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/uploads",
    tag = "uploads",
    request_body = CreateUpload,
    responses(
        (status = 201, description = "upload créé", body = UploadResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreateUpload>,
) -> Result<(StatusCode, Json<UploadResponse>)> {
    let upload = service::create(state.core().db(), state.cache(), &state.mail, input).await?;

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
        (status = 404, description = "upload introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateUpload>,
) -> Result<Json<UploadResponse>> {
    Ok(Json(
        service::update(state.core().db(), state.cache(), id, input).await?,
    ))
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
    service::delete(state.core().db(), state.cache(), state.storage.as_ref(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}

// region: put_content
// Le contenu voyage hors du DTO : un corps binaire n'a pas sa place dans un JSON, et le
// faire passer en base64 obligerait à charger deux fois le fichier en mémoire.
#[utoipa::path(
    put,
    path = "/uploads/{id}/content",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    request_body(content = String, content_type = "application/octet-stream"),
    responses(
        (status = 204, description = "contenu déposé"),
        (status = 404, description = "upload introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn put_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    content: Bytes,
) -> Result<StatusCode> {
    service::put_content(
        state.core().db(),
        state.storage.as_ref(),
        id,
        content.to_vec(),
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
// endregion: put_content

#[utoipa::path(
    get,
    path = "/uploads/{id}/content",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    responses(
        (status = 200, description = "contenu du upload", content_type = "application/octet-stream"),
        (status = 404, description = "contenu introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn get_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let content = service::get_content(state.storage.as_ref(), id).await?;

    Ok(([("content-type", "application/octet-stream")], content))
}

// region: head_content
#[utoipa::path(
    head,
    path = "/uploads/{id}/content",
    tag = "uploads",
    params(("id" = Uuid, Path, description = "identifiant de upload")),
    responses(
        (status = 204, description = "un contenu est déposé"),
        (status = 404, description = "aucun contenu", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn head_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if service::has_content(state.storage.as_ref(), id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(rbs_core::Error::NotFound("contenu"))
    }
}
// endregion: head_content
