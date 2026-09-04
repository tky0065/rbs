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
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de posts", body = Page<PostResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Page<PostResponse>>> {
    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

// region: create
/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
/// Le garde de rôle ne s'y applique donc pas, pas plus qu'à `list` ou `find`.
#[utoipa::path(
    post,
    path = "/posts/filter",
    tag = "posts",
    operation_id = "posts_filter",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = PostFilter,
    responses(
        (status = 200, description = "page de posts filtrés", body = Page<PostResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    pagination: Pagination,
    Json(filtre): Json<PostFilter>,
) -> Result<Json<Page<PostResponse>>> {
    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

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
        (status = 403, description = "réservé aux administrateurs", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(entree): ValidatedJson<CreatePost>,
) -> Result<(StatusCode, Json<PostResponse>)> {
    identite.require_role(Role::Admin)?;

    let post = service::create(state.core().db(), entree).await?;

    Ok((StatusCode::CREATED, Json(post)))
}
// endregion: create

#[utoipa::path(
    get,
    path = "/posts/{id}",
    tag = "posts",
    operation_id = "posts_find",
    params(("id" = Uuid, Path, description = "identifiant de post")),
    responses(
        (status = 200, description = "post demandé", body = PostResponse),
        (status = 404, description = "post introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PostResponse>> {
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
        (status = 403, description = "réservé aux administrateurs", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "post introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
    ValidatedJson(entree): ValidatedJson<UpdatePost>,
) -> Result<Json<PostResponse>> {
    identite.require_role(Role::Admin)?;

    Ok(Json(service::update(state.core().db(), id, entree).await?))
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
        (status = 403, description = "réservé aux administrateurs", body = ProblemDetails, content_type = "application/problem+json"),
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
