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
const OCTETS: usize = 32;

/// Tire un jeton opaque de 32 octets, encodé en base64url sans remplissage.
///
/// # Panics
///
/// Panique si le générateur du système est indisponible. Aucun appelant ne saurait
/// traiter cet échec : sans source d'aléa, il n'y a pas de session à ouvrir.
pub fn aleatoire() -> String {
    let mut octets = [0u8; OCTETS];
    SysRng
        .try_fill_bytes(&mut octets)
        .expect("le générateur du système doit être disponible");

    URL_SAFE_NO_PAD.encode(octets)
}

/// Empreinte SHA-256 d'un jeton, en hexadécimal minuscule, pour le stockage.
///
/// C'est elle qui va en base : une base lue par un tiers ne lui donne alors aucune
/// session utilisable.
pub fn empreinte(jeton: &str) -> String {
    Sha256::digest(jeton.as_bytes())
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_tirages_successifs_different() {
        assert_ne!(aleatoire(), aleatoire());
    }

    #[test]
    fn le_jeton_decode_porte_au_moins_32_octets() {
        let octets = URL_SAFE_NO_PAD
            .decode(aleatoire())
            .expect("base64url sans remplissage");

        assert!(octets.len() >= 32, "obtenu : {} octets", octets.len());
    }

    #[test]
    fn l_empreinte_est_deterministe_et_ne_rend_pas_le_jeton() {
        let jeton = aleatoire();

        assert_eq!(empreinte(&jeton), empreinte(&jeton));
        assert_ne!(empreinte(&jeton), jeton);
        assert_eq!(empreinte(&jeton).len(), 64, "SHA-256 en hexadécimal");
        assert_ne!(empreinte(&jeton), empreinte(&aleatoire()));
    }
}
