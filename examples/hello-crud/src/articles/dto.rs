use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use super::model::Model;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateArticle {
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateArticle {
    pub title: Option<String>,
    pub body: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ArticleResponse {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub published: bool,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeWithTimeZone,
}

impl From<Model> for ArticleResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            title: model.title,
            body: model.body,
            published: model.published,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
