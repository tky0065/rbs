//! L'état du working tree d'un projet, tel que `git` le rapporte.
//!
//! Une bibliothèque Git serait une dépendance pour une seule question ; `git status
//! --porcelain` y répond, et dans les mots que le développeur lira ensuite lui-même.

use std::path::Path;
use std::process::Command;

/// Les chemins des fichiers suivis modifiés sous `racine`.
///
/// Vide hors d'un dépôt Git, si `git` est introuvable, ou si le working tree est propre :
/// dans ces trois cas, il n'y a rien à protéger.
pub(crate) fn fichiers_modifies(racine: &Path) -> Vec<String> {
    let Ok(sortie) = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(racine)
        .output()
    else {
        return Vec::new();
    };

    if !sortie.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&sortie.stdout)
        .lines()
        // Les fichiers non suivis sont précisément ceux que le CLI s'apprête à créer.
        .filter(|ligne| !ligne.starts_with("??"))
        .filter_map(chemin)
        .collect()
}

/// Le chemin d'une ligne `XY chemin`, ou sa destination pour un renommage
/// `R  ancien -> nouveau`.
fn chemin(ligne: &str) -> Option<String> {
    let chemin = ligne.get(3..)?.trim();
    let chemin = chemin.rsplit(" -> ").next()?;

    (!chemin.is_empty()).then(|| chemin.to_string())
}

/// Énumère des chemins en cause, sans dérouler une liste illisible.
///
/// Un working tree sale peut compter des centaines de fichiers : les nommer tous noie le
/// message dans ce qu'il est censé rendre lisible.
pub(crate) fn enumerer(fichiers: &[String]) -> String {
    const NOMMES: usize = 5;

    let debut = fichiers
        .iter()
        .take(NOMMES)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    match fichiers.len().saturating_sub(NOMMES) {
        0 => debut,
        reste => format!("{debut} … et {reste} autres"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;

    /// Un dépôt Git jetable, avec un commit initial : `git status` d'un dépôt sans commit
    /// se comporte autrement.
    fn depot() -> TempDir {
        let depot = TempDir::new().expect("répertoire temporaire créable");

        git(depot.path(), &["init", "--quiet"]);
        git(depot.path(), &["config", "user.email", "rbs@example.test"]);
        git(depot.path(), &["config", "user.name", "rbs"]);

        fs::write(depot.path().join("suivi.txt"), "initial\n").expect("fichier écrivable");

        git(depot.path(), &["add", "suivi.txt"]);
        git(depot.path(), &["commit", "--quiet", "-m", "initial"]);

        depot
    }

    fn git(racine: &Path, arguments: &[&str]) {
        let sortie = Command::new("git")
            .args(arguments)
            .current_dir(racine)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git doit être lançable");

        assert!(
            sortie.status.success(),
            "git {arguments:?} a échoué :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
    }

    #[test]
    fn un_working_tree_propre_ne_signale_rien() {
        assert!(fichiers_modifies(depot().path()).is_empty());
    }

    #[test]
    fn un_fichier_suivi_modifie_est_signale() {
        let depot = depot();
        fs::write(depot.path().join("suivi.txt"), "modifié\n").expect("fichier réécrivable");

        assert_eq!(
            fichiers_modifies(depot.path()),
            vec!["suivi.txt".to_string()]
        );
    }

    #[test]
    fn un_fichier_non_suivi_ne_bloque_pas() {
        let depot = depot();
        fs::write(depot.path().join("nouveau.txt"), "jamais ajouté\n").expect("fichier écrivable");

        assert!(fichiers_modifies(depot.path()).is_empty());
    }

    #[test]
    fn un_fichier_suivi_renomme_est_signale_par_sa_destination() {
        let depot = depot();
        git(depot.path(), &["mv", "suivi.txt", "renomme.txt"]);

        assert_eq!(
            fichiers_modifies(depot.path()),
            vec!["renomme.txt".to_string()]
        );
    }

    #[test]
    fn un_repertoire_hors_depot_ne_signale_rien() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        assert!(fichiers_modifies(ailleurs.path()).is_empty());
    }
}
