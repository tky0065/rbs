//! Signature et vérification des jetons d'accès.
//!
//! HS256 : le secret signe et vérifie. Un algorithme asymétrique n'a d'intérêt que
//! lorsqu'un tiers vérifie sans pouvoir signer, ce qu'un monolithe généré ne fait pas.

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Charge utile d'un jeton d'accès.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// Identifiant de l'utilisateur.
    pub sub: String,
    /// Rôle en clair : l'enum `Role` est généré dans le projet, invisible au noyau.
    pub role: String,
    /// Expiration, en secondes depuis l'époque Unix.
    pub exp: i64,
    /// Émission, en secondes depuis l'époque Unix.
    pub iat: i64,
    /// Identifiant du jeton.
    pub jti: String,
}

/// Échec de vérification d'un jeton.
///
/// Le serveur répond 401 dans les trois cas ; le distinguo sert au client, à qui
/// « ton jeton a expiré, rafraîchis-le » indique quoi faire.
#[derive(Debug, thiserror::Error)]
pub enum ErreurJwt {
    /// La date d'expiration du jeton est passée.
    #[error("jeton expiré")]
    Expire,
    /// La signature ne correspond pas au secret.
    #[error("signature invalide")]
    Signature,
    /// Structure, algorithme ou claims illisibles.
    #[error("jeton malformé : {0}")]
    Malforme(String),
}

// Aucune des trois causes ne se distingue dans la réponse : divulguer laquelle a joué
// renseignerait un attaquant sur l'état de son jeton.
impl From<ErreurJwt> for Error {
    fn from(_: ErreurJwt) -> Self {
        Self::Unauthorized
    }
}

/// Signe `claims` en HS256 avec `secret`.
///
/// # Erreurs
///
/// Échoue si les claims ne peuvent pas être sérialisés ou le secret exploité.
pub fn signer(claims: &Claims, secret: &str) -> crate::Result<String> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|erreur| Error::Internal(anyhow::anyhow!("signature du jeton : {erreur}")))
}

/// Vérifie `jeton` avec `secret` et rend ses claims.
///
/// L'algorithme attendu est imposé ici et non lu dans l'en-tête du jeton : un
/// vérificateur qui fait confiance à l'en-tête accepte un jeton signé autrement.
///
/// # Erreurs
///
/// Rend [`ErreurJwt::Expire`], [`ErreurJwt::Signature`] ou [`ErreurJwt::Malforme`].
pub fn verifier(jeton: &str, secret: &str) -> Result<Claims, ErreurJwt> {
    decode::<Claims>(
        jeton,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|jeton| jeton.claims)
    .map_err(|erreur| match erreur.kind() {
        ErrorKind::ExpiredSignature => ErreurJwt::Expire,
        ErrorKind::InvalidSignature => ErreurJwt::Signature,
        autre => ErreurJwt::Malforme(format!("{autre:?}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "un secret de test qui porte au moins trente-deux octets";

    /// Un jeu de claims complet, dont seule l'expiration varie d'un test à l'autre.
    fn claims(exp: i64) -> Claims {
        Claims {
            sub: "u1".to_owned(),
            role: "user".to_owned(),
            exp,
            iat: 0,
            jti: "j1".to_owned(),
        }
    }

    /// Expiration lointaine, pour les cas où la validité temporelle n'est pas le sujet.
    const PLUS_TARD: i64 = 4_102_444_800;

    #[test]
    fn signer_puis_verifier_restitue_les_claims() {
        let attendu = claims(PLUS_TARD);

        let jeton = signer(&attendu, SECRET).expect("signature");

        assert_eq!(verifier(&jeton, SECRET).expect("vérification"), attendu);
    }

    #[test]
    fn un_jeton_expire_rend_une_erreur_distincte_de_la_signature() {
        let jeton = signer(&claims(0), SECRET).expect("signature");

        assert!(matches!(verifier(&jeton, SECRET), Err(ErreurJwt::Expire),));
    }

    #[test]
    fn une_signature_invalide_est_rejetee() {
        let jeton = signer(&claims(PLUS_TARD), SECRET).expect("signature");

        assert!(matches!(
            verifier(&jeton, "un autre secret tout aussi long ici"),
            Err(ErreurJwt::Signature),
        ));
    }

    #[test]
    fn un_jeton_alg_none_est_rejete() {
        let jeton = signer(&claims(PLUS_TARD), SECRET).expect("signature");
        let charge = jeton.split('.').nth(1).expect("charge utile");
        // `{"alg":"none","typ":"JWT"}` en base64url, suivi d'une signature vide.
        let forge = format!("eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.{charge}.");

        assert!(
            verifier(&forge, SECRET).is_err(),
            "un jeton non signé ne doit jamais être accepté"
        );
    }

    // `alg: none` ne peut pas franchir la désérialisation de l'en-tête : `Algorithm` n'a
    // pas de variante `None`. La confusion d'algorithme réellement atteignable est
    // celle-ci — un algorithme supporté, mais autre que celui attendu.
    #[test]
    fn un_jeton_signe_avec_un_autre_algorithme_est_rejete() {
        let jeton = encode(
            &Header::new(Algorithm::HS512),
            &claims(PLUS_TARD),
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("signature HS512");

        assert!(
            verifier(&jeton, SECRET).is_err(),
            "l'algorithme attendu doit être imposé, pas lu dans l'en-tête"
        );
    }
}
