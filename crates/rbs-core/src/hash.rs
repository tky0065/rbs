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

/// Hache `mot_de_passe` avec Argon2id et un sel tiré pour cet appel.
///
/// # Erreurs
///
/// Échoue si le générateur du système ou Argon2 défaille — jamais du fait de l'entrée.
pub fn hacher(mot_de_passe: &str) -> crate::Result<String> {
    let sel = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(mot_de_passe.as_bytes(), &sel)
        .map(|hash| hash.to_string())
        .map_err(|erreur| Error::Internal(anyhow::anyhow!("hachage Argon2 : {erreur}")))
}

/// Vérifie `mot_de_passe` contre un `hash` au format PHC.
///
/// # Erreurs
///
/// Échoue si `hash` est illisible, ce qui vient de la base ou d'un bug, jamais du
/// client. Un mot de passe faux n'est pas une erreur : c'est `Ok(false)`.
pub fn verifier(mot_de_passe: &str, hash: &str) -> crate::Result<bool> {
    let attendu = PasswordHash::new(hash)
        .map_err(|erreur| Error::Internal(anyhow::anyhow!("hash PHC illisible : {erreur}")))?;

    match Argon2::default().verify_password(mot_de_passe.as_bytes(), &attendu) {
        Ok(()) => Ok(true),
        // Le seul cas où l'échec vient du client : il ne doit pas devenir un 500.
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(erreur) => Err(Error::Internal(anyhow::anyhow!(
            "vérification Argon2 : {erreur}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_hachages_du_meme_mot_de_passe_different() {
        let a = hacher("correct horse battery staple").expect("hachage");
        let b = hacher("correct horse battery staple").expect("hachage");
        assert_ne!(a, b, "le sel doit être tiré à chaque appel");
    }

    #[test]
    fn verifier_accepte_le_bon_mot_de_passe_et_rejette_un_autre() {
        let hash = hacher("s3cr3t").expect("hachage");
        assert!(verifier("s3cr3t", &hash).expect("vérification"));
        assert!(!verifier("s3cr3T", &hash).expect("vérification"));
    }

    #[test]
    fn un_hash_malforme_rend_une_erreur_sans_paniquer() {
        assert!(verifier("s3cr3t", "pas un hash PHC").is_err());
    }
}
