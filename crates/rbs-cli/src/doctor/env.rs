//! Contrôle du `.env` du projet.
//!
//! `.env.example` sert de référence : il est versionné, généré par le squelette et mis à
//! jour en même temps que lui. Comparer à une liste tenue dans le CLI aurait fait deux
//! vérités à synchroniser.

use std::path::Path;

use crate::dotenv;

use super::Check;

const TITRE: &str = ".env";
const FICHIER: &str = ".env";
const EXEMPLE: &str = ".env.example";

/// Vérifie que le `.env` porte tout ce que `.env.example` déclare.
pub(crate) fn check(root: &Path) -> Check {
    let attendues = match dotenv::read(&root.join(EXEMPLE)) {
        Ok(paires) => paires,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("{EXEMPLE} est la référence du diagnostic : restaurez-le depuis Git"),
            );
        }
    };

    let presentes = match dotenv::read(&root.join(FICHIER)) {
        Ok(paires) => paires,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("cp {EXEMPLE} {FICHIER}, puis renseignez l'URL de votre base"),
            );
        }
    };

    // Une variable propre au projet est légitime : seule l'absence est un défaut.
    let manquantes: Vec<&str> = attendues
        .iter()
        .map(|(key, _)| key.as_str())
        .filter(|key| dotenv::value(&presentes, key).is_none())
        .collect();

    if manquantes.is_empty() {
        return Check::ok(
            TITRE,
            format!(
                "les {} variables de {EXEMPLE} sont renseignées",
                attendues.len()
            ),
        );
    }

    Check::failed(
        TITRE,
        format!(
            "{} absente{} du {FICHIER}",
            manquantes.join(", "),
            if manquantes.len() > 1 { "s" } else { "" }
        ),
        format!(
            "ajoutez au {FICHIER} :\n{}",
            manquantes
                .iter()
                .map(|key| format!("{key}={}", dotenv::value(&attendues, key).unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\n")
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

    fn project() -> (TempDir, PathBuf) {
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

        (parent, project.root)
    }

    /// Retire du `.env` la ligne portant `key`.
    fn remove(root: &Path, key: &str) {
        let path = root.join(FICHIER);
        let source = fs::read_to_string(&path).expect("le .env est lisible");
        let ampute: Vec<_> = source
            .lines()
            .filter(|line| !line.starts_with(key))
            .collect();
        fs::write(&path, ampute.join("\n")).expect("le .env est réécrivable");
    }

    #[test]
    fn a_fresh_project_has_a_complete_env() {
        let (_parent, root) = project();

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(check.remedy.is_none());
    }

    #[test]
    fn a_variable_from_the_example_missing_from_env_is_named() {
        let (_parent, root) = project();
        remove(&root, "RBS_LOG_FORMAT");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("RBS_LOG_FORMAT"));
        assert!(
            check
                .remedy
                .expect("un échec porte son remède")
                .contains("RBS_LOG_FORMAT")
        );
    }

    #[test]
    fn the_finding_agrees_with_the_number_of_missing_variables() {
        let (_parent, root) = project();
        remove(&root, "RBS_LOG_FORMAT");

        assert!(check(&root).detail.contains("absente du"));

        remove(&root, "RUST_LOG");

        assert!(check(&root).detail.contains("absentes du"));
    }

    #[test]
    fn a_project_specific_variable_does_not_get_in_the_way() {
        let (_parent, root) = project();
        let path = root.join(FICHIER);
        let source = fs::read_to_string(&path).expect("le .env est lisible");
        fs::write(&path, format!("{source}\nSTRIPE_KEY=sk_test\n")).expect("écriture");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn a_missing_env_points_to_the_example_that_rebuilds_it() {
        let (_parent, root) = project();
        fs::remove_file(root.join(FICHIER)).expect("le .env existe");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(
            check
                .remedy
                .expect("un échec porte son remède")
                .contains(EXEMPLE)
        );
    }

    #[test]
    fn without_the_example_file_the_check_says_so_rather_than_concluding_green() {
        let (_parent, root) = project();
        fs::remove_file(root.join(EXEMPLE)).expect("l'exemple existe");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains(EXEMPLE));
    }
}
