//! Contrôle de la feature `redis`.
//!
//! La feature se déclare `redis` mais s'installe en `src/cache/`, sous une section
//! `[cache]` : c'est le nom de la crate d'un côté, celui du service rendu de l'autre. Le
//! contrôle porte le nom déclaré, comme les autres, et nomme la section dans son détail.

use super::{Check, Config};

const TITRE: &str = "redis";
const SECTION: &str = "cache";

/// Vérifie ce dont la feature `redis` a besoin pour démarrer.
pub(crate) fn check(config: &Config) -> Check {
    super::section_check(
        config,
        TITRE,
        SECTION,
        "la configuration du cache est en place",
        "url = \"redis://127.0.0.1:6379\"\nttl_secs = 300",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::super::{CONFIG, State};
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add redis` y dépose.
    ///
    /// La commande n'est pas appelée : ce contrôle ne lit qu'un fichier, et le poser
    /// directement garde le test à la seconde plutôt qu'à la minute.
    fn project_with_redis() -> (TempDir, PathBuf) {
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
            format!("{source}\n[cache]\nurl = \"redis://127.0.0.1:6379\"\nttl_secs = 300\n"),
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
    fn without_a_cache_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_redis();
        rewrite(&root, "[cache]", "# section retirée par le test");

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[cache]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    /// Une section en commentaire n'est pas une section.
    #[test]
    fn a_commented_out_cache_does_not_count_as_a_section() {
        let (_parent, root) = project_with_redis();
        rewrite(&root, "[cache]", "# [cache]");

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Echec, "{}", check.detail);
    }

    #[test]
    fn a_properly_configured_project_reports_nothing() {
        let (_parent, root) = project_with_redis();

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }
}
