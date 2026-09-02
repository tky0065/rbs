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
pub mod observability;
pub mod redis;
pub mod relations;
pub mod render;
pub mod storage;
pub mod versions;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::metadata;

/// Verdict d'un contrôle.
///
/// Les noms rendus en JSON sont en ASCII, et sont ceux qu'un script lit : `ok`,
/// `avertissement`, `erreur`. Le dernier ne reprend pas le nom de sa variante — un échec
/// de contrôle *est* une erreur du point de vue de qui exploite le document, et c'est le
/// mot que porte le contrat publié.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    /// Rien à signaler.
    #[serde(rename = "ok")]
    Bon,
    /// Ce qui mérite d'être su sans empêcher le projet de fonctionner.
    Avertissement,
    /// Ce qui empêche le projet de fonctionner.
    #[serde(rename = "erreur")]
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
    let metadata::Racine { root, manifeste } = metadata::racine(directory)?;

    // Une seule lecture de chaque fichier pour tout le diagnostic : le manifeste comme
    // la configuration étaient relus et réanalysés par chaque contrôle pour une question
    // d'une ligne. Le manifeste vient de la remontée elle-même, qui a dû l'ouvrir pour
    // reconnaître la racine.
    let projet = Projet {
        config: Config::read(&root),
        manifeste: Ok(manifeste),
        root,
    };

    let controles = plan(&projet.manifeste);
    let titres: Vec<&'static str> = controles.iter().map(|controle| controle.titre).collect();
    sortie.debut(&titres);

    let mut checks = Vec::with_capacity(controles.len());

    for controle in controles {
        let check = {
            let mut annonce = |raison: &str| sortie.annonce(controle.titre, raison);
            (controle.executer)(&projet, &mut annonce)
        };

        sortie.constat(&check);
        checks.push(check);
    }

    Ok(Report { checks })
}

/// Le projet diagnostiqué, tel que chaque contrôle le reçoit.
pub(crate) struct Projet {
    /// Racine du projet.
    pub root: PathBuf,
    /// `config/default.toml`, lu et analysé une seule fois.
    pub config: Config,
    /// `Cargo.toml`, lu et analysé une seule fois.
    pub manifeste: Manifeste,
}

/// Ce que le manifeste du projet apprend au diagnostic, ou la faute qui l'en empêche.
///
/// Le document analysé y voyage avec les métadonnées : `versions` lit la dépendance au
/// noyau et `base` la feature de `sea-orm`, deux déclarations que
/// `[package.metadata.rbs]` ne dit pas et que chacun rouvrait le fichier pour trouver.
///
/// La faute est gardée plutôt que remplacée par un défaut : le moteur, les features et
/// la version que le manifeste porte commandent la moitié des contrôles, et chacun a sa
/// façon de dire qu'il ne les a pas.
pub(crate) type Manifeste = Result<metadata::Manifeste, metadata::Error>;

/// Lit le manifeste du projet enraciné en `root`.
///
/// Le diagnostic ne passe plus par là — `metadata::racine` lui rend le manifeste qu'elle
/// a dû lire pour reconnaître la racine. Restent les tests des contrôles, qui réécrivent
/// le manifeste entre la création du projet et le verdict, et doivent donc le relire à
/// cet instant-là.
#[cfg(test)]
pub(crate) fn manifeste(root: &Path) -> Manifeste {
    metadata::manifeste(&root.join("Cargo.toml"))
}

/// Un contrôle du diagnostic : son titre, connu avant qu'il ne s'exécute, et son
/// exécution.
///
/// Un contrôle qui n'interroge pas l'annonce l'ignore par une fermeture : lui imposer un
/// paramètre qu'il n'emploie pas se lirait comme une dépendance qu'il n'a pas.
#[derive(Clone, Copy)]
struct Controle {
    /// Ce qui est vérifié, tel qu'il paraîtra au rapport.
    titre: &'static str,
    /// Le contrôle lui-même.
    executer: Execution,
}

/// Ce que reçoit un contrôle : le projet, lu une seule fois pour tous, et de quoi
/// annoncer ce qu'il s'apprête à faire.
type Execution = fn(&Projet, &mut dyn FnMut(&str)) -> Check;

