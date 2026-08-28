use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::Model;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateSubscriber {
    #[validate(email)]
    pub email: String,
    pub name: String,
    pub confirmed: bool,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateSubscriber {
    #[validate(email)]
    pub email: Option<String>,
    pub name: Option<String>,
    pub confirmed: Option<bool>,
}

// region: broadcast_dto
/// La lettre à diffuser, telle que la reçoit `POST /subscribers/broadcast`.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct Broadcast {
    #[validate(length(min = 1))]
    pub subject: String,
    #[validate(length(min = 1))]
    pub body: String,
}

/// Ce que la diffusion rend : le nombre de lettres enfilées, non envoyées.
#[derive(Debug, Serialize, ToSchema)]
pub struct BroadcastAccepted {
    pub enqueued: usize,
}
// endregion: broadcast_dto

#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriberResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub confirmed: bool,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for SubscriberResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            email: model.email,
            name: model.name,
            confirmed: model.confirmed,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
