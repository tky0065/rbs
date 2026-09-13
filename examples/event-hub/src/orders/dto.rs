use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::Model;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateOrder {
    #[validate(length(max = 255))]
    pub reference: String,
    pub amount: i32,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateOrder {
    #[validate(length(max = 255))]
    pub reference: Option<String>,
    pub amount: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderResponse {
    pub id: Uuid,
    pub reference: String,
    pub amount: i32,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for OrderResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            reference: model.reference,
            amount: model.amount,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
