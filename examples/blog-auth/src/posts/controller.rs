//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Identity, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{CreatePost, PostResponse, UpdatePost};
use super::filter::PostFilter;
use super::service;
use crate::auth::guard::RequireRole;
use crate::auth::model::Role;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/posts",
    tag = "posts",
    operation_id = "posts_list",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de posts", body = Page<PostResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
) -> Result<Json<Page<PostResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
#[utoipa::path(
    post,
    path = "/posts/filter",
    tag = "posts",
    operation_id = "posts_filter",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = PostFilter,
    responses(
        (status = 200, description = "page de posts filtrés", body = Page<PostResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
    Json(filtre): Json<PostFilter>,
) -> Result<Json<Page<PostResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

// region: create
#[utoipa::path(
    post,
    path = "/posts",
    tag = "posts",
    operation_id = "posts_create",
    security(("bearer" = [])),
    request_body = CreatePost,
    responses(
        (status = 201, description = "post créé", body = PostResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreatePost>,
) -> Result<(StatusCode, Json<PostResponse>)> {
    identite.require_role(Role::Admin)?;

    let post = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(post)))
}
// endregion: create

#[utoipa::path(
    get,
    path = "/posts/{id}",
    tag = "posts",
    operation_id = "posts_find",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de post")),
    responses(
        (status = 200, description = "post demandé", body = PostResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "post introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<PostResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::find(state.core().db(), id).await?))
}

#[utoipa::path(
    patch,
    path = "/posts/{id}",
    tag = "posts",
    operation_id = "posts_update",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de post")),
    request_body = UpdatePost,
    responses(
        (status = 200, description = "post mis à jour", body = PostResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "post introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdatePost>,
) -> Result<Json<PostResponse>> {
    identite.require_role(Role::Admin)?;

    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/posts/{id}",
    tag = "posts",
    operation_id = "posts_delete",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de post")),
    responses(
        (status = 204, description = "post supprimé"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "post introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    identite.require_role(Role::Admin)?;

    service::delete(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
