//! Hachage et vérification des mots de passe.
//!
//! Argon2id avec les paramètres par défaut de la crate `argon2`, et un sel tiré du
//! générateur du système à chaque appel. Le résultat est une chaîne PHC
//! (`$argon2id$v=19$...`) qui porte son sel et ses paramètres : rehacher un mot de passe
//! stocké sous d'anciens paramètres reste possible sans migration de schéma.

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use crate::Error;

/// Hache `password` avec Argon2id et un sel tiré pour cet appel.
///
/// # Erreurs
///
/// Échoue si le générateur du système ou Argon2 défaille — jamais du fait de l'entrée.
pub fn hash_password(password: &str) -> crate::Result<String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| Error::Internal(anyhow::anyhow!("hachage Argon2 : {error}")))
}

/// Vérifie `password` contre un `hash` au format PHC.
///
/// # Erreurs
///
/// Échoue si `hash` est illisible, ce qui vient de la base ou d'un bug, jamais du
/// client. Un mot de passe faux n'est pas une erreur : c'est `Ok(false)`.
pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
    let expected = PasswordHash::new(hash)
        .map_err(|error| Error::Internal(anyhow::anyhow!("hash PHC illisible : {error}")))?;

    match Argon2::default().verify_password(password.as_bytes(), &expected) {
        Ok(()) => Ok(true),
        // Le seul cas où l'échec vient du client : il ne doit pas devenir un 500.
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(error) => Err(Error::Internal(anyhow::anyhow!(
            "vérification Argon2 : {error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_hashes_of_the_same_password_differ() {
        let a = hash_password("correct horse battery staple").expect("hachage");
        let b = hash_password("correct horse battery staple").expect("hachage");
        assert_ne!(a, b, "le salt doit être tiré à chaque appel");
    }

    #[test]
    fn verify_accepts_the_right_password_and_rejects_another() {
        let hash = hash_password("s3cr3t").expect("hachage");
        assert!(verify_password("s3cr3t", &hash).expect("vérification"));
        assert!(!verify_password("s3cr3T", &hash).expect("vérification"));
    }

    #[test]
    fn a_malformed_hash_returns_an_error_without_panicking() {
        assert!(verify_password("s3cr3t", "pas un hash PHC").is_err());
    }
}
