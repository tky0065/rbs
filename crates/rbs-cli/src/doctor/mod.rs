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
pub mod json;
pub mod mail;
pub mod redis;
pub mod relations;
pub mod render;
pub mod storage;
pub mod versions;

use std::path::Path;

use serde::Serialize;

use crate::metadata;

/// Verdict d'un contrôle.
///
/// Les noms rendus en JSON sont ceux du dépôt, en ASCII : `ok` est déjà celui du
/// constructeur `Check::ok`, et les deux autres ceux des variantes ci-dessous. Un
/// troisième vocabulaire serait un de plus à tenir à jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    /// Rien à signaler.
    #[serde(rename = "ok")]
    Bon,
    /// Ce qui mérite d'être su sans empêcher le projet de fonctionner.
    Avertissement,
    /// Ce qui empêche le projet de fonctionner.
    Echec,
}

/// Ce qu'un contrôle a constaté.
///
/// Les noms des champs en JSON suivent la seule autre sortie structurée du dépôt, le
/// corps de `GET /health` de `rbs-core` : `status` y désigne déjà un verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Check {
    /// Ce qui est vérifié, en un mot : `anchors`, `.env`, `versions`, `base`.
    #[serde(rename = "name")]
    pub title: &'static str,
    /// Verdict.
    #[serde(rename = "status")]
    pub state: State,
    /// Ce qui a été constaté, en une ligne.
    pub detail: String,
    /// Quoi faire, quand il y a quelque chose à faire.
    #[serde(rename = "remede", skip_serializing_if = "Option::is_none")]
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

