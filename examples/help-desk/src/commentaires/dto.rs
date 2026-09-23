use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::Model;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateCommentaire {
    pub corps: String,
    pub ticket_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateCommentaire {
    pub corps: Option<String>,
    pub ticket_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommentaireResponse {
    pub id: Uuid,
    pub corps: String,
    pub ticket_id: Uuid,
    pub auteur_id: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for CommentaireResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            corps: model.corps,
            ticket_id: model.ticket_id,
            auteur_id: model.auteur_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
