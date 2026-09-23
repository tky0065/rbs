//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Identity, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{CommentaireResponse, CreateCommentaire, UpdateCommentaire};
use super::filter::CommentaireFilter;
use super::service;
use crate::auth::guard::RequireRole;
use crate::auth::model::Role;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/commentaires",
    tag = "commentaires",
    operation_id = "commentaires_list",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de commentaires", body = Page<CommentaireResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
) -> Result<Json<Page<CommentaireResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
#[utoipa::path(
    post,
    path = "/commentaires/filter",
    tag = "commentaires",
    operation_id = "commentaires_filter",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = CommentaireFilter,
    responses(
        (status = 200, description = "page de commentaires filtrés", body = Page<CommentaireResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
    Json(filtre): Json<CommentaireFilter>,
) -> Result<Json<Page<CommentaireResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/commentaires",
    tag = "commentaires",
    operation_id = "commentaires_create",
    security(("bearer" = [])),
    request_body = CreateCommentaire,
    responses(
        (status = 201, description = "commentaire créé", body = CommentaireResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateCommentaire>,
) -> Result<(StatusCode, Json<CommentaireResponse>)> {
    identite.require_role(Role::User)?;
    // L'auteur est l'appelant : lu dans le corps, il laisserait écrire au nom d'autrui.
    let auteur = identite.user_uuid()?;

    let commentaire = service::create(state.core().db(), auteur, input).await?;

    Ok((StatusCode::CREATED, Json(commentaire)))
}

#[utoipa::path(
    get,
    path = "/commentaires/{id}",
    tag = "commentaires",
    operation_id = "commentaires_find",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de commentaire")),
    responses(
        (status = 200, description = "commentaire demandé", body = CommentaireResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "commentaire introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<CommentaireResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::find(state.core().db(), id).await?))
}

#[utoipa::path(
    patch,
    path = "/commentaires/{id}",
    tag = "commentaires",
    operation_id = "commentaires_update",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de commentaire")),
    request_body = UpdateCommentaire,
    responses(
        (status = 200, description = "commentaire mis à jour", body = CommentaireResponse),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "commentaire introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateCommentaire>,
) -> Result<Json<CommentaireResponse>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/commentaires/{id}",
    tag = "commentaires",
    operation_id = "commentaires_delete",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de commentaire")),
    responses(
        (status = 204, description = "commentaire supprimé"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "commentaire introuvable", body = ProblemDetails, content_type = "application/problem+json")
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
