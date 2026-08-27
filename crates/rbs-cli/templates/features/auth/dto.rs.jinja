use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Ce que rendent `login` et `refresh`.
///
/// `refresh_token` est le jeton en clair, remis une seule fois : la base n'en garde que
/// l'empreinte.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}
