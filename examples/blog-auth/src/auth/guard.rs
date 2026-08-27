use rbs_core::{Error, Identity, Result};
use sea_orm::ActiveEnum;

use super::model::Role;

/// Exige un rôle de l'appelant.
///
/// `Identity` vient du noyau, qui ne connaît le rôle qu'en clair : l'enum `Role` vit ici,
/// dans le projet, et c'est ce trait qui les réunit. Un rôle de plus dans `model.rs` est
/// aussitôt utilisable par cette garde.
///
/// L'appel se fait en tête de handler, après l'extraction de `Identity` — laquelle rejette
/// déjà une requête sans jeton. La garde ne répond donc jamais à qui n'est pas identifié.
///
/// ```ignore
/// pub async fn supprimer(identite: Identity, ...) -> Result<StatusCode> {
///     identite.require_role(Role::Admin)?;
///     ...
/// }
/// ```
// region: require_role
pub trait RequireRole {
    /// Rend [`Error::Forbidden`] si l'appelant ne porte pas `attendu`.
    fn require_role(&self, attendu: Role) -> Result<()>;
}

impl RequireRole for Identity {
    fn require_role(&self, attendu: Role) -> Result<()> {
        // Un rôle que l'enum ne connaît plus vient d'un jeton signé par une version
        // antérieure du projet : il n'ouvre rien, et ne fait pas tomber le serveur.
        let porte = Role::try_from_value(&self.role).map_err(|_| Error::Forbidden)?;

        if porte == attendu {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}
// endregion: require_role