/// Ce qui reçoit le rapport au fil des contrôles.
///
/// Le diagnostic ne s'assemble plus avant de s'afficher : un contrôle qui va bloquer une
/// minute doit pouvoir le dire pendant que les précédents sont déjà à l'écran.
pub(crate) trait Sortie {
    /// Les titres de tous les contrôles prévus, avant que le premier ne s'exécute.
    ///
    /// La colonne des détails s'aligne sur le plus long d'entre eux, largeur qu'un rendu
    /// au fil de l'eau ne peut plus découvrir après coup.
    fn debut(&mut self, titres: &[&'static str]);

    /// Ce qu'un contrôle s'apprête à faire, quand cela va prendre du temps.
    fn annonce(&mut self, titre: &'static str, raison: &str);

    /// Le constat qui vient d'être fait.
    fn constat(&mut self, check: &Check);
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

/// Diagnostique le projet qui contient `directory`, en remettant chaque constat à
/// `sortie` au moment où il est fait.
pub(crate) fn run(directory: &Path, sortie: &mut dyn Sortie) -> Result<Report, Error> {
    let root = metadata::project_root(directory)?;

    let controles = plan(&root);
    let titres: Vec<&'static str> = controles.iter().map(|controle| controle.titre).collect();
    sortie.debut(&titres);

    // Une seule lecture pour toute la boucle : la configuration était relue et
    // réanalysée par chaque contrôle pour une question d'une ligne.
    let config = Config::read(&root);
    let mut checks = Vec::with_capacity(controles.len());

    for controle in controles {
        let check = {
            let mut annonce = |raison: &str| sortie.annonce(controle.titre, raison);
            (controle.executer)(&root, &config, &mut annonce)
        };

        sortie.constat(&check);
        checks.push(check);
    }

    Ok(Report { checks })
}

/// Un contrôle du diagnostic : son titre, connu avant qu'il ne s'exécute, et son
/// exécution.
///
/// Un contrôle qui n'interroge ni la configuration ni l'annonce les ignore par une
/// fermeture : lui imposer un paramètre qu'il n'emploie pas se lirait comme une
/// dépendance qu'il n'a pas.
#[derive(Clone, Copy)]
struct Controle {
    /// Ce qui est vérifié, tel qu'il paraîtra au rapport.
    titre: &'static str,
    /// Le contrôle lui-même.
    executer: Execution,
}

/// Ce que reçoit un contrôle : la racine du projet, la configuration lue une seule fois,
/// et de quoi annoncer ce qu'il s'apprête à faire.
type Execution = fn(&Path, &Config, &mut dyn FnMut(&str)) -> Check;

/// Les contrôles à jouer sur ce projet, dans l'ordre du rapport.
///
/// Le plan se construit avant le premier verdict : c'est ce qui permet à un rendu écrit
/// au fil de l'eau de connaître la largeur de sa colonne de titres.
fn plan(root: &Path) -> Vec<Controle> {
    let mut controles = vec![
        Controle {
            titre: anchors::TITRE,
            executer: |root, _, _| anchors::check(root),
        },
        Controle {
            titre: agents::TITRE,
            executer: |root, _, _| agents::check(root),
        },
        Controle {
            titre: relations::TITRE,
            executer: |root, _, _| relations::check(root),
        },
        Controle {
            titre: env::TITRE,
            executer: |root, _, _| env::check(root),
        },
        Controle {
            titre: versions::TITRE,
            executer: |root, _, _| versions::check(root),
        },
        Controle {
            titre: base::TITRE,
            executer: |root, _, annonce| base::check(root, annonce),
        },
    ];

    let installees = metadata::read(&root.join("Cargo.toml"))
        .map(|metadonnees| metadonnees.features)
        .unwrap_or_default();

    // Un projet qui n'a pas installé une feature n'a pas à lire une ligne à son sujet :
    // le rapport ne porte que des contrôles dont le verdict le concerne.
    for (feature, controle) in FEATURE_CHECKS {
        if installees.iter().any(|installee| installee == feature) {
            controles.push(controle);
        }
    }

    controles
}

/// Le contrôle propre à chaque feature, sous le nom qu'elle porte dans le manifeste.
///
/// `redis` s'installe en `src/cache/` sous une section `[cache]` : c'est le nom de la
/// crate d'un côté, celui du service rendu de l'autre. Le tableau porte le nom déclaré,
/// seul commun aux quatre.
///
/// Une feature peut y figurer deux fois : `auth` amène de quoi vérifier son secret, et de
/// quoi juger les routes que les rôles qu'elle installe pourraient protéger.
const FEATURE_CHECKS: [(&str, Controle); 6] = [
    (
        "auth",
        Controle {
            titre: auth::TITRE,
            executer: |root, config, _| auth::check(root, config),
        },
    ),
    (
        "auth",
        Controle {
            titre: guards::TITRE,
            executer: |root, _, _| guards::check(root),
        },
    ),
    (
        "redis",
        Controle {
            titre: redis::TITRE,
            executer: |_, config, _| redis::check(config),
        },
    ),
    (
        "mail",
        Controle {
            titre: mail::TITRE,
            executer: |root, config, _| mail::check(root, config),
        },
    ),
    (
        "storage",
        Controle {
            titre: storage::TITRE,
            executer: |root, config, _| storage::check(root, config),
        },
    ),
    (
        "jobs",
        Controle {
            titre: jobs::TITRE,
            executer: |_, config, _| jobs::check(config),
        },
    ),
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

    /// Le diagnostic, sans rien afficher : ces tests jugent le rapport, pas son rendu.
    fn run_with(root: &std::path::Path, sortie: &mut dyn Sortie) -> Result<Report, Error> {
        super::run(root, sortie)
    }

    /// Un puits qui laisse tomber ce qu'il reçoit.
    struct Muet;

    impl Sortie for Muet {
        fn debut(&mut self, _titres: &[&'static str]) {}

        fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

        fn constat(&mut self, _check: &Check) {}
    }

    /// Un puits qui note ce qu'il reçoit, dans l'ordre où il le reçoit.
    struct Journal {
        titres: Vec<&'static str>,
        constats: Vec<&'static str>,
    }

    impl Sortie for Journal {
        fn debut(&mut self, titres: &[&'static str]) {
            self.titres = titres.to_vec();
        }

        fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

        fn constat(&mut self, check: &Check) {
            self.constats.push(check.title);
        }
    }

    #[test]
    fn outside_an_rbs_project_nothing_is_diagnosed() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run_with(ailleurs.path(), &mut Muet).expect_err("ce n'est pas un projet");

        assert!(matches!(error, Error::PasUnProjet));
    }

    /// Les titres sont connus avant le premier verdict — c'est ce qui fixe la largeur de
    /// la colonne sans attendre le dernier — et chaque constat est remis au fil de l'eau.
    #[test]
    fn the_sink_learns_every_title_before_the_first_finding() {
        let (_parent, root) = project(&["health", "jobs"]);
        let mut journal = Journal {
            titres: Vec::new(),
            constats: Vec::new(),
        };

        let report = run_with(&root, &mut journal).expect("c'est un projet rbs");

        assert_eq!(journal.titres, titles(&report));
        assert_eq!(journal.constats, titles(&report));
    }

    #[test]
    fn a_report_without_a_failure_has_succeeded() {
        let report = Report {
            checks: vec![Check::ok("ancres", "les 5 sont en place")],
        };

        assert!(report.succeeded());
    }

    /// Le projet de `crate::fixtures::project`, dont le manifeste est réécrit pour ne
    /// déclarer que les `features` passées.
    ///
    /// `pub(super)` : `doctor/guards.rs` est son seul réemploi hors de ce module.
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

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

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

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

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

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

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

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

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

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

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
