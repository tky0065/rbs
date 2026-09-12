use chrono::{Duration, Utc};
use rbs_core::config::AuthConfig;
use rbs_core::{Error, Result, hash, token};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, ResetPasswordRequest, TokenPair};
use super::super::model::TokenPurpose;
use super::super::repository;
use super::{close_every_session, issue, normalise};

/// Change le mot de passe d'un compte identifié, et rend une paire neuve.
///
/// Toutes les sessions tombent, celle de l'appelant comprise : la requête porte un jeton
/// d'accès, et rien ne le relie à la ligne qui l'a émis — la session courante ne peut pas
/// être épargnée faute d'être identifiable. Plutôt que de déconnecter quelqu'un qui vient
/// de faire la bonne chose, on révoque puis on réémet.
pub async fn change(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    user_id: Uuid,
    input: ChangePasswordRequest,
) -> Result<TokenPair> {
    let utilisateur = repository::find(db, user_id)
        .await?
        // Un jeton valide dont le compte a disparu ne vaut pas mieux qu'un jeton invalide.
        .ok_or(Error::Unauthorized)?;

    // `Forbidden` et non `Unauthorized` : l'appelant est identifié, son Bearer est bon.
    // Un 401 lui dirait que son jeton est mort, et son client tenterait un
    // rafraîchissement qui ne réglerait rien.
    if !hash::verify_password(&input.current_password, &utilisateur.password_hash)? {
        return Err(Error::Forbidden);
    }

    let nouveau = hash::hash_password(&input.new_password)?;
    repository::user::set_password(db, user_id, &nouveau).await?;

    // Une boîte compromise a pu recevoir une demande de réinitialisation d'un attaquant
    // avant que son titulaire ne s'en aperçoive et ne change ici son mot de passe pour
    // reprendre la main : laisser ce lien-là vivant jusqu'à son terme le rendrait à
    // l'attaquant malgré le geste qui devait l'en priver. C'est le scénario que ce champ
    // existe déjà pour fermer à l'émission d'un nouveau jeton ; il vaut tout autant ici.
    repository::one_time_token::invalidate_pending(db, user_id, TokenPurpose::PasswordReset)
        .await?;

    let fermees = close_every_session(db, user_id).await?;

    // Rechargé : `issue` lit l'estampille que la fermeture vient de poser, et n'émettrait
    // sinon qu'un jeton né dans la seconde qu'elle vient de tuer.
    let utilisateur = repository::find(db, user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Ni l'adresse ni les jetons : l'identifiant du compte suffit à retrouver ce qui s'est
    // passé, et le journal ne porte pas ce que la réponse tait.
    tracing::info!(
        user_id = %user_id,
        sessions_revoquees = fermees,
        "mot de passe changé : les sessions du compte sont révoquées"
    );

    issue(db, auth, &utilisateur).await
}

/// Ouvre un jeton de réinitialisation, et rend le compte avec le jeton **en clair**.
///
/// Le jeton en clair ne se relit nulle part : la base n'en garde que l'empreinte. Le
/// rendre ici est ce qui permet au contrôleur de le mettre dans un courriel — et aux
/// tests du projet de dérouler le parcours entier sans qu'aucun SMTP soit joignable.
///
/// `None` quand aucun compte ne porte l'adresse. C'est l'appelant qui décide d'en tirer
/// une réponse indiscernable, et il le fait.
pub async fn request_reset(
    db: &DatabaseConnection,
    ttl_secs: u64,
    email: &str,
) -> Result<Option<(repository::Model, String)>> {
    let Some(utilisateur) = repository::find_by_email(db, &normalise(email)).await? else {
        return Ok(None);
    };

    repository::one_time_token::invalidate_pending(db, utilisateur.id, TokenPurpose::PasswordReset)
        .await?;

    let jeton = token::random();
    repository::one_time_token::issue(
        db,
        utilisateur.id,
        TokenPurpose::PasswordReset,
        token::fingerprint(&jeton),
        (Utc::now() + Duration::seconds(ttl_secs as i64)).fixed_offset(),
    )
    .await?;

    Ok(Some((utilisateur, jeton)))
}

/// Consomme un jeton et pose le nouveau mot de passe.
///
/// Toutes les sessions tombent : on ne sait pas si le compte avait été pris, et les
/// laisser tourner reviendrait à valider la prise.
pub async fn reset(db: &DatabaseConnection, input: ResetPasswordRequest) -> Result<()> {
    let fingerprint = token::fingerprint(&input.token);

    // Jeton inconnu, périmé ou déjà consommé : la même erreur pour les trois. Les
    // distinguer renseignerait sur l'état des demandes en cours.
    let ligne = repository::one_time_token::find(db, &fingerprint, TokenPurpose::PasswordReset)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Rien ici ne relit `consumed_at` ni `expires_at` : c'est `consume` qui porte les deux
    // conditions, et elle seule peut les porter sans laisser passer deux
    // réinitialisations concurrentes.
    if !repository::one_time_token::consume(db, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    let nouveau = hash::hash_password(&input.new_password)?;
    repository::user::set_password(db, ligne.user_id, &nouveau).await?;

    // Symétrique à `change` : un mot de passe qui vient d'être posé rend caducs les
    // autres liens de réinitialisation en attente, plutôt que d'en laisser un survivre à
    // côté du mot de passe qu'il prétendait remplacer.
    repository::one_time_token::invalidate_pending(db, ligne.user_id, TokenPurpose::PasswordReset)
        .await?;

    let fermees = close_every_session(db, ligne.user_id).await?;

    tracing::info!(
        user_id = %ligne.user_id,
        sessions_revoquees = fermees,
        "mot de passe réinitialisé : les sessions du compte sont révoquées"
    );

    Ok(())
}
