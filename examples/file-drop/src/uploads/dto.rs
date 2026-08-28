use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::Model;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateUpload {
    pub title: String,
    #[validate(email)]
    pub owner_email: String,
    pub content_type: String,
    pub size: i32,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateUpload {
    pub title: Option<String>,
    #[validate(email)]
    pub owner_email: Option<String>,
    pub content_type: Option<String>,
    pub size: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    pub id: Uuid,
    pub title: String,
    pub owner_email: String,
    pub content_type: String,
    pub size: i32,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for UploadResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            title: model.title,
            owner_email: model.owner_email,
            content_type: model.content_type,
            size: model.size,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
