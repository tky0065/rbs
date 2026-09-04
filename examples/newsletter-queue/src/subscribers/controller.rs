use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Page, Pagination, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{
    Broadcast, BroadcastAccepted, CreateSubscriber, SubscriberResponse, UpdateSubscriber,
};
use super::service;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/subscribers",
    tag = "subscribers",
    operation_id = "subscribers_list",
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

#[utoipa::path(
    post,
    path = "/subscribers",
    tag = "subscribers",
    operation_id = "subscribers_create",
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
    operation_id = "subscribers_broadcast",
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
    operation_id = "subscribers_find",
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
    operation_id = "subscribers_update",
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
    operation_id = "subscribers_delete",
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
