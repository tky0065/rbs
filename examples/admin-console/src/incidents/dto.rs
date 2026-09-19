use sea_orm::prelude::Date;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::{IncidentGravite, Model};

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateIncident {
    #[validate(length(max = 255))]
    pub reference: String,
    #[validate(length(max = 255))]
    pub sujet: String,
    pub detail: String,
    pub gravite: IncidentGravite,
    pub ouvert: bool,
    pub duree_minutes: Option<i32>,
    pub echeance: Option<Date>,
    #[schema(value_type = String, format = DateTime)]
    pub constate_le: DateTimeWithTimeZone,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateIncident {
    #[validate(length(max = 255))]
    pub reference: Option<String>,
    #[validate(length(max = 255))]
    pub sujet: Option<String>,
    pub detail: Option<String>,
    pub gravite: Option<IncidentGravite>,
    pub ouvert: Option<bool>,
    pub duree_minutes: Option<i32>,
    pub echeance: Option<Date>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub constate_le: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IncidentResponse {
    pub id: Uuid,
    pub reference: String,
    pub sujet: String,
    pub detail: String,
    pub gravite: IncidentGravite,
    pub ouvert: bool,
    pub duree_minutes: Option<i32>,
    pub echeance: Option<Date>,
    #[schema(value_type = String, format = DateTime)]
    pub constate_le: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for IncidentResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            reference: model.reference,
            sujet: model.sujet,
            detail: model.detail,
            gravite: model.gravite,
            ouvert: model.ouvert,
            duree_minutes: model.duree_minutes,
            echeance: model.echeance,
            constate_le: model.constate_le,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
