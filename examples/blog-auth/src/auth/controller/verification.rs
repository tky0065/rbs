use axum::extract::State;
use axum::http::StatusCode;
use rbs_core::{HasCoreState, ProblemDetails, Result, ValidatedJson};

use super::super::dto::{EmailRequest, TokenRequest};
use super::super::service;
use crate::state::AppState;

// region: resend_verification
// L'envoi part détaché, et le statut est 202 quoi qu'il arrive — même raison qu'à
// `/auth/forgot-password` : attendre le SMTP dirait par le temps de réponse ce que le
// code de statut refuse de dire.
#[utoipa::path(
    post,
    path = "/auth/resend-verification",
    tag = "auth",
    operation_id = "auth_resend_verification",
    request_body = EmailRequest,
    responses(
        (status = 202, description = "demande acceptée, que l'adresse soit inscrite ou non"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn resend_verification(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<EmailRequest>,
) -> Result<StatusCode> {
    let flows = state.flows();

    if let Some((utilisateur, jeton)) =
        service::verification::request(state.core().db(), flows.verification_ttl_secs, &input.email)
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
            "Confirmez votre adresse",
            "verification.html",
            minijinja::context! { link => flows.link("verify-email", &jeton) },
        ) {
            tracing::error!(
                user_id = %utilisateur.id,
                %error,
                "envoi du courriel de vérification échoué"
            );
        }
    }

    Ok(StatusCode::ACCEPTED)
}
// endregion: resend_verification

// region: verify_email
#[utoipa::path(
    post,
    path = "/auth/verify-email",
    tag = "auth",
    operation_id = "auth_verify_email",
    request_body = TokenRequest,
    responses(
        (status = 204, description = "adresse vérifiée"),
        (status = 401, description = "jeton inconnu, périmé, déjà consommé ou émis pour un autre usage", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn verify_email(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<TokenRequest>,
) -> Result<StatusCode> {
    service::verification::verify(state.core().db(), &input.token).await?;

    Ok(StatusCode::NO_CONTENT)
}
// endregion: verify_email
