pub mod config;
pub mod controller;
pub mod dto;
pub mod guard;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::http::Extensions;
use axum::routing::{delete, get, post};
use rbs_core::jwt::Claims;
use rbs_core::{Error, HasAuth, HasCoreState};
use sea_orm::ActiveEnum;

use crate::state::AppState;

// `HasAuth` n'est pas implémenté pour tout état : c'est ce qui laisse un projet tirer son
// secret d'ailleurs qu'un fichier de configuration. L'implémentation vit ici plutôt que
// dans `state.rs` — elle arrive avec la feature, et repart avec elle.
impl HasAuth for AppState {
    async fn accept(&self, claims: &Claims) -> rbs_core::Result<()> {
        admit(self, claims).await.map(drop)
    }

    /// Comme `accept`, et laisse dans la requête la date de vérification du compte relu :
    /// une garde qui suit `Identity`, comme `VerifiedIdentity`, la reprend au lieu de
    /// relire la même ligne.
    async fn accept_in(
        &self,
        claims: &Claims,
        extensions: &mut Extensions,
    ) -> rbs_core::Result<()> {
        let compte = admit(self, claims).await?;
        extensions.insert(guard::Accepted(compte.email_verified_at));
        Ok(())
    }

    // <rbs:auth_impl>
    // </rbs:auth_impl>
}

/// Ce que la signature ne dit pas : le compte existe-t-il encore, a-t-il fermé ses
/// sessions depuis l'émission, porte-t-il toujours ce rôle. Rend le compte lu.
///
/// Une lecture de `users` par requête authentifiée — le prix d'un jeton d'accès qui
/// meurt avec les sessions au lieu de survivre `access_ttl_secs`. Fermer une seule
/// session nommée ne passe pas ici : rien ne relie un jeton d'accès à sa ligne.
async fn admit(state: &AppState, claims: &Claims) -> rbs_core::Result<repository::Model> {
    let id = claims.user_uuid()?;
    let compte = repository::find(state.core().db(), id)
        .await?
        .ok_or(Error::Unauthorized)?;

    // La seconde de la révocation comprise : `iat` n'a pas mieux, et `issue` n'émet
    // jamais dedans.
    if compte
        .sessions_revoked_at
        .is_some_and(|estampille| claims.iat <= estampille.timestamp())
    {
        return Err(Error::Unauthorized);
    }

    // Le client rafraîchit, et repart avec le rôle courant.
    if compte.role.to_value() != claims.role {
        return Err(Error::Unauthorized);
    }

    Ok(compte)
}

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart avec
// elle.
impl AppState {
    /// Les réglages des parcours de réinitialisation et de vérification.
    pub fn flows(&self) -> &config::FlowConfig {
        &self.flows
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(controller::register))
        .route("/auth/registration", get(controller::registration_status))
        .route("/auth/login", post(controller::login))
        .route("/auth/refresh", post(controller::refresh))
        .route("/auth/logout", post(controller::logout))
        // Les deux méthodes du même chemin en une fois, comme `/auth/sessions` plus bas :
        // axum refuse deux `route()` sur un chemin identique, et le dit par une panique au
        // démarrage.
        .route(
            "/auth/me",
            get(controller::me).patch(controller::account::update_me),
        )
        .route(
            "/auth/change-password",
            post(controller::password::change_password),
        )
        .route(
            "/auth/forgot-password",
            post(controller::password::forgot_password),
        )
        .route(
            "/auth/reset-password",
            post(controller::password::reset_password),
        )
        .route(
            "/auth/verify-email",
            post(controller::verification::verify_email),
        )
        .route(
            "/auth/resend-verification",
            post(controller::verification::resend_verification),
        )
        // Les deux méthodes du même chemin se déclarent en une fois : axum refuse — et le
        // dit par une panique au démarrage — deux `route()` sur un chemin identique.
        .route(
            "/auth/sessions",
            get(controller::list_sessions).delete(controller::revoke_sessions),
        )
        .route("/auth/sessions/{id}", delete(controller::revoke_session))
        .route("/users/filter", post(controller::account::filter_users))
}
