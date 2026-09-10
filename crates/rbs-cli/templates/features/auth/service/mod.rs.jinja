//! La couche qui porte les règles, un fichier par parcours.
//!
//! `issue()` et `profile()` vivent ici plutôt que dans l'un des parcours : les trois s'en
//! servent, et les descendre dans l'un d'eux ferait dépendre les deux autres de ce
//! voisin-là.

use chrono::{Duration, Utc};
use rbs_core::Result;
use rbs_core::config::AuthConfig;
use rbs_core::jwt::Claims;
use rbs_core::{jwt, token};
use sea_orm::ActiveEnum;
use sea_orm::DatabaseConnection;

use super::dto::{TokenPair, UserResponse};
use super::repository::{self, Model};

pub mod session;

pub use session::{login, logout, me, refresh, register};

/// Signe un jeton d'accès et ouvre la session de rafraîchissement qui l'accompagne.
async fn issue(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    utilisateur: &Model,
) -> Result<TokenPair> {
    let maintenant = Utc::now();

    let claims = Claims {
        sub: utilisateur.id.to_string(),
        role: utilisateur.role.clone().to_value(),
        exp: (maintenant + Duration::seconds(auth.access_ttl_secs as i64)).timestamp(),
        iat: maintenant.timestamp(),
        // Un jeton opaque fait un identifiant de jeton aussi bon qu'un UUID, sans réclamer
        // au projet le générateur qu'il n'embarque pas.
        jti: token::random(),
    };

    let access_token = jwt::sign(&claims, &auth.secret)?;

    let refresh_token = token::random();
    repository::create_refresh_token(
        db,
        utilisateur.id,
        token::fingerprint(&refresh_token),
        (maintenant + Duration::seconds(auth.refresh_ttl_secs as i64)).fixed_offset(),
    )
    .await?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_owned(),
        expires_in: auth.access_ttl_secs,
    })
}

/// La vue publique d'un utilisateur.
///
/// Le hash n'a aucun chemin vers le client : `UserResponse` ne porte pas le champ, et
/// cette fonction est le seul passage du modèle vers la réponse.
fn profile(utilisateur: Model) -> UserResponse {
    UserResponse {
        id: utilisateur.id,
        email: utilisateur.email,
        role: utilisateur.role.to_value(),
        created_at: utilisateur.created_at,
    }
}
