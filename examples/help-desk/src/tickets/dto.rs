use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::{Model, TicketPriorite, TicketStatut};

// region: entree
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateTicket {
    #[validate(length(max = 255))]
    pub sujet: String,
    pub detail: String,
    pub statut: TicketStatut,
    pub priorite: TicketPriorite,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateTicket {
    #[validate(length(max = 255))]
    pub sujet: Option<String>,
    pub detail: Option<String>,
    pub statut: Option<TicketStatut>,
    pub priorite: Option<TicketPriorite>,
}
// endregion: entree

#[derive(Debug, Serialize, ToSchema)]
pub struct TicketResponse {
    pub id: Uuid,
    pub sujet: String,
    pub detail: String,
    pub statut: TicketStatut,
    pub priorite: TicketPriorite,
    pub auteur_id: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for TicketResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            sujet: model.sujet,
            detail: model.detail,
            statut: model.statut,
            priorite: model.priorite,
            auteur_id: model.auteur_id,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