/// Les contrôles à jouer sur ce projet, dans l'ordre du rapport.
///
/// Le plan se construit avant le premier verdict : c'est ce qui permet à un rendu écrit
/// au fil de l'eau de connaître la largeur de sa colonne de titres.
fn plan(manifeste: &Manifeste) -> Vec<Controle> {
    let mut controles = vec![
        Controle {
            titre: anchors::TITRE,
            executer: |projet, _| anchors::check(&projet.root),
        },
        Controle {
            titre: agents::TITRE,
            executer: |projet, _| agents::check(&projet.root, &projet.manifeste),
        },
        Controle {
            titre: relations::TITRE,
            executer: |projet, _| relations::check(&projet.root),
        },
        Controle {
            titre: env::TITRE,
            executer: |projet, _| env::check(&projet.root),
        },
        Controle {
            titre: versions::TITRE,
            executer: |projet, _| versions::check(&projet.manifeste),
        },
        Controle {
            titre: base::TITRE,
            executer: |projet, annonce| base::check(&projet.root, &projet.manifeste, annonce),
        },
    ];

    let installees = manifeste
        .as_ref()
        .map(|manifeste| manifeste.metadonnees.features.as_slice())
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
const FEATURE_CHECKS: [(&str, Controle); 7] = [
    (
        "auth",
        Controle {
            titre: auth::TITRE,
            executer: |projet, _| auth::check(&projet.root, &projet.config),
        },
    ),
    (
        "auth",
        Controle {
            titre: guards::TITRE,
            executer: |projet, _| guards::check(&projet.root),
        },
    ),
    (
        "redis",
        Controle {
            titre: redis::TITRE,
            executer: |projet, _| redis::check(&projet.config),
        },
    ),
    (
        "mail",
        Controle {
            titre: mail::TITRE,
            executer: |projet, _| mail::check(&projet.root, &projet.config),
        },
    ),
    (
        "storage",
        Controle {
            titre: storage::TITRE,
            executer: |projet, _| storage::check(&projet.root, &projet.config),
        },
    ),
    (
        "jobs",
        Controle {
            titre: jobs::TITRE,
            executer: |projet, _| jobs::check(&projet.config),
        },
    ),
    (
        "observability",
        Controle {
            titre: observability::TITRE,
            executer: |projet, _| observability::check(&projet.config),
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
    match defaut_de_section(config, section, reglages) {
        Some((detail, remede)) => Check::failed(titre, detail, remede),
        None => Check::ok(titre, present),
    }
}

/// Ce qui empêche `section` d'être en place, et le geste qui le corrige.
///
/// Trois fautes, et non une : le fichier peut n'être pas là, n'être pas analysable, ou
/// ne pas porter la section. Les confondre faisait annoncer une section manquante d'un
/// fichier qui n'avait pas pu être lu, et proposer d'y ajouter ce qui s'y trouvait déjà.
///
/// `reglages` ne sert qu'aux deux cas où le remède est d'écrire la section : un fichier
/// mal formé se corrige avant qu'on y ajoute quoi que ce soit.
fn defaut_de_section(config: &Config, section: &str, reglages: &str) -> Option<(String, String)> {
    match config {
        Config::Fautif { detail, remede } => Some((detail.clone(), remede.clone())),
        Config::Absent => Some((
            format!("{CONFIG} est absent"),
            format!("créez {CONFIG} avec :\n[{section}]\n{reglages}"),
        )),
        Config::Lu(document) if document.get(section).is_none() => Some((
            format!("{CONFIG} ne porte pas de section `[{section}]`"),
            format!("ajoutez à {CONFIG} :\n[{section}]\n{reglages}"),
        )),
        Config::Lu(_) => None,
    }
}

/// `config/default.toml` du projet, lu et analysé une seule fois.
///
/// Un diagnostic complet interrogeait ce fichier jusqu'à huit fois, chaque contrôle le
/// relisant et le réanalysant pour une question d'une ligne — `storage` en enchaînait
/// trois d'affilée.
pub(crate) enum Config {
    /// Le document, analysé.
    Lu(toml_edit::DocumentMut),
    /// Aucun fichier à cet emplacement.
    Absent,
    /// Le fichier est là, mais illisible ou mal formé : ce qui a été constaté, et le
    /// geste qui le corrige, tels qu'un contrôle les rendra.
    Fautif {
        /// Ce qui a empêché d'analyser le fichier.
        detail: String,
        /// Quoi faire pour qu'il s'analyse.
        remede: String,
    },
}

impl Config {
    /// Lit la configuration du projet.
    ///
    /// Les trois issues restent distinctes jusqu'au rapport : un fichier qu'on n'a pas su
    /// analyser n'apprend rien sur les sections qu'il porte, et le tenir pour vide faisait
    /// dire au diagnostic l'inverse de la panne.
    pub(crate) fn read(root: &Path) -> Self {
        let source = match std::fs::read_to_string(root.join(CONFIG)) {
            Ok(source) => source,
            Err(faute) if faute.kind() == std::io::ErrorKind::NotFound => return Self::Absent,
            Err(faute) => {
                return Self::Fautif {
                    detail: format!("{CONFIG} est inaccessible : {faute}"),
                    remede: format!(
                        "rendez {CONFIG} lisible : aucun réglage de feature ne se \
                         diagnostique sans lui"
                    ),
                };
            }
        };

        match source.parse::<toml_edit::DocumentMut>() {
            Ok(document) => Self::Lu(document),
            // Seule la première ligne du report de `toml_edit` : elle porte la position
            // fautive, les suivantes n'en sont que le soulignement, et un constat tient
            // sur une ligne au rapport.
            Err(faute) => Self::Fautif {
                detail: format!(
                    "{CONFIG} n'est pas un TOML valide : {}",
                    faute.to_string().lines().next().unwrap_or_default()
                ),
                remede: format!("corrigez la syntaxe de {CONFIG}, puis relancez le diagnostic"),
            },
        }
    }

    /// Le document, quand il a pu être analysé.
    fn document(&self) -> Option<&toml_edit::DocumentMut> {
        match self {
            Self::Lu(document) => Some(document),
            Self::Absent | Self::Fautif { .. } => None,
        }
    }

    /// Vrai si la configuration porte une section `[name]`.
    ///
    /// Analysé par `toml_edit` et non cherché en texte : une section en commentaire n'est
    /// pas une section.
    pub(crate) fn section(&self, name: &str) -> bool {
        self.document()
            .is_some_and(|document| document.get(name).is_some())
    }

    /// Valeur entière d'un champ, s'il est renseigné et qu'il en porte une.
    ///
    /// Un port écrit entre guillemets n'en est pas un : la cascade de configuration le
    /// refuserait au démarrage, et le contrôle qui le lirait comme un entier jugerait
    /// une valeur que le projet n'a jamais eue.
    pub(crate) fn integer(&self, section: &str, key: &str) -> Option<i64> {
        self.document().and_then(|document| {
            document
                .get(section)
                .and_then(|table| table.get(key))
                .and_then(toml_edit::Item::as_integer)
        })
    }

    /// Valeur d'un champ, s'il est renseigné.
    pub(crate) fn field(&self, section: &str, key: &str) -> Option<String> {
        self.document().and_then(|document| {
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

    /// Le contrôle de section, tel que `redis` et `jobs` l'appellent.
    fn section(root: &std::path::Path) -> Check {
        section_check(
            &Config::read(root),
            "jobs",
            "jobs",
            "la configuration de la file est en place",
            "max_attempts = 5",
        )
    }

    /// Un `config/default.toml` mal formé n'est pas un fichier absent : le rapport
    /// annonçait une section manquante et proposait d'ajouter ce qui s'y trouvait déjà.
    #[test]
    fn a_malformed_config_names_its_syntax_error() {
        let (_parent, root) = project(&["health", "jobs"]);
        std::fs::write(root.join(CONFIG), "[jobs\nmax_attempts = 5\n").expect("config cassée");

        let check = section(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("TOML valide"), "{}", check.detail);
        assert!(
            !check.detail.contains("ne porte pas de section"),
            "le remède mentirait sur la panne : {}",
            check.detail
        );
    }

    /// Un fichier absent se nomme comme tel : la section n'y manque pas, c'est le fichier
    /// entier qui manque.
    #[test]
    fn an_absent_config_names_the_missing_file() {
        let (_parent, root) = project(&["health", "jobs"]);
        std::fs::remove_file(root.join(CONFIG)).expect("config supprimable");

        let check = section(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("est absent"), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_ref()
                .is_some_and(|remede| remede.contains("[jobs]")),
            "{:?}",
            check.remedy
        );
    }

    /// Un fichier lisible qui ne porte pas la section garde le verdict qu'il a toujours
    /// eu : distinguer les trois cas n'en efface aucun.
    #[test]
    fn a_readable_config_without_the_section_still_says_so() {
        let (_parent, root) = project(&["health", "jobs"]);

        let check = section(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("ne porte pas de section"),
            "{}",
            check.detail
        );
    }

    /// La faute traverse le rapport entier : chaque contrôle de feature la nomme, au lieu
    /// de réclamer huit fois une section qu'il n'a pas pu chercher.
    #[test]
    fn a_malformed_config_is_named_by_every_feature_check() {
        let (_parent, root) = project(&["health", "jobs", "redis", "observability"]);
        std::fs::write(root.join(CONFIG), "[jobs\nmax_attempts = 5\n").expect("config cassée");

        let report = run_with(&root, &mut Muet).expect("c'est un projet rbs");

        for titre in ["jobs", "redis", "observability"] {
            let check = report
                .checks
                .iter()
                .find(|check| check.title == titre)
                .unwrap_or_else(|| panic!("{titre} doit figurer au rapport"));

            assert!(
                check.detail.contains("TOML valide"),
                "{titre} : {}",
                check.detail
            );
        }
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
