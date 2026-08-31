//! `rbs doctor` : diagnostic d'un projet généré.
//!
//! Chaque contrôle est indépendant et rend son verdict sans interrompre les autres : un
//! diagnostic qui s'arrête au premier problème oblige à le relancer autant de fois qu'il
//! y a de problèmes.

pub mod agents;
pub mod anchors;
pub mod auth;
pub mod base;
pub mod env;
pub mod guards;
pub mod jobs;
pub mod mail;
pub mod redis;
pub mod relations;
pub mod render;
pub mod storage;
pub mod versions;

use std::path::Path;

use crate::metadata;

/// Verdict d'un contrôle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// Rien à signaler.
    Bon,
    /// Ce qui mérite d'être su sans empêcher le projet de fonctionner.
    Avertissement,
    /// Ce qui empêche le projet de fonctionner.
    Echec,
}

/// Ce qu'un contrôle a constaté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Check {
    /// Ce qui est vérifié, en un mot : `anchors`, `.env`, `versions`, `base`.
    pub title: &'static str,
    /// Verdict.
    pub state: State,
    /// Ce qui a été constaté, en une ligne.
    pub detail: String,
    /// Quoi faire, quand il y a quelque chose à faire.
    pub remedy: Option<String>,
}

impl Check {
    /// Un contrôle sans rien à signaler.
    pub(crate) fn ok(title: &'static str, detail: impl Into<String>) -> Self {
        Self {
            title,
            state: State::Bon,
            detail: detail.into(),
            remedy: None,
        }
    }

    /// Un contrôle en échec, et le geste qui le corrige.
    pub(crate) fn failed(
        title: &'static str,
        detail: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            title,
            state: State::Echec,
            detail: detail.into(),
            remedy: Some(remedy.into()),
        }
    }

    /// Un constat qui n'empêche rien, et ce qu'on peut en faire.
    pub(crate) fn warned(
        title: &'static str,
        detail: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            title,
            state: State::Avertissement,
            detail: detail.into(),
            remedy: Some(remedy.into()),
        }
    }
}

/// L'ensemble des constats, dans l'ordre où ils ont été faits.
#[derive(Debug)]
pub(crate) struct Report {
    /// Les contrôles, tous exécutés.
    pub checks: Vec<Check>,
}

impl Report {
    /// Vrai si aucun contrôle n'a échoué : un avertissement n'y fait pas obstacle.
    pub(crate) fn succeeded(&self) -> bool {
        self.checks.iter().all(|c| c.state != State::Echec)
    }
}

/// Ce qui peut empêcher de diagnostiquer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

/// Diagnostique le projet qui contient `directory`.
pub(crate) fn run(directory: &Path) -> Result<Report, Error> {
    let root = metadata::project_root(directory)?;

    let mut checks = vec![
        anchors::check(&root),
        agents::check(&root),
        relations::check(&root),
        env::check(&root),
        versions::check(&root),
        base::check(&root),
    ];

    // Une seule lecture pour toute la boucle : le manifeste était réanalysé en entier à
    // chaque entrée du tableau, et la configuration relue par chaque contrôle pour une
    // question d'une ligne.
    let installees = metadata::read(&root.join("Cargo.toml"))
        .map(|metadonnees| metadonnees.features)
        .unwrap_or_default();
    let config = Config::read(&root);

    // Un projet qui n'a pas installé une feature n'a pas à lire une ligne à son sujet :
    // le rapport ne porte que des contrôles dont le verdict le concerne.
    for (feature, check) in FEATURE_CHECKS {
        if installees.iter().any(|installee| installee == feature) {
            checks.push(check(&root, &config));
        }
    }

    Ok(Report { checks })
}

