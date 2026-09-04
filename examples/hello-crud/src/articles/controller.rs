use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};
use super::filter::ArticleFilter;
use super::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/articles",
    tag = "articles",
    operation_id = "articles_list",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de articles", body = Page<ArticleResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Page<ArticleResponse>>> {
    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
/// Le garde de rôle ne s'y applique donc pas, pas plus qu'à `list` ou `find`.
#[utoipa::path(
    post,
    path = "/articles/filter",
    tag = "articles",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = ArticleFilter,
    responses(
        (status = 200, description = "page de articles filtrés", body = Page<ArticleResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    pagination: Pagination,
    Json(filtre): Json<ArticleFilter>,
) -> Result<Json<Page<ArticleResponse>>> {
    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

// region: create
#[utoipa::path(
    post,
    path = "/articles",
    tag = "articles",
    operation_id = "articles_create",
    request_body = CreateArticle,
    responses(
        (status = 201, description = "article créé", body = ArticleResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreateArticle>,
) -> Result<(StatusCode, Json<ArticleResponse>)> {
    let article = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(article)))
}
// endregion: create

#[utoipa::path(
    get,
    path = "/articles/{id}",
    tag = "articles",
    operation_id = "articles_find",
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

#[utoipa::path(
    patch,
    path = "/articles/{id}",
    tag = "articles",
    operation_id = "articles_update",
    params(("id" = Uuid, Path, description = "identifiant de article")),
    request_body = UpdateArticle,
    responses(
        (status = 200, description = "article mis à jour", body = ArticleResponse),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
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
    operation_id = "articles_delete",
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
