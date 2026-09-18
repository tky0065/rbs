use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use sea_orm::ActiveEnum;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateApiKey {
    /// Ce que la clé sert. Sans lui, une liste de clés est illisible au bout de trois.
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    /// `user` par défaut : une clé qui administre se demande, elle ne s'obtient pas par
    /// omission. Refusé au-delà du rôle du créateur.
    pub role: Option<String>,
    /// Sans échéance, la clé ne périme pas — c'est un choix, pas un oubli.
    ///
    /// Bornée des deux côtés : à zéro la clé naîtrait déjà périmée, silencieusement
    /// inutile ; au-delà de dix ans, le calcul d'échéance sort de ce que `chrono` sait
    /// représenter et l'addition panique, ce qu'un appelant authentifié ne doit pas
    /// pouvoir provoquer avec un seul champ.
    #[validate(range(min = 1, max = 3650))]
    pub expires_in_days: Option<u32>,
}

/// Ce que rend la création, et cette seule fois.
///
/// `key` n'y paraît qu'ici : une seule lecture de la liste livrerait sinon toutes les clés
/// du compte d'un coup.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyCreated {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub role: String,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// La clé en clair. Elle n'existe nulle part ailleurs : ni la base, ni cette API ne
    /// sauront la redonner.
    pub key: String,
}

/// La création porte sa propre réponse pour n'être jamais mise en cache.
///
/// L'en-tête tient au type et non au handler : un second handler qui rendrait ce corps le
/// rendrait sinon sans lui, et rien ne le signalerait.
impl IntoResponse for ApiKeyCreated {
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

/// La vue publique d'une clé. Jamais `token_hash`, jamais la clé.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub role: String,
    /// Nul : cette clé n'a jamais servi. C'est ce qui permet de la révoquer sans crainte.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

impl From<super::model::Model> for ApiKeyResponse {
    fn from(cle: super::model::Model) -> Self {
        Self {
            id: cle.id,
            name: cle.name,
            prefix: cle.prefix,
            role: cle.role.to_value(),
            last_used_at: cle.last_used_at,
            expires_at: cle.expires_at,
            revoked_at: cle.revoked_at,
            created_at: cle.created_at,
        }
    }
}