/// Une feature, sous le nom qu'elle porte dans le manifeste, et le contrôle qui la juge.
type FeatureCheck = (&'static str, fn(&Path, &Config) -> Check);

/// Le contrôle propre à chaque feature, sous le nom qu'elle porte dans le manifeste.
///
/// `redis` s'installe en `src/cache/` sous une section `[cache]` : c'est le nom de la
/// crate d'un côté, celui du service rendu de l'autre. Le tableau porte le nom déclaré,
/// seul commun aux quatre.
///
/// Une feature peut y figurer deux fois : `auth` amène de quoi vérifier son secret, et de
/// quoi juger les routes que les rôles qu'elle installe pourraient protéger.
///
/// Un contrôle qui n'interroge pas la configuration, ou qui n'interroge qu'elle, le dit
/// par une fermeture : lui imposer un paramètre qu'il n'emploie pas se lirait comme une
/// dépendance qu'il n'a pas.
const FEATURE_CHECKS: [FeatureCheck; 6] = [
    ("auth", auth::check),
    ("auth", |root, _| guards::check(root)),
    ("redis", |_, config| redis::check(config)),
    ("mail", mail::check),
    ("storage", storage::check),
    ("jobs", |_, config| jobs::check(config)),
];

/// Le fichier de configuration que les contrôles de feature interrogent.
const CONFIG: &str = "config/default.toml";

/// Le contrôle d'une feature dont tout le diagnostic tient à sa section de configuration.
///
/// Seul `config/default.toml` est lu : le CLI ne sait pas quel `RBS_ENV` l'utilisateur
/// emploiera, et une section posée dans le seul `config/production.toml` échapperait donc
/// au diagnostic comme elle échappe au défaut du projet.
///
/// `present` est le constat du succès, propre à chaque feature : le cache et la file ne
/// se nomment pas de la même façon dans un rapport.
fn section_check(
    config: &Config,
    titre: &'static str,
    section: &str,
    present: &str,
    reglages: &str,
) -> Check {
    if config.section(section) {
        return Check::ok(titre, present);
    }

    Check::failed(
        titre,
        format!("{CONFIG} ne porte pas de section `[{section}]`"),
        format!("ajoutez à {CONFIG} :\n[{section}]\n{reglages}"),
    )
}

/// `config/default.toml` du projet, lu et analysé une seule fois.
///
/// Un diagnostic complet interrogeait ce fichier jusqu'à huit fois, chaque contrôle le
/// relisant et le réanalysant pour une question d'une ligne — `storage` en enchaînait
/// trois d'affilée.
pub(crate) struct Config(Option<toml_edit::DocumentMut>);

impl Config {
    /// Lit la configuration du projet.
    ///
    /// Un fichier absent ou illisible se comporte comme un fichier vide : ce qui
    /// intéresse un contrôle est de disposer ou non de la valeur, jamais laquelle des
    /// couches manque.
    pub(crate) fn read(root: &Path) -> Self {
        Self(
            std::fs::read_to_string(root.join(CONFIG))
                .ok()
                .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok()),
        )
    }

    /// Vrai si la configuration porte une section `[name]`.
    ///
    /// Analysé par `toml_edit` et non cherché en texte : une section en commentaire n'est
    /// pas une section.
    pub(crate) fn section(&self, name: &str) -> bool {
        self.0
            .as_ref()
            .is_some_and(|document| document.get(name).is_some())
    }

    /// Valeur d'un champ, s'il est renseigné.
    pub(crate) fn field(&self, section: &str, key: &str) -> Option<String> {
        self.0.as_ref().and_then(|document| {
            document
                .get(section)
                .and_then(|table| table.get(key))
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn outside_an_rbs_project_nothing_is_diagnosed() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(error, Error::PasUnProjet));
    }

    #[test]
    fn a_report_without_a_failure_has_succeeded() {
        let report = Report {
            checks: vec![Check::ok("ancres", "les 5 sont en place")],
        };

        assert!(report.succeeded());
    }

    /// Un projet neuf, dont les features sont celles passées.
    ///
    /// `pub(super)` pour que les contrôles la réemploient : chaque module de `doctor/` qui
    /// s'en écrirait une copie ferait diverger la sienne du projet que `rbs new` produit.
    pub(super) fn project(features: &[&str]) -> (TempDir, std::path::PathBuf) {
        let (parent, root) = crate::fixtures::project();

        let manifest = root.join("Cargo.toml");
        let source = std::fs::read_to_string(&manifest).expect("manifeste lisible");
        let declarees = features
            .iter()
            .map(|feature| format!("\"{feature}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            &manifest,
            source.replace(
                "features = [\"health\"]",
                &format!("features = [{declarees}]"),
            ),
        )
        .expect("manifeste inscriptible");

        (parent, root)
    }

    fn titles(report: &Report) -> Vec<&'static str> {
        report.checks.iter().map(|c| c.title).collect()
    }

    #[test]
    fn a_project_without_auth_has_no_auth_check() {
        let (_parent, root) = project(&["health"]);

        let report = run(&root).expect("c'est un projet rbs");

        for title in ["auth", "gardes"] {
            assert!(
                !titles(&report).contains(&title),
                "un projet sans auth n'a pas à lire une ligne à son sujet : {:?}",
                titles(&report)
            );
        }
    }

    #[test]
    fn a_project_carrying_auth_receives_its_check() {
        let (_parent, root) = project(&["health", "auth"]);

        let report = run(&root).expect("c'est un projet rbs");

        for title in ["auth", "gardes"] {
            assert!(
                titles(&report).contains(&title),
                "la feature est déclarée, ses contrôles doivent figurer : {:?}",
                titles(&report)
            );
        }
    }

    #[test]
    fn a_project_without_the_v03_features_has_none_of_their_checks() {
        let (_parent, root) = project(&["health"]);

        let report = run(&root).expect("c'est un projet rbs");

        for feature in ["redis", "mail", "storage"] {
            assert!(
                !titles(&report).contains(&feature),
                "`{feature}` n'est pas installée, sa ligne n'a rien à faire au rapport : {:?}",
                titles(&report)
            );
        }
    }

    /// L'ordre du rapport est celui du tableau, et non celui du manifeste : deux projets
    /// portant les mêmes features se lisent pareil.
    #[test]
    fn the_three_v03_features_receive_their_checks_in_order() {
        let (_parent, root) = project(&["health", "storage", "mail", "redis"]);

        let report = run(&root).expect("c'est un projet rbs");

        let installes: Vec<&str> = titles(&report)
            .into_iter()
            .filter(|title| ["redis", "mail", "storage"].contains(title))
            .collect();

        assert_eq!(installes, vec!["redis", "mail", "storage"], "{installes:?}");
    }

    /// La feature ne se lit qu'au manifeste : un projet qui la déclare reçoit sa ligne,
    /// que sa configuration porte ou non la section correspondante.
    #[test]
    fn a_project_declaring_jobs_receives_its_check() {
        let (_parent, root) = project(&["health", "jobs"]);

        let report = run(&root).expect("c'est un projet rbs");

        assert!(
            titles(&report).contains(&"jobs"),
            "la feature est déclarée, son contrôle doit figurer : {:?}",
            titles(&report)
        );
    }

    #[test]
    fn a_single_failure_fails_the_report() {
        let report = Report {
            checks: vec![
                Check::ok("ancres", "les 5 sont en place"),
                Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV"),
            ],
        };

        assert!(!report.succeeded());
    }

    /// Un avertissement s'affiche sans faire échouer la commande : `rbs doctor` sort en 0
    /// sur un projet qui porte du code écrit à la main, ce que le guide autorise.
    #[test]
    fn a_warning_does_not_make_the_report_fail() {
        let report = Report {
            checks: vec![Check::warned("cli", "1 module hors CLI", "rien à faire")],
        };

        assert!(report.succeeded());
    }

    #[test]
    fn a_failure_still_makes_the_report_fail() {
        let report = Report {
            checks: vec![
                Check::warned("cli", "1 module hors CLI", "rien à faire"),
                Check::failed(".env", "RBS_ENV manque", "ajoutez-la"),
            ],
        };

        assert!(!report.succeeded());
    }
}
