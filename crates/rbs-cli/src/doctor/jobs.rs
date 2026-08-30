//! Contrôle de la feature `jobs`.
//!
//! Le manifeste dit ce qui est installé, la configuration ce qui est réglé. L'écart
//! entre les deux est la faute que ce contrôle nomme : le projet compile — les défauts
//! du fragment sont dans son `Config` — mais le worker démarre sur des réglages que
//! personne ne voit plus.

use std::path::Path;

use super::Check;

const TITRE: &str = "jobs";
const CONFIG: &str = "config/default.toml";
const SECTION: &str = "jobs";

/// Vérifie que la file a les réglages sous lesquels le fragment a été installé.
///
/// Seul `config/default.toml` est lu : le CLI ne sait pas quel `RBS_ENV` l'utilisateur
/// emploiera, et une section posée dans le seul `config/production.toml` échapperait donc
/// au diagnostic comme elle échappe au défaut du projet.
pub(crate) fn check(root: &Path) -> Check {
    if super::section(root, SECTION) {
        return Check::ok(TITRE, "la configuration de la file est en place");
    }

    Check::failed(
        TITRE,
        format!("{CONFIG} ne porte pas de section `[{SECTION}]`"),
        format!(
            "ajoutez à {CONFIG} :\n[{SECTION}]\nmax_attempts = 5\nretry_delay_secs = 30\npoll_interval_secs = 1"
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add jobs` y dépose.
    ///
    /// La commande n'est pas appelée : ce contrôle ne lit qu'un fichier, et le poser
    /// directement garde le test à la seconde plutôt qu'à la minute.
    fn project_with_jobs() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: crate::lang::Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let root = project.root;
        let config = root.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(
            &config,
            format!(
                "{source}\n[jobs]\nmax_attempts = 5\nretry_delay_secs = 30\npoll_interval_secs = 1\n"
            ),
        )
        .expect("config inscriptible");

        (parent, root)
    }

    /// Remplace dans `config/default.toml`, ce que font les tests qui mordent la section.
    fn rewrite(root: &Path, from: &str, to: &str) {
        let config = root.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(&config, source.replace(from, to)).expect("config inscriptible");
    }

    #[test]
    fn without_a_jobs_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_jobs();
        rewrite(&root, "[jobs]", "# section retirée par le test");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[jobs]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    /// Une section en commentaire n'est pas une section.
    #[test]
    fn a_commented_out_jobs_does_not_count_as_a_section() {
        let (_parent, root) = project_with_jobs();
        rewrite(&root, "[jobs]", "# [jobs]");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
    }

    /// Le remède se colle tel quel : les trois clés que `add jobs` pose y figurent.
    #[test]
    fn the_remedy_carries_the_three_settings_of_the_fragment() {
        let (_parent, root) = project_with_jobs();
        rewrite(&root, "[jobs]", "# [jobs]");

        let remedy = check(&root).remedy.expect("un échec porte son remède");

        for key in ["max_attempts", "retry_delay_secs", "poll_interval_secs"] {
            assert!(remedy.contains(key), "`{key}` manque au remède : {remedy}");
        }
    }

    #[test]
    fn a_properly_configured_project_reports_nothing() {
        let (_parent, root) = project_with_jobs();

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }
}
