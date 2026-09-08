use rbs_core::{Error, Identity, Result};
use sea_orm::ActiveEnum;

use super::model::Role;

/// Exige un rôle **au moins** égal à celui donné.
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
// Le fragment seul n'appelle pas cette garde, et un binaire n'exporte rien qui la
// tiendrait en vie : sans cette ligne, un projet portant `auth` et aucun CRUD ne
// compilerait pas sous `clippy -D warnings`. Un CRUD engendré sous `auth` l'appelle, lui —
// dès votre première feature cette ligne ne masque donc plus rien, et se retire.
#[allow(dead_code)]
// region: require_role
pub trait RequireRole {
    /// Rend [`Error::Forbidden`] si l'appelant porte un rôle inférieur à `minimum`.
    fn require_role(&self, minimum: Role) -> Result<()>;
}

impl RequireRole for Identity {
    fn require_role(&self, minimum: Role) -> Result<()> {
        // Un rôle que l'enum ne connaît plus vient d'un jeton signé par une version
        // antérieure du projet : il n'ouvre rien, et ne fait pas tomber le serveur.
        let porte = Role::try_from_value(&self.role).map_err(|_| Error::Forbidden)?;

        // Le seuil, et non l'égalité : un Admin satisfait une exigence User. C'est
        // l'ordre de déclaration de l'enum qui range les variantes.
        if porte >= minimum {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}
// endregion: require_role
