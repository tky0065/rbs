use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rbs_core::{Error, HasCoreState, Identity, Result};
use sea_orm::ActiveEnum;

use super::model::Role;
use super::repository;
use crate::state::AppState;

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

/// Une identité dont l'adresse est prouvée.
///
/// Sous le défaut `login_requires_verification = true`, seul un compte vérifié se
/// connecte : la garde sert le projet qui a mis la clé à `false` pour connecter dès
/// l'inscription, et décide alors des routes où la vérification devient obligatoire.
///
/// L'état est relu en base et non lu dans le jeton : le jeton d'accès porte `sub` et
/// `role`, et y mettre la vérification la figerait pour sa durée — une adresse tout juste
/// vérifiée resterait non vérifiée un quart d'heure.
///
/// ```ignore
/// pub async fn publier(identite: VerifiedIdentity, ...) -> Result<StatusCode> { ... }
/// ```
// Aucune route du fragment ne la porte, et un binaire n'exporte rien qui la tiendrait en
// vie : sans cette ligne, un projet portant `auth` ne compilerait pas sous
// `clippy -D warnings`. Elle se retire dès la première route qui l'emploie.
#[allow(dead_code)]
pub struct VerifiedIdentity(pub Identity);

/// Le compte qu'`accept_in` a relu pour juger le jeton, laissé dans la requête.
///
/// Un type propre au fragment plutôt que le `Model` nu : seule l'acceptation peut l'y avoir
/// mis.
#[derive(Clone)]
pub(super) struct Accepted(pub(super) repository::Model);

impl FromRequestParts<AppState> for VerifiedIdentity {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        // `Identity` rejette déjà une requête sans jeton : la garde ne répond donc jamais
        // à qui n'est pas identifié.
        let identite = Identity::from_request_parts(parts, state).await?;

        // `accept_in` vient de relire le compte pour juger le jeton, et l'a laissé là.
        let utilisateur = match parts.extensions.remove::<Accepted>() {
            Some(Accepted(compte)) => compte,
            // Un `accept_in` réécrit qui ne le dépose plus : relire plutôt que laisser
            // passer. Un compte disparu ne vaut pas mieux qu'un jeton invalide —
            // `Forbidden` laisserait entendre qu'il existe.
            None => repository::find(state.core().db(), identite.user_uuid()?)
                .await?
                .ok_or(Error::Unauthorized)?,
        };

        if utilisateur.email_verified_at.is_none() {
            return Err(Error::Forbidden);
        }

        Ok(Self(identite))
    }
}
