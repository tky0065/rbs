//! Contrôle de la feature `ci`.
//!
//! Le fragment pose le workflow que GitHub lit, et la configuration de Dependabot qui en
//! monte les actions : c'est le premier que ce contrôle cherche. Un workflow disparu est une
//! CI qui ne tourne plus, et rien ne vire au rouge pour le dire — il n'y a plus rien pour le
//! peindre. Sans Dependabot, la CI tourne encore : ses actions cessent seulement de monter.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "ci";
const FICHIER: &str = ".github/workflows/ci.yml";

/// Vérifie que le workflow que pose le fragment est en place.
pub(crate) fn check(root: &Path) -> Check {
    if root.join(FICHIER).is_file() {
        return Check::ok(TITRE, "le workflow d'intégration continue est en place");
    }

    Check::failed(
        TITRE,
        format!("{FICHIER} est absent : aucune CI ne tourne plus sur ce projet"),
        super::restaurer(FICHIER),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    #[test]
    fn the_workflow_reports_nothing() {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        let fichier = racine.path().join(FICHIER);
        fs::create_dir_all(fichier.parent().expect("le workflow a un parent"))
            .expect("répertoire créable");
        fs::write(&fichier, "name: CI\n").expect("workflow inscriptible");

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn a_missing_workflow_is_named_and_restored_from_git() {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains("git checkout")),
            "{:?}",
            check.remedy
        );
    }

    /// Le fichier cherché est celui que le fragment pose.
    #[test]
    fn the_file_is_the_one_the_fragment_writes() {
        let manifeste = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/ci/feature.toml"
        ))
        .expect("le manifeste du fragment se lit");

        assert!(
            manifeste.contains(&format!("destination = \"{FICHIER}\"")),
            "le fragment ne pose plus {FICHIER}"
        );
    }
}
