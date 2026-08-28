//! Tirage et empreinte des jetons opaques.
//!
//! Volontairement sans Argon2 : un jeton porte 256 bits tirés au hasard, hors d'atteinte
//! de toute recherche exhaustive. Un KDF lent se paierait à chaque rafraîchissement sans
//! rien acheter. C'est SHA-256 qui empreinte, et le jeton lui-même ne quitte le processus
//! que vers le client.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::TryRng;
use rand::rngs::SysRng;
use sha2::{Digest, Sha256};

/// Longueur du jeton tiré, en octets.
const BYTES: usize = 32;

/// Tire un jeton opaque de 32 octets, encodé en base64url sans remplissage.
///
/// # Panics
///
/// Panique si le générateur du système est indisponible. Aucun appelant ne saurait
/// traiter cet échec : sans source d'aléa, il n'y a pas de session à ouvrir.
pub fn random() -> String {
    let mut bytes = [0u8; BYTES];
    SysRng
        .try_fill_bytes(&mut bytes)
        .expect("le générateur du système doit être disponible");

    URL_SAFE_NO_PAD.encode(bytes)
}

/// Empreinte SHA-256 d'un jeton, en hexadécimal minuscule, pour le stockage.
///
/// C'est elle qui va en base : une base lue par un tiers ne lui donne alors aucune
/// session utilisable.
pub fn fingerprint(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_successive_draws_differ() {
        assert_ne!(random(), random());
    }

    #[test]
    fn the_decoded_token_carries_at_least_32_bytes() {
        let bytes = URL_SAFE_NO_PAD
            .decode(random())
            .expect("base64url sans remplissage");

        assert!(bytes.len() >= 32, "obtenu : {} bytes", bytes.len());
    }

    #[test]
    fn the_fingerprint_is_deterministic_and_does_not_return_the_token() {
        let token = random();

        assert_eq!(fingerprint(&token), fingerprint(&token));
        assert_ne!(fingerprint(&token), token);
        assert_eq!(fingerprint(&token).len(), 64, "SHA-256 en hexadécimal");
        assert_ne!(fingerprint(&token), fingerprint(&random()));
    }
}
