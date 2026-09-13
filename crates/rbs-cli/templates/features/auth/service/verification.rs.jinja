use chrono::{Duration, Utc};
use rbs_core::{Error, Result, token};
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::super::model::TokenPurpose;
use super::super::repository;
use super::normalise;

/// Ouvre un jeton de vérification, et rend le compte avec le jeton **en clair**.
///
/// Une seule fonction pour l'inscription et pour le renvoi : les deux partent d'une
/// adresse et rendent la même chose. En écrire une seconde qui prendrait le modèle ferait
/// appeler le repository depuis le contrôleur de `register`, et la dépendance des couches
/// ne le permet pas.
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
