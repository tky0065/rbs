//! Contrôle de la feature `rate-limit`.
//!
//! La section porte la limite globale et les bornes plus strictes des routes coûteuses —
//! `/auth/login` hache un Argon2 à chaque requête. Le manifeste dit la feature installée, la
//! configuration ce qui est réglé : c'est l'écart entre les deux que ce contrôle nomme,
//! comme celui de `jobs`.

use super::{Check, Config};

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "rate-limit";
const SECTION: &str = "rate_limit";
const REGLAGES: &str = concat!(
    "limit = 120\n",
    "window_secs = 60\n",
    "trust_forwarded_for = false\n",
    "routes = [\n",
    "  { path = \"/auth/login\", limit = 5, window_secs = 60 },\n",
    "  { path = \"/auth/forgot-password\", limit = 3, window_secs = 3600 },\n",
    "  { path = \"/auth/resend-verification\", limit = 3, window_secs = 3600 },\n",
    "  { path = \"/auth/register\", limit = 10, window_secs = 3600 },\n",
    "]",
);

/// Vérifie que la limite de débit a les réglages sous lesquels le fragment a été installé.
pub(crate) fn check(config: &Config) -> Check {
    super::section_check(
        config,
        TITRE,
        SECTION,
        "la limite de débit a ses réglages",
        REGLAGES,
    )
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
    fn without_a_rate_limit_section_the_diagnosis_says_so() {
        let (_racine, config) = config("[server]\nport = 8080\n");

        let check = check(&config);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("[rate_limit]"), "{}", check.detail);
    }

    #[test]
    fn a_configured_project_reports_nothing() {
        let (_racine, config) = config(&format!("[rate_limit]\n{REGLAGES}\n"));

        let check = check(&config);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le remède se colle tel quel, bornes des routes coûteuses comprises : sans elles,
    /// `/auth/login` et ses voisines retombent sous la limite globale.
    #[test]
    fn the_remedy_carries_the_settings_of_the_fragment() {
        assert_eq!(
            REGLAGES,
            super::super::tests::reglages_du_fragment("rate-limit", "rate_limit")
        );
    }
}
