//! Contrôle de la feature `observability`.
//!
//! Deux verdicts, et une seule question par verdict : la section est-elle là, et le
//! second listener a-t-il un port à lui. Un `metrics_port` égal à `server.port` fait
//! échouer le `bind` au démarrage, et un démarrage qui tombe sur un port occupé coûte
//! plus cher à diagnostiquer qu'une configuration refusée avant.
//!
//! L'endpoint OTLP n'est pas contrôlé : son absence est un mode de fonctionnement
//! légitime — un poste de développement n'a pas de collecteur — et non une faute.

use super::{CONFIG, Check, Config};

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "observability";
const SECTION: &str = "observability";
const REGLAGES: &str = "metrics_port = 9090";

/// Le port du fragment, quand la section ne le dit pas.
const PORT_PAR_DEFAUT: i64 = 9090;

/// Vérifie que les métriques ont une section et un port qui n'est pas celui de l'API.
pub(crate) fn check(config: &Config) -> Check {
    if !config.section(SECTION) {
        return super::section_check(
            config,
            TITRE,
            SECTION,
            "les métriques ont leur section",
            REGLAGES,
        );
    }

    let metriques = config
        .integer(SECTION, "metrics_port")
        .unwrap_or(PORT_PAR_DEFAUT);

    // Le port de l'API vaut son propre défaut : une section `[server]` sans `port` laisse
    // le noyau écouter le sien, et la collision serait la même.
    if config.integer("server", "port") == Some(metriques) {
        return Check::failed(
            TITRE,
            format!("`observability.metrics_port` et `server.port` valent tous deux {metriques}"),
            format!(
                "donnez aux métriques un port à elles dans {CONFIG} :\n[{SECTION}]\n{REGLAGES}"
            ),
        );
    }

    Check::ok(
        TITRE,
        format!("les métriques écoutent sur {metriques}, à part de l'API"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add observability` y dépose.
    ///
    /// La commande n'est pas appelée : ce contrôle ne lit qu'un fichier, et le poser
    /// directement garde le test à la seconde plutôt qu'à la minute.
    fn project_with_observability() -> (TempDir, PathBuf) {
        let (parent, root) = crate::fixtures::project();
        let config = root.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(
            &config,
            format!("{source}\n[observability]\nmetrics_port = 9090\n"),
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
    fn a_properly_configured_project_reports_nothing() {
        let (_parent, root) = project_with_observability();

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn without_an_observability_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_observability();
        rewrite(&root, "[observability]", "# section retirée par le test");

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[observability]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    /// Le verdict qui justifie le contrôle : deux listeners sur un même port, c'est un
    /// `bind` qui échoue au démarrage — et rien, dans le message du noyau, ne dirait
    /// lequel des deux est de trop.
    #[test]
    fn the_metrics_port_may_not_be_the_one_the_api_listens_on() {
        let (_parent, root) = project_with_observability();
        rewrite(&root, "metrics_port = 9090", "metrics_port = 8080");

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        for nomme in ["observability.metrics_port", "server.port", "8080"] {
            assert!(
                check.detail.contains(nomme),
                "`{nomme}` manque au détail : {}",
                check.detail
            );
        }
        assert!(
            check
                .remedy
                .expect("un échec porte son remède")
                .contains("metrics_port"),
            "le remède doit se coller tel quel"
        );
    }

    /// La section sans son port n'est pas une section sans port : le fragment en a un par
    /// défaut, et la collision se juge sur celui-là.
    #[test]
    fn an_unset_port_is_judged_on_the_default_of_the_fragment() {
        let (_parent, root) = project_with_observability();
        rewrite(&root, "metrics_port = 9090", "");
        rewrite(&root, "port = 8080", "port = 9090");

        let check = check(&Config::read(&root));

        assert_eq!(check.state, State::Echec, "{}", check.detail);
    }
}
