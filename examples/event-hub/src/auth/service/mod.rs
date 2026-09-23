//! La couche qui porte les règles, un fichier par parcours.
//!
//! `issue()`, `notify()`, `detach()` et `warn_taken()` vivent ici plutôt que dans l'un
//! des parcours : plusieurs s'en servent, et les descendre dans l'un d'eux ferait dépendre
//! les autres de ce voisin-là.
//! `profile()` et `session_view()` y sont aussi : ce sont les deux seuls passages du
//! modèle vers la réponse, et les garder côte à côte tient cette règle en un endroit.

use chrono::{Duration, Utc};
use rbs_core::Result;
use rbs_core::config::AuthConfig;
use rbs_core::jwt::Claims;
use rbs_core::{jwt, token};
use sea_orm::ActiveEnum;
use sea_orm::ConnectionTrait;
use sea_orm::prelude::Uuid;
use serde::Serialize;

use super::config::FlowConfig;
use super::dto::{SessionResponse, TokenPair, UserResponse};
use super::repository::{self, Model};
use crate::modules::mail::Mailer;

pub mod account;
pub mod password;
pub mod session;
pub mod verification;

pub use account::{change_email, filter_users};
pub use session::{
    login, logout, me, refresh, register, revoke_session, revoke_sessions, sessions,
};

/// Signe un jeton d'accès et ouvre la session de rafraîchissement qui l'accompagne.
async fn issue(
    db: &impl ConnectionTrait,
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
pub(super) async fn close_every_session(db: &impl ConnectionTrait, user_id: Uuid) -> Result<u64> {
    let fermees = repository::revoke_sessions_of(db, user_id).await?;
    repository::user::stamp_sessions_revoked(db, user_id).await?;

    Ok(fermees)
}

/// L'adresse telle que la base la voit.
///
/// Minuscules et sans blancs : la partie locale est théoriquement sensible à la casse
/// (RFC 5321 §2.4), aucun fournisseur ne l'honore, et c'est l'attaquant qui en
/// profiterait — deux comptes pour une boîte, dont un vérifié par l'autre.
pub(super) fn normalise(email: &str) -> String {
    email.trim().to_lowercase()
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

/// La vue publique d'une session.
///
/// Même règle que `profile()` pour `UserResponse` : `SessionResponse` ne porte pas
/// `token_hash`, et cette fonction est le seul passage du modèle vers la réponse.
fn session_view(session: repository::refresh_token::Model) -> SessionResponse {
    SessionResponse {
        id: session.id,
        created_at: session.created_at,
        expires_at: session.expires_at,
    }
}

/// Envoie un courriel à un compte, sans que son échec atteigne la réponse.
fn notify(
    mail: &Mailer,
    destinataire: &Model,
    objet: &str,
    gabarit: &str,
    contexte: impl Serialize,
) {
    // `send_template_detached` rend le gabarit sur-le-champ et peut donc échouer —
    // gabarit absent, mal formé, ou adresse que `lettre` refuse d'analyser. On n'arrive
    // ici que depuis une tâche détachée, la réponse déjà partie : l'échec n'a plus
    // d'appelant où remonter, et reste au journal. `forgot-password` et
    // `resend-verification` sont le rattrapage d'un courriel qui ne part pas.
    if let Err(error) = mail.send_template_detached(&destinataire.email, objet, gabarit, contexte) {
        tracing::error!(
            user_id = %destinataire.id,
            gabarit,
            %error,
            "préparation du courriel échouée"
        );
    }
}

/// Prévient le titulaire d'une adresse qu'on a tenté de la lui prendre.
///
/// Détaché comme le lien d'une adresse neuve : les deux branches de `register`, comme
/// celles de `change_email`, répondent dans le même temps.
pub(super) fn warn_taken(mail: &Mailer, flows: &FlowConfig, titulaire: Model) {
    let (mail, flows) = (mail.clone(), flows.clone());
    detach(titulaire.id, "inscription", async move {
        notify(
            &mail,
            &titulaire,
            "Tentative d'inscription avec votre adresse",
            "inscription.html",
            minijinja::context! { forgot_url => flows.page("forgot-password") },
        );

        Ok(())
    });
}

/// Lance `travail` sans l'attendre ; son échec va au journal, avec le compte.
///
/// Ce qu'une route publique fait pour un compte existant — écrire un jeton, rendre un
/// courriel — prend un temps qu'une adresse inconnue ne prend pas : attendu, cet écart
/// dirait lesquelles sont inscrites.
fn detach(
    user_id: Uuid,
    parcours: &'static str,
    travail: impl Future<Output = Result<()>> + Send + 'static,
) {
    tokio::spawn(async move {
        if let Err(error) = travail.await {
            tracing::error!(%user_id, parcours, %error, "émission détachée échouée");
        }
    });
}
