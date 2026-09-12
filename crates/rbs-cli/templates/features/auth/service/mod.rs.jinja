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
use sea_orm::prelude::Uuid;

use super::dto::{TokenPair, UserResponse};
use super::repository::{self, Model};

pub mod password;
pub mod session;
pub mod verification;

pub use session::{
    login, logout, me, refresh, register, revoke_session, revoke_sessions, sessions,
};

/// Signe un jeton d'accès et ouvre la session de rafraîchissement qui l'accompagne.
async fn issue(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    utilisateur: &Model,
) -> Result<TokenPair> {
    let maintenant = Utc::now();

    // Jamais dans la seconde d'une révocation : `iat` n'a pas mieux que la seconde, et
    // un jeton émis dans celle-là serait indiscernable de ceux qu'elle a tués — la paire
    // que `change-password` rend naîtrait morte.
    let plancher = utilisateur
        .sessions_revoked_at
        .map_or(i64::MIN, |estampille| estampille.timestamp() + 1);
    let iat = maintenant.timestamp().max(plancher);

    let claims = Claims {
        sub: utilisateur.id.to_string(),
        role: utilisateur.role.clone().to_value(),
        exp: iat + auth.access_ttl_secs as i64,
        iat,
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

/// Ferme toutes les sessions d'un compte, jetons d'accès compris.
///
/// Deux écritures, une fonction : les quatre chemins qui ferment un compte — rejeu,
/// changement, réinitialisation, `DELETE /auth/sessions` — passent ici, et aucun ne peut
/// fermer les rafraîchissements en laissant vivre les accès.
pub(super) async fn close_every_session(db: &DatabaseConnection, user_id: Uuid) -> Result<u64> {
    let fermees = repository::revoke_sessions_of(db, user_id).await?;
    repository::user::stamp_sessions_revoked(db, user_id).await?;

    Ok(fermees)
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
        email_verified_at: utilisateur.email_verified_at,
        created_at: utilisateur.created_at,
    }
}
