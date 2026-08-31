//! L'état du working tree d'un projet, tel que `git` le rapporte.
//!
//! Une bibliothèque Git serait une dépendance pour une seule question ; `git status
//! --porcelain` y répond, et dans les mots que le développeur lira ensuite lui-même.

use std::path::Path;
use std::process::Command;

/// Les chemins des fichiers suivis modifiés sous `root`.
///
/// Vide hors d'un dépôt Git, si `git` est introuvable, ou si le working tree est propre :
/// dans ces trois cas, il n'y a rien à protéger.
pub(crate) fn modified_files(root: &Path) -> Vec<String> {
    let Ok(output) = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        // Les fichiers non suivis sont précisément ceux que le CLI s'apprête à créer.
        .filter(|line| !line.starts_with("??"))
        .filter_map(path)
        .collect()
}

/// Refuse d'écrire dans un working tree qui porte des modifications non commitées.
///
/// Ce qu'une commande écrirait s'y mêlerait à ce que le développeur n'a pas encore
/// enregistré, et `git diff` ne les distinguerait plus.
pub(crate) fn garde(root: &Path) -> Result<(), crate::errors::WorkingTreeSale> {
    let modifies = modified_files(root);

    if modifies.is_empty() {
        return Ok(());
    }

    Err(crate::errors::WorkingTreeSale {
        files: enumerate(&modifies),
    })
}

/// Le chemin d'une ligne `XY path`, ou sa destination pour un renommage
/// `R  ancien -> new`.
fn path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    let path = path.rsplit(" -> ").next()?;

    (!path.is_empty()).then(|| path.to_string())
}

/// Énumère des chemins en cause, sans dérouler une liste illisible.
///
/// Un working tree sale peut compter des centaines de fichiers : les nommer tous noie le
/// message dans ce qu'il est censé rendre lisible.
fn enumerate(files: &[String]) -> String {
    const NOMMES: usize = 5;

    let debut = files
        .iter()
        .take(NOMMES)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    match files.len().saturating_sub(NOMMES) {
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
    fn repo() -> TempDir {
        let repo = TempDir::new().expect("répertoire temporaire créable");

        git(repo.path(), &["init", "--quiet"]);
        git(repo.path(), &["config", "user.email", "rbs@example.test"]);
        git(repo.path(), &["config", "user.name", "rbs"]);

        fs::write(repo.path().join("suivi.txt"), "initial\n").expect("fichier écrivable");

        git(repo.path(), &["add", "suivi.txt"]);
        git(repo.path(), &["commit", "--quiet", "-m", "initial"]);

        repo
    }

    fn git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git doit être lançable");

        assert!(
            output.status.success(),
            "git {arguments:?} a échoué :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn a_clean_working_tree_reports_nothing() {
        assert!(modified_files(repo().path()).is_empty());
    }

    #[test]
    fn a_modified_tracked_file_is_reported() {
        let repo = repo();
        fs::write(repo.path().join("suivi.txt"), "modifié\n").expect("fichier réécrivable");

        assert_eq!(modified_files(repo.path()), vec!["suivi.txt".to_string()]);
    }

    #[test]
    fn an_untracked_file_does_not_block() {
        let repo = repo();
        fs::write(repo.path().join("nouveau.txt"), "jamais ajouté\n").expect("fichier écrivable");

        assert!(modified_files(repo.path()).is_empty());
    }

    #[test]
    fn a_renamed_tracked_file_is_reported_by_its_destination() {
        let repo = repo();
        git(repo.path(), &["mv", "suivi.txt", "renomme.txt"]);

        assert_eq!(modified_files(repo.path()), vec!["renomme.txt".to_string()]);
    }

    #[test]
    fn a_directory_outside_a_repository_reports_nothing() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        assert!(modified_files(ailleurs.path()).is_empty());
    }
}
