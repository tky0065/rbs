use axum::extract::State;
use axum::http::StatusCode;
use rbs_core::{HasAuth, HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, EmailRequest, ResetPasswordRequest, TokenPair};
use super::super::service;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/auth/change-password",
    tag = "auth",
    operation_id = "auth_change_password",
    security(("bearer" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "mot de passe changé, paire neuve", body = TokenPair),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "mot de passe courant refusé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<ChangePasswordRequest>,
) -> Result<TokenPair> {
    // `sub` porte l'identifiant sous forme de chaîne : le jeton est signé, mais rien ne
    // garantit que celui-ci a été émis par une version du service qui y mettait un UUID.
    let id = Uuid::parse_str(&identite.user_id).map_err(|_| rbs_core::Error::Unauthorized)?;

    service::password::change(state.core().db(), state.auth(), id, input).await
}

// region: forgot_password
// L'envoi part détaché, et le statut est 202 quoi qu'il arrive. Attendre le SMTP
// rendrait par le temps de réponse ce que le code de statut refuse de dire : quelques
// centaines de millisecondes séparent une adresse inscrite d'une adresse inconnue.
#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = "auth",
    operation_id = "auth_forgot_password",
    request_body = EmailRequest,
    responses(
        (status = 202, description = "demande acceptée, que l'adresse soit inscrite ou non"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<EmailRequest>,
) -> Result<StatusCode> {
    let flows = state.flows();

    if let Some((utilisateur, jeton)) =
        service::password::request_reset(state.core().db(), flows.reset_ttl_secs, &input.email)
            .await?
    {
        // `send_template_detached` rend le gabarit sur-le-champ et peut donc échouer —
        // gabarit absent, mal formé, ou adresse que `lettre` refuse d'analyser. Propager
        // cette erreur ferait de cette branche, atteinte seulement quand le compte
        // existe, la seule à répondre 500 : un attaquant qui essaie plusieurs adresses
        // verrait alors le code de statut lui dire lesquelles sont inscrites, ce que 202
        // existe précisément pour taire.
        if let Err(error) = state.mail().send_template_detached(
            &utilisateur.email,
            "Réinitialisation de votre mot de passe",
            "reinitialisation.html",
            minijinja::context! {
                link => flows.link("reset-password", &jeton),
                heures => flows.reset_ttl_secs / 3600,
            },
        ) {
            tracing::error!(
                user_id = %utilisateur.id,
                %error,
                "envoi du courriel de réinitialisation échoué"
            );
        }
    }

    Ok(StatusCode::ACCEPTED)
}
// endregion: forgot_password

// region: reset_password
#[utoipa::path(
    post,
    path = "/auth/reset-password",
    tag = "auth",
    operation_id = "auth_reset_password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "mot de passe réinitialisé, sessions révoquées"),
        (status = 401, description = "jeton inconnu, périmé ou déjà consommé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<ResetPasswordRequest>,
) -> Result<StatusCode> {
    service::password::reset(state.core().db(), input).await?;

    Ok(StatusCode::NO_CONTENT)
}
// endregion: reset_password
