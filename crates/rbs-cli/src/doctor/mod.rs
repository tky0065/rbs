//! `rbs doctor` : diagnostic d'un projet généré.
//!
//! Chaque contrôle est indépendant et rend son verdict sans interrompre les autres : un
//! diagnostic qui s'arrête au premier problème oblige à le relancer autant de fois qu'il
//! y a de problèmes.

pub mod anchors;
pub mod auth;
pub mod base;
pub mod env;
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
}

/// L'ensemble des constats, dans l'ordre où ils ont été faits.
#[derive(Debug)]
pub(crate) struct Report {
    /// Les contrôles, tous exécutés.
    pub checks: Vec<Check>,
}

impl Report {
    /// Vrai si aucun contrôle n'a échoué.
    pub(crate) fn succeeded(&self) -> bool {
        self.checks.iter().all(|c| c.state == State::Bon)
    }
}

/// Ce qui peut empêcher de diagnostiquer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error(
        "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici"
    )]
    PasUnProjet,
}

/// Diagnostique le projet qui contient `directory`.
pub(crate) fn run(directory: &Path) -> Result<Report, Error> {
    let root = metadata::project_root(directory).ok_or(Error::PasUnProjet)?;

    let mut checks = vec![
        anchors::check(&root),
        relations::check(&root),
        env::check(&root),
        versions::check(&root),
        base::check(&root),
    ];

    // Un projet qui n'a pas installé une feature n'a pas à lire une ligne à son sujet :
    // le rapport ne porte que des contrôles dont le verdict le concerne.
    for (feature, check) in FEATURE_CHECKS {
        if installed_feature(&root, feature) {
            checks.push(check(&root));
        }
    }

    Ok(Report { checks })
}

/// Une feature, sous le nom qu'elle porte dans le manifeste, et le contrôle qui la juge.
type FeatureCheck = (&'static str, fn(&Path) -> Check);

/// Le contrôle propre à chaque feature, sous le nom qu'elle porte dans le manifeste.
///
/// `redis` s'installe en `src/cache/` sous une section `[cache]` : c'est le nom de la
/// crate d'un côté, celui du service rendu de l'autre. Le tableau porte le nom déclaré,
/// seul commun aux quatre.
const FEATURE_CHECKS: [FeatureCheck; 5] = [
    ("auth", auth::check),
    ("redis", redis::check),
    ("mail", mail::check),
    ("storage", storage::check),
    ("jobs", jobs::check),
];

/// Vrai si `config/default.toml` porte une section `[name]`.
///
/// Lu par `toml_edit` et non par recherche de texte : une section en commentaire n'est
/// pas une section.
fn section(root: &Path, name: &str) -> bool {
    std::fs::read_to_string(root.join("config/default.toml"))
        .ok()
        .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
        .is_some_and(|document| document.get(name).is_some())
}

/// Valeur d'un champ de `config/default.toml`, s'il est renseigné.
///
/// Rend `None` aussi bien pour une section absente que pour un champ absent : ce qui
/// intéresse un contrôle est de disposer ou non de la valeur, jamais laquelle des deux
/// couches manque.
fn field(root: &Path, section: &str, key: &str) -> Option<String> {
    std::fs::read_to_string(root.join("config/default.toml"))
        .ok()
        .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|document| {
            document
                .get(section)
                .and_then(|table| table.get(key))
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
}

/// Vrai si `name` figure dans `[package.metadata.rbs].features`.
fn installed_feature(root: &Path, name: &str) -> bool {
    metadata::read(&root.join("Cargo.toml"))
        .is_ok_and(|metadonnees| metadonnees.features.iter().any(|feature| feature == name))
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
    fn project(features: &[&str]) -> (TempDir, std::path::PathBuf) {
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

        let manifest = project.root.join("Cargo.toml");
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

        (parent, project.root)
    }

    fn titles(report: &Report) -> Vec<&'static str> {
        report.checks.iter().map(|c| c.title).collect()
    }

    #[test]
    fn a_project_without_auth_has_no_auth_check() {
        let (_parent, root) = project(&["health"]);

        let report = run(&root).expect("c'est un projet rbs");

        assert!(
            !titles(&report).contains(&"auth"),
            "un projet sans auth n'a pas à lire une ligne à son sujet : {:?}",
            titles(&report)
        );
    }

    #[test]
    fn a_project_carrying_auth_receives_its_check() {
        let (_parent, root) = project(&["health", "auth"]);

        let report = run(&root).expect("c'est un projet rbs");

        assert!(
            titles(&report).contains(&"auth"),
            "la feature est déclarée, son contrôle doit figurer : {:?}",
            titles(&report)
        );
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
}
