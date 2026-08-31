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
#[non_exhaustive]
pub enum JwtError {
    /// La date d'expiration du jeton est passée.
    #[error("token expiré")]
    Expired,
    /// La signature ne correspond pas au secret.
    #[error("signature invalide")]
    Signature,
    /// Structure, algorithme ou claims illisibles.
    #[error("token malformé : {0}")]
    Malformed(String),
}

// Aucune des trois causes ne se distingue dans la réponse : divulguer laquelle a joué
// renseignerait un attaquant sur l'état de son jeton.
impl From<JwtError> for Error {
    fn from(_: JwtError) -> Self {
        Self::Unauthorized
    }
}

/// Signe `claims` en HS256 avec `secret`.
///
/// # Erreurs
///
/// Échoue si les claims ne peuvent pas être sérialisés ou le secret exploité.
pub fn sign(claims: &Claims, secret: &str) -> crate::Result<String> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| Error::Internal(anyhow::anyhow!("signature du token : {error}")))
}

/// Vérifie `token` avec `secret` et rend ses claims.
///
/// L'algorithme attendu est imposé ici et non lu dans l'en-tête du jeton : un
/// vérificateur qui fait confiance à l'en-tête accepte un jeton signé autrement.
///
/// # Erreurs
///
/// Rend [`JwtError::Expired`], [`JwtError::Signature`] ou [`JwtError::Malformed`].
pub fn verify(token: &str, secret: &str) -> Result<Claims, JwtError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map(|token| token.claims)
    .map_err(|error| match error.kind() {
        ErrorKind::ExpiredSignature => JwtError::Expired,
        ErrorKind::InvalidSignature => JwtError::Signature,
        other => JwtError::Malformed(format!("{other:?}")),
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
    const LATER: i64 = 4_102_444_800;

    #[test]
    fn signing_then_verifying_restores_the_claims() {
        let expected = claims(LATER);

        let token = sign(&expected, SECRET).expect("signature");

        assert_eq!(verify(&token, SECRET).expect("vérification"), expected);
    }

    #[test]
    fn an_expired_token_returns_an_error_distinct_from_the_signature() {
        let token = sign(&claims(0), SECRET).expect("signature");

        assert!(matches!(verify(&token, SECRET), Err(JwtError::Expired),));
    }

    #[test]
    fn an_invalid_signature_is_rejected() {
        let token = sign(&claims(LATER), SECRET).expect("signature");

        assert!(matches!(
            verify(&token, "un other secret tout aussi long ici"),
            Err(JwtError::Signature),
        ));
    }

    #[test]
    fn an_alg_none_token_is_rejected() {
        let token = sign(&claims(LATER), SECRET).expect("signature");
        let charge = token.split('.').nth(1).expect("charge utile");
        // `{"alg":"none","typ":"JWT"}` en base64url, suivi d'une signature vide.
        let forge = format!("eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.{charge}.");

        assert!(
            verify(&forge, SECRET).is_err(),
            "un token non signé ne doit jamais être accepté"
        );
    }

    // `alg: none` ne peut pas franchir la désérialisation de l'en-tête : `Algorithm` n'a
    // pas de variante `None`. La confusion d'algorithme réellement atteignable est
    // celle-ci — un algorithme supporté, mais autre que celui attendu.
    #[test]
    fn a_token_signed_with_another_algorithm_is_rejected() {
        let token = encode(
            &Header::new(Algorithm::HS512),
            &claims(LATER),
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("signature HS512");

        assert!(
            verify(&token, SECRET).is_err(),
            "l'algorithme attendu doit être imposé, pas lu dans l'en-tête"
        );
    }
}
