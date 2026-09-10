use rbs_core::config::AuthConfig;
use rbs_core::{Error, Result, hash};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, TokenPair};
use super::super::repository;
use super::issue;

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

    let fermees = repository::revoke_sessions_of(db, user_id).await?;

    // Ni l'adresse ni les jetons : l'identifiant du compte suffit à retrouver ce qui s'est
    // passé, et le journal ne porte pas ce que la réponse tait.
    tracing::info!(
        user_id = %user_id,
        sessions_revoquees = fermees,
        "mot de passe changé : les sessions du compte sont révoquées"
    );

    issue(db, auth, &utilisateur).await
}
