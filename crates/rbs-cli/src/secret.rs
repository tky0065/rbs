//! Tirage des secrets que `rbs` dépose dans le `.env` d'un projet, et écriture de ce fichier.
//!
//! L'hexadécimal plutôt que le base64 de `rbs-core` : la valeur traverse un fichier
//! d'environnement, où seul un alphabet sans `+`, `/` ni `=` échappe à la question du
//! guillemet.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use rand::TryRng;
use rand::rngs::SysRng;

/// Le fichier qui porte les secrets d'un projet, et le seul que [`write`] referme.
///
/// `.env.example` n'en porte aucun : il est versionné, et garde les droits d'un fichier
/// ordinaire.
const FICHIER_ENV: &str = ".env";

/// Écrit `contenu` dans `path`, comme `fs::write` — sauf un `.env`, rendu sous Unix
/// lisible de son seul propriétaire, qu'il soit créé ou réécrit.
pub(crate) fn write(path: &Path, contenu: &[u8]) -> io::Result<()> {
    if path.file_name() != Some(FICHIER_ENV.as_ref()) {
        return fs::write(path, contenu);
    }

    ouvre_ferme(path)?.write_all(contenu)
}

/// Ouvre un `.env` à l'écriture, droits réduits au propriétaire avant tout octet écrit.
///
/// `mode` ne vaut qu'à la création : un `.env` antérieur en 0644 le resterait. D'où le
/// `set_permissions` sur le descripteur, avant le contenu — aucune fenêtre où le secret
/// serait écrit dans un fichier lisible de tous.
#[cfg(unix)]
fn ouvre_ferme(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;

    Ok(file)
}

#[cfg(not(unix))]
fn ouvre_ferme(path: &Path) -> io::Result<fs::File> {
    fs::File::create(path)
}

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
