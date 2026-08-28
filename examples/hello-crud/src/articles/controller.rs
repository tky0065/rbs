use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};
use super::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/articles",
    tag = "articles",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses((status = 200, description = "page de articles", body = Page<ArticleResponse>))
)]
pub async fn list(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Page<ArticleResponse>>> {
    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

#[utoipa::path(
    post,
    path = "/articles",
    tag = "articles",
    request_body = CreateArticle,
    responses(
        (status = 201, description = "article créé", body = ArticleResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreateArticle>,
) -> Result<(StatusCode, Json<ArticleResponse>)> {
    let article = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(article)))
}

#[utoipa::path(
    get,
    path = "/articles/{id}",
    tag = "articles",
    params(("id" = Uuid, Path, description = "identifiant de article")),
    responses(
        (status = 200, description = "article demandé", body = ArticleResponse),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ArticleResponse>> {
    Ok(Json(service::find(state.core().db(), id).await?))
}

// Un champ absent du corps garde sa valeur : la mise à jour est une fusion, non un
// remplacement, malgré le verbe `PUT` qu'attend un client de CRUD.
#[utoipa::path(
    put,
    path = "/articles/{id}",
    tag = "articles",
    params(("id" = Uuid, Path, description = "identifiant de article")),
    request_body = UpdateArticle,
    responses(
        (status = 200, description = "article mis à jour", body = ArticleResponse),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateArticle>,
) -> Result<Json<ArticleResponse>> {
    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/articles/{id}",
    tag = "articles",
    params(("id" = Uuid, Path, description = "identifiant de article")),
    responses(
        (status = 204, description = "article supprimé"),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode> {
    service::delete(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
