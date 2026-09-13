use chrono::{Duration, Utc};
use rbs_core::{Error, Result, token};
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::super::config::FlowConfig;
use super::super::model::TokenPurpose;
use super::super::repository;
use super::{normalise, notify};
use crate::modules::mail::Mailer;

/// Ouvre un jeton de vérification, et rend le compte avec le jeton **en clair**.
///
/// Le jeton sort en clair pour `send_link`, qui le met dans un courriel, et pour les tests
/// du projet, qui déroulent le parcours sans qu'aucun SMTP soit joignable.
pub async fn request(
    db: &DatabaseConnection,
    ttl_secs: u64,
    email: &str,
) -> Result<Option<(repository::Model, String)>> {
    let Some(utilisateur) = repository::find_by_email(db, &normalise(email)).await? else {
        return Ok(None);
    };

    repository::one_time_token::invalidate_pending(
        db,
        utilisateur.id,
        TokenPurpose::EmailVerification,
    )
    .await?;

    let jeton = token::random();
    repository::one_time_token::issue(
        db,
        utilisateur.id,
        TokenPurpose::EmailVerification,
        token::fingerprint(&jeton),
        (Utc::now() + Duration::seconds(ttl_secs as i64)).fixed_offset(),
    )
    .await?;

    Ok(Some((utilisateur, jeton)))
}

/// Envoie un lien de vérification neuf, si un compte porte l'adresse.
///
/// Une seule fonction pour l'inscription et pour le renvoi : les deux partent d'une
/// adresse. Une adresse inconnue ne reçoit rien, et l'appelant n'en sait rien.
pub async fn send_link(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    email: &str,
) -> Result<()> {
    if let Some((utilisateur, jeton)) = request(db, flows.verification_ttl_secs, email).await? {
        notify(
            mail,
            &utilisateur,
            "Confirmez votre adresse",
            "verification.html",
            minijinja::context! { link => flows.link("verify-email", &jeton) },
        );
    }

    Ok(())
}

/// Consomme un jeton et date la vérification.
///
/// Jeton inconnu, périmé, déjà consommé, ou émis pour un autre usage : la même erreur
/// pour les quatre. Les distinguer renseignerait sur l'état des demandes en cours.
pub async fn verify(db: &DatabaseConnection, token_clair: &str) -> Result<()> {
    let fingerprint = token::fingerprint(token_clair);

    let ligne = repository::one_time_token::find(db, &fingerprint, TokenPurpose::EmailVerification)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Tout ou rien : un jeton consommé sans que l'adresse soit datée serait brûlé pour
    // rien, et l'utilisateur devrait en redemander un.
    let transaction = db.begin().await?;

    if !repository::one_time_token::consume(&transaction, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    repository::user::mark_verified(&transaction, ligne.user_id).await?;
    transaction.commit().await?;

    Ok(())
}
