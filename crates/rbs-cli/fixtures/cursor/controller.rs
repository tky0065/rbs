//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{
    Cursor, CursorPage, HasCoreState, Identity, Page, Pagination, ProblemDetails, Result,
    ValidatedJson,
};
use sea_orm::prelude::Uuid;

use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};
use super::filter::ArticleFilter;
use super::service;
use crate::auth::guard::RequireRole;
use crate::auth::model::Role;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/articles",
    tag = "articles",
    operation_id = "articles_list",
    security(("bearer" = [])),
    params(
        ("after" = Option<Uuid>, Query, description = "identifiant après lequel reprendre ; absent, la première page"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de articles", body = CursorPage<ArticleResponse>),
        (status = 400, description = "curseur ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
    cursor: Cursor,
) -> Result<Json<CursorPage<ArticleResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::list(state.core().db(), &cursor).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
#[utoipa::path(
    post,
    path = "/articles/filter",
    tag = "articles",
    operation_id = "articles_filter",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = ArticleFilter,
    responses(
        (status = 200, description = "page de articles filtrés", body = Page<ArticleResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
    Json(filtre): Json<ArticleFilter>,
) -> Result<Json<Page<ArticleResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/articles",
    tag = "articles",
    operation_id = "articles_create",
    security(("bearer" = [])),
    request_body = CreateArticle,
    responses(
        (status = 201, description = "article créé", body = ArticleResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateArticle>,
) -> Result<(StatusCode, Json<ArticleResponse>)> {
    identite.require_role(Role::User)?;

    let article = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(article)))
}

#[utoipa::path(
    get,
    path = "/articles/{id}",
    tag = "articles",
    operation_id = "articles_find",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de article")),
    responses(
        (status = 200, description = "article demandé", body = ArticleResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ArticleResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::find(state.core().db(), id).await?))
}

#[utoipa::path(
    patch,
    path = "/articles/{id}",
    tag = "articles",
    operation_id = "articles_update",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de article")),
    request_body = UpdateArticle,
    responses(
        (status = 200, description = "article mis à jour", body = ArticleResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateArticle>,
) -> Result<Json<ArticleResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/articles/{id}",
    tag = "articles",
    operation_id = "articles_delete",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de article")),
    responses(
        (status = 204, description = "article supprimé"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "article introuvable", body = ProblemDetails, content_type = "application/problem+json")
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
