use chrono::{Duration, Utc};
use rbs_core::{Error, Result, token};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use super::super::config::FlowConfig;
use super::super::model::TokenPurpose;
use super::super::repository;
use super::{detach, normalise, notify};
use crate::modules::mail::Mailer;

/// Ouvre un jeton de vérification pour un compte, et le rend **en clair**.
///
/// Le jeton sort en clair pour le courriel de `send_link_detached`, et pour les tests du
/// projet, qui déroulent le parcours sans qu'aucun SMTP soit joignable. Les jetons échus
/// de tous les comptes partent d'abord, pour la raison que donne `password::open_reset`.
pub async fn open(
    db: &impl ConnectionTrait,
    ttl_secs: u64,
    utilisateur: &repository::Model,
) -> Result<String> {
    repository::one_time_token::purge_expired(db).await?;
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

    Ok(jeton)
}

/// Le compte qui porte l'adresse, s'il attend encore sa vérification.
async fn awaiting_proof(db: &DatabaseConnection, email: &str) -> Result<Option<repository::Model>> {
    let compte = repository::find_by_email(db, &normalise(email)).await?;

    // La date sert un jour à redemander une preuve aux plus anciennes adresses : une
    // seconde vérification la rajeunirait, et la fausserait.
    Ok(compte.filter(|compte| compte.email_verified_at.is_none()))
}

/// `open` pour le compte qui porte l'adresse, rendu avec le jeton.
///
/// `None` quand aucun compte ne porte l'adresse, ou qu'elle est déjà vérifiée.
pub async fn request(
    db: &DatabaseConnection,
    ttl_secs: u64,
    email: &str,
) -> Result<Option<(repository::Model, String)>> {
    let Some(utilisateur) = awaiting_proof(db, email).await? else {
        return Ok(None);
    };

    let jeton = open(db, ttl_secs, &utilisateur).await?;

    Ok(Some((utilisateur, jeton)))
}

// region: send_link
/// Envoie un lien de vérification neuf, si un compte non vérifié porte l'adresse.
///
/// Une adresse inconnue ou déjà vérifiée ne reçoit rien, et l'appelant n'en sait rien.
pub async fn send_link(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    email: &str,
) -> Result<()> {
    if let Some(utilisateur) = awaiting_proof(db, email).await? {
        send_link_detached(db, mail, flows, utilisateur);
    }

    Ok(())
}

/// Émet un jeton de vérification pour un compte et lui en envoie le lien, sans attendre.
///
/// Une seule fonction pour l'inscription et pour le renvoi : les deux ont le compte en
/// main au moment d'émettre, et aucune des deux ne doit dire par son temps de réponse ce
/// que cette émission lui a coûté.
pub fn send_link_detached(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    utilisateur: repository::Model,
) {
    let (db, mail, flows) = (db.clone(), mail.clone(), flows.clone());
    detach(utilisateur.id, "vérification", async move {
        let jeton = open(&db, flows.verification_ttl_secs, &utilisateur).await?;
        notify(
            &mail,
            &utilisateur,
            "Confirmez votre adresse",
            "verification.html",
            minijinja::context! { link => flows.link("verify-email", &jeton) },
        );

        Ok(())
    });
}
// endregion: send_link

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
