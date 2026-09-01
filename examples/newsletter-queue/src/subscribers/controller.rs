use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{
    Broadcast, BroadcastAccepted, CreateSubscriber, SubscriberResponse, UpdateSubscriber,
};
use super::filter::SubscriberFilter;
use super::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/subscribers",
    tag = "subscribers",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de subscribers", body = Page<SubscriberResponse>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    pagination: Pagination,
) -> Result<Json<Page<SubscriberResponse>>> {
    Ok(Json(service::list(state.core().db(), &pagination).await?))
}

/// Filtrer est une lecture : le corps porte les conditions, que l'URL rendrait illisibles.
/// Le garde de rôle ne s'y applique donc pas, pas plus qu'à `list` ou `find`.
#[utoipa::path(
    post,
    path = "/subscribers/filter",
    tag = "subscribers",
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = SubscriberFilter,
    responses(
        (status = 200, description = "page de subscribers filtrés", body = Page<SubscriberResponse>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter(
    State(state): State<AppState>,
    pagination: Pagination,
    Json(filtre): Json<SubscriberFilter>,
) -> Result<Json<Page<SubscriberResponse>>> {
    Ok(Json(
        service::filter(state.core().db(), &filtre, &pagination).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/subscribers",
    tag = "subscribers",
    request_body = CreateSubscriber,
    responses(
        (status = 201, description = "subscriber créé", body = SubscriberResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreateSubscriber>,
) -> Result<(StatusCode, Json<SubscriberResponse>)> {
    let subscriber = service::create(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(subscriber)))
}

// region: broadcast
// `202` et non `200` : la réponse dit que les lettres sont enfilées, pas qu'elles sont
// parties. Ce que le client tient est un accusé de prise en charge.
#[utoipa::path(
    post,
    path = "/subscribers/broadcast",
    tag = "subscribers",
    request_body = Broadcast,
    responses(
        (status = 202, description = "lettres enfilées", body = BroadcastAccepted),
        (status = 422, description = "lettre invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn broadcast(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<Broadcast>,
) -> Result<(StatusCode, Json<BroadcastAccepted>)> {
    let enqueued = service::broadcast(state.core().db(), input).await?;

    Ok((StatusCode::ACCEPTED, Json(BroadcastAccepted { enqueued })))
}
// endregion: broadcast

#[utoipa::path(
    get,
    path = "/subscribers/{id}",
    tag = "subscribers",
    params(("id" = Uuid, Path, description = "identifiant de subscriber")),
    responses(
        (status = 200, description = "subscriber demandé", body = SubscriberResponse),
        (status = 404, description = "subscriber introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SubscriberResponse>> {
    Ok(Json(service::find(state.core().db(), id).await?))
}

#[utoipa::path(
    patch,
    path = "/subscribers/{id}",
    tag = "subscribers",
    params(("id" = Uuid, Path, description = "identifiant de subscriber")),
    request_body = UpdateSubscriber,
    responses(
        (status = 200, description = "subscriber mis à jour", body = SubscriberResponse),
        (status = 404, description = "subscriber introuvable", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 409, description = "valeur déjà prise sur une colonne unique", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateSubscriber>,
) -> Result<Json<SubscriberResponse>> {
    Ok(Json(service::update(state.core().db(), id, input).await?))
}

#[utoipa::path(
    delete,
    path = "/subscribers/{id}",
    tag = "subscribers",
    params(("id" = Uuid, Path, description = "identifiant de subscriber")),
    responses(
        (status = 204, description = "subscriber supprimé"),
        (status = 404, description = "subscriber introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode> {
    service::delete(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
