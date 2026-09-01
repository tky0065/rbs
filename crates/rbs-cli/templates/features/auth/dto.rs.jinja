use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 12, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    // La borne haute vaut autant ici : sans elle, `/auth/login` hache en Argon2 tout ce
    // qu'on lui poste, sans qu'aucun compte n'ait à exister.
    #[validate(length(min = 12, max = 128))]
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

/// La paire porte sa propre réponse pour n'être jamais mise en cache — RFC 6749 §5.1.
///
/// L'en-tête tient au type et non aux deux handlers : un troisième, ajouté ici plus tard,
/// rendrait sinon la paire sans lui, et rien ne le signalerait.
impl IntoResponse for TokenPair {
    fn into_response(self) -> Response {
        (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
            ],
            Json(self),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}
