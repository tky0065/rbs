//! Contrôle de la feature `cors`.
//!
//! Le fragment écrit `origins = []`, et c'est délibéré : une API qui n'énumère pas ses
//! clients n'a aucune raison d'en autoriser un. Ce défaut sûr est aussi le premier obstacle
//! qu'un front rencontre — ses appels refusés par le navigateur, sans une ligne au journal
//! du serveur. Le contrôle le signale donc, sans rendre malade un projet qui n'a pas encore
//! de front : un avertissement, et non un échec.

use super::{CONFIG, Check, Config};

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "cors";
const SECTION: &str = "cors";
const REGLAGES: &str = concat!(
    "origins = []\n",
    "methods = [\"GET\", \"POST\", \"PUT\", \"PATCH\", \"DELETE\", \"OPTIONS\"]\n",
    "headers = [\"authorization\", \"content-type\"]\n",
    "credentials = false\n",
    "max_age_secs = 3600",
);

/// Vérifie que la section est là, et dit si elle autorise une origine.
pub(crate) fn check(config: &Config) -> Check {
    if !config.section(SECTION) {
        return super::section_check(
            config,
            TITRE,
            SECTION,
            "les origines autorisées sont énumérées",
            REGLAGES,
        );
    }

    // Une clé absente vaut la liste vide que serde y met.
    match config.array_len(SECTION, "origins").unwrap_or(0) {
        0 => Check::warned(
            TITRE,
            "`cors.origins` est vide : aucun front ne peut appeler l'API depuis un navigateur",
            format!(
                "énumérez les origines de votre front dans {CONFIG} — ou dans le profil de \
                 l'environnement qui les sert :\n[{SECTION}]\norigins = [\"http://localhost:5173\"]"
            ),
        ),
        1 => Check::ok(TITRE, "1 origine autorisée"),
        nombre => Check::ok(TITRE, format!("{nombre} origines autorisées")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::{CONFIG, Config, State};
    use super::*;

    /// La configuration d'un projet réduite à `contenu`, lue comme le diagnostic la lit.
    fn config(contenu: &str) -> (TempDir, Config) {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        let chemin = racine.path().join(CONFIG);
        fs::create_dir_all(chemin.parent().expect("la configuration a un parent"))
            .expect("répertoire de configuration créable");
        fs::write(&chemin, contenu).expect("configuration inscriptible");
        let config = Config::read(racine.path());

        (racine, config)
    }

    #[test]
    fn without_a_cors_section_the_diagnosis_says_so() {
        let (_racine, config) = config("[server]\nport = 8080\n");

        let check = check(&config);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("[cors]"), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains("origins = []")),
            "{:?}",
            check.remedy
        );
    }

    /// Le défaut que le fragment écrit est le défaut sûr : il se signale sans rendre le
    /// projet malade.
    #[test]
    fn the_empty_origins_the_fragment_ships_are_a_warning_not_a_failure() {
        let (_racine, config) = config(&format!("[cors]\n{REGLAGES}\n"));

        let check = check(&config);

        assert_eq!(check.state, State::Avertissement, "{}", check.detail);
        assert!(check.detail.contains("origins"), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains("origins = [\"")),
            "{:?}",
            check.remedy
        );
    }

    /// Une clé absente vaut la liste vide que serde y met.
    #[test]
    fn a_section_without_origins_is_the_same_warning() {
        let (_racine, config) = config("[cors]\ncredentials = false\n");

        assert_eq!(check(&config).state, State::Avertissement);
    }

    #[test]
    fn a_named_origin_reports_nothing() {
        let (_racine, config) = config(&format!(
            "[cors]\n{}\n",
            REGLAGES.replace("origins = []", "origins = [\"http://localhost:5173\"]")
        ));

        let check = check(&config);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(
            check.detail.contains("1 origine autorisée"),
            "{}",
            check.detail
        );
    }

    #[test]
    fn two_origins_are_counted_in_the_plural() {
        let (_racine, config) =
            config("[cors]\norigins = [\"https://a.exemple\", \"https://b.exemple\"]\n");

        let check = check(&config);

        assert!(
            check.detail.contains("2 origines autorisées"),
            "{}",
            check.detail
        );
    }

    /// Le remède se colle tel quel : ce sont les réglages que `add cors` écrit.
    #[test]
    fn the_remedy_carries_the_settings_of_the_fragment() {
        assert_eq!(
            REGLAGES,
            super::super::tests::reglages_du_fragment("cors", "cors")
        );
    }
}
