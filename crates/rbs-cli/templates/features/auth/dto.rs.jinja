use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use rbs_core::{Comparison, Sort, TextMatch};
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

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ChangePasswordRequest {
    // La même borne haute que sur `login`, et pour la même raison : sans elle, la route
    // hache en Argon2 tout ce qu'on lui poste.
    #[validate(length(min = 12, max = 128))]
    pub current_password: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

/// Ce que postent `forgot-password`, `resend-verification` et `PATCH /auth/me`.
///
/// Une seule structure pour les trois : elles prennent la même chose, et trois structures
/// identiques divergeraient un jour sans raison. C'est aussi ce qui tient la règle que la
/// dernière porte — `PATCH /auth/me` n'accepte *que* l'adresse : un champ ajouté ici
/// serait aussitôt visible dans les deux autres, et ne passerait pas inaperçu.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct EmailRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct TokenRequest {
    pub token: String,
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
    /// Nul tant que l'adresse n'est pas prouvée.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

/// Ce que rend `GET /auth/registration`.
///
/// Le seul moyen qu'a une application servie en fichiers statiques de connaître un
/// réglage que le serveur lit à son démarrage : sans cette route, l'écran d'inscription
/// ne saurait pas qu'il est fermé, et le visiteur ne l'apprendrait qu'en postant.
#[derive(Debug, Serialize, ToSchema)]
pub struct RegistrationStatus {
    pub enabled: bool,
}

/// La vue publique d'une session.
///
/// Jamais `token_hash` : la vue d'une session n'a aucune raison de porter de quoi la
/// présenter.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    pub id: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: DateTimeWithTimeZone,
}

/// Ce que `POST /users/filter` rend d'un compte : de quoi le choisir et le nommer, rien de
/// plus — ni le rôle ni les dates ne servent à une liste de sélection.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
}

/// Les conditions de `POST /users/filter`, écrites comme celles d'un filtre engendré.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UserFilter {
    #[schema(value_type = Option<rbs_core::UuidComparisonSchema>)]
    pub id: Option<Comparison<Uuid>>,
    #[schema(value_type = Option<rbs_core::TextMatchSchema>)]
    pub email: Option<TextMatch>,
    /// Colonnes de tri, préfixées de `-` pour l'ordre décroissant : `email`, `created_at`.
    #[schema(value_type = Option<Vec<String>>)]
    pub sort: Option<Sort>,
}
