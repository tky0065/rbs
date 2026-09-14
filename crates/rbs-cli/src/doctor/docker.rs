//! Contrôle de la feature `docker`.
//!
//! Le service `api` du compose que le fragment écrit pose `RBS_ENV: production`, et la
//! cascade de configuration lit alors `config/production.toml` par-dessus le défaut. C'est
//! ce profil qui coupe `/docs` et le document OpenAPI : sans lui, le déploiement publie le
//! plan de toute l'API, sans qu'aucune erreur ne le signale.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "docker";
const FICHIER: &str = "config/production.toml";
const REGLAGES: &str = "[docs]\nswagger_ui = false\nopenapi_json = false";

/// Vérifie que le profil que retient le compose existe.
pub(crate) fn check(root: &Path) -> Check {
    if root.join(FICHIER).is_file() {
        return Check::ok(
            TITRE,
            "le profil de production que retient le compose est là",
        );
    }

    Check::failed(
        TITRE,
        format!("{FICHIER} est absent : le service `api` du compose pose RBS_ENV=production"),
        format!("créez {FICHIER} avec :\n{REGLAGES}"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    const FRAGMENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features/docker");

    #[test]
    fn the_production_profile_the_compose_selects_reports_nothing() {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        fs::create_dir_all(racine.path().join("config")).expect("répertoire créable");
        fs::write(racine.path().join(FICHIER), format!("{REGLAGES}\n"))
            .expect("profil inscriptible");

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn a_missing_production_profile_is_named_with_its_content() {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
        assert!(check.detail.contains("RBS_ENV"), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains(REGLAGES)),
            "{:?}",
            check.remedy
        );
    }

    /// Le fichier cherché est celui que le fragment pose, et le remède en recopie les
    /// réglages : un profil qui en dériverait rouvrirait la documentation en production.
    #[test]
    fn the_remedy_carries_the_profile_the_fragment_writes() {
        let manifeste = fs::read_to_string(format!("{FRAGMENT}/feature.toml"))
            .expect("le manifeste du fragment se lit");
        assert!(
            manifeste.contains(&format!("destination = \"{FICHIER}\"")),
            "le fragment ne pose plus {FICHIER}"
        );

        let profil = fs::read_to_string(format!("{FRAGMENT}/config/production.toml.jinja"))
            .expect("le profil du fragment se lit");
        let reglages = profil
            .lines()
            .filter(|ligne| !ligne.trim().is_empty() && !ligne.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(REGLAGES, reglages);
    }
}
