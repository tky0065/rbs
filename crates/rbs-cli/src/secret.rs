//! Tirage des secrets que `rbs` dépose dans le `.env` d'un projet.
//!
//! L'hexadécimal plutôt que le base64 de `rbs-core` : la valeur traverse un fichier
//! d'environnement, où seul un alphabet sans `+`, `/` ni `=` échappe à la question du
//! guillemet.

use rand::TryRng;
use rand::rngs::SysRng;

/// Longueur du secret tiré, en octets.
///
/// Le double du minimum qu'exige `rbs-core` : la marge coûte 32 octets dans un fichier
/// et dispense d'y revenir.
const OCTETS: usize = 32;

/// Tire un secret de 32 octets, rendu en hexadécimal minuscule.
///
/// # Panics
///
/// Panique si le générateur du système est indisponible. Aucun appelant ne saurait
/// traiter cet échec : sans source d'aléa, il n'y a pas de secret à écrire, et en
/// inventer un serait précisément le défaut que cette fonction corrige.
pub(crate) fn tire_au_hasard() -> String {
    let mut octets = [0u8; OCTETS];
    SysRng
        .try_fill_bytes(&mut octets)
        .expect("le générateur du système doit être disponible");

    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_is_sixty_four_hexadecimal_characters() {
        let secret = tire_au_hasard();

        assert_eq!(secret.len(), 64, "{secret}");
        assert!(
            secret
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "{secret}"
        );
    }

    /// Le critère de la tâche : deux installations ne partagent pas leur secret.
    #[test]
    fn two_draws_do_not_collide() {
        assert_ne!(tire_au_hasard(), tire_au_hasard());
    }
}
