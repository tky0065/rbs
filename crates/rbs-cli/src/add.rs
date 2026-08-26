//! `rbs add <feature>` : le fragment d'une feature déposé dans un projet existant.
//!
//! Rien de propre à `docker` ou à `ci` n'est écrit ici. Le catalogue d'une feature est son
//! répertoire sous `templates/features`, et son contexte de rendu se déduit du projet
//! visé : ajouter une feature qui n'apporte pas de code Rust, c'est ajouter un répertoire.
//!
//! La séquence est celle de `generate` — racine, garde Git, plan, application — pour la
//! même raison : ce qui modifie un projet existant se montre avant de s'écrire, et
//! s'écrit en entier ou pas du tout.

use std::io;
use std::path::{Path, PathBuf};

use minijinja::context;

use crate::git;
use crate::metadata;
use crate::plan;
use crate::template::Renderer;
use crate::templates::{self, Source};

/// Ce qu'il faut savoir pour installer une feature.
pub(crate) struct Options {
    /// Nom de la feature, tel que le sous-répertoire de `templates/features` la nomme.
    pub feature: String,
    /// Répertoire d'où la commande est lancée.
    pub repertoire: PathBuf,
    /// Installe même si le projet porte des modifications non commitées.
    pub force: bool,
    /// Répertoire de templates remplaçant celles embarquées.
    pub template_dir: Option<PathBuf>,
}

/// Ce qu'une installation fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planifiee {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Chemins des fichiers de la feature, relatifs à la racine du projet.
    pub fichiers: Vec<String>,
}

/// Ce qui peut empêcher d'installer une feature.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Aucun fragment ne porte ce nom.
    #[error("{0}")]
    Inconnue(#[from] templates::Inconnue),

    /// Un fichier du projet ou une template n'a pu être lu.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif.
        chemin: String,
        /// Cause système.
        source: io::Error,
    },

    /// Une template ne s'est pas rendue.
    #[error("{fichier} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        fichier: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// Le projet porte des modifications non commitées, qu'une installation rendrait
    /// indiscernables des siennes.
    #[error("le working tree n'est pas propre : {fichiers} — commitez, ou relancez avec --force")]
    WorkingTreeSale {
        /// Fichiers suivis modifiés, énumérés.
        fichiers: String,
    },

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadonnees(#[from] metadata::Erreur),

    /// Le plan de l'installation n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Erreur),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Erreur),
}

/// Calcule ce que l'installation de `options` ferait au projet, sans rien écrire.
pub(crate) fn planifier(options: &Options) -> Result<Planifiee, Erreur> {
    let depart = options
        .repertoire
        .canonicalize()
        .map_err(|source| acces(&options.repertoire, source))?;
    let racine = metadata::racine_du_projet(&depart).ok_or(Erreur::PasUnProjet)?;

    if !options.force {
        let modifies = git::fichiers_modifies(&racine);
        if !modifies.is_empty() {
            return Err(Erreur::WorkingTreeSale {
                fichiers: git::enumerer(&modifies),
            });
        }
    }

    let source = Source::feature(options.template_dir.as_deref(), &options.feature)?;
    let nom_projet = metadata::nom_du_paquet(&racine.join("Cargo.toml"))?;
    let contexte = context! {
        nom_projet => nom_projet.clone(),
        nom_crate => nom_projet.replace('-', "_"),
    };

    let renderer = Renderer::new();
    let mut constructeur = plan::Constructeur::nouveau(racine);
    let mut fichiers = Vec::new();

    for fichier in source.fichiers().map_err(|source| Erreur::Acces {
        chemin: options.feature.clone(),
        source,
    })? {
        let chemin = fichier.destination.to_string_lossy().into_owned();
        let contenu = renderer
            .rendre(&fichier.source, contexte.clone())
            .map_err(|source| Erreur::Rendu {
                fichier: chemin.clone(),
                source,
            })?;

        constructeur.creer(&chemin, &contenu)?;
        fichiers.push(chemin);
    }

    constructeur.patcher(plan::PatchToml::InscrireFeature(options.feature.clone()))?;

    Ok(Planifiee {
        plan: constructeur.finir(),
        fichiers,
    })
}

fn acces(chemin: &Path, source: io::Error) -> Erreur {
    Erreur::Acces {
        chemin: chemin.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::plan::Statut;

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    fn empreinte(racine: &Path) -> BTreeMap<PathBuf, String> {
        let mut vue = BTreeMap::new();
        let mut a_visiter = vec![racine.to_path_buf()];

        while let Some(repertoire) = a_visiter.pop() {
            for entree in fs::read_dir(&repertoire).expect("le répertoire se lit") {
                let chemin = entree.expect("l'entrée se lit").path();
                let relatif = chemin
                    .strip_prefix(racine)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();

                if chemin.is_dir() {
                    vue.insert(relatif, String::new());
                    a_visiter.push(chemin);
                } else {
                    vue.insert(relatif, fs::read_to_string(&chemin).unwrap_or_default());
                }
            }
        }

        vue
    }

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
    fn projet() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let projet = crate::new::creer(
            &crate::new::Options {
                nom: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, projet.racine)
    }

    fn options(racine: &Path, feature: &str) -> Options {
        Options {
            feature: feature.to_string(),
            repertoire: racine.to_path_buf(),
            force: false,
            template_dir: None,
        }
    }

    /// Planifie puis applique, comme la commande le fait.
    fn executer(options: &Options) -> Result<Planifiee, Erreur> {
        let planifiee = planifier(options)?;
        crate::plan::application::appliquer(&planifiee.plan, options.force)?;

        Ok(planifiee)
    }

    /// Fait du projet un dépôt dont tout est commité.
    fn commiter(racine: &Path) {
        for arguments in [
            vec!["config", "user.email", "rbs@example.test"],
            vec!["config", "user.name", "rbs"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "projet neuf"],
        ] {
            let sortie = std::process::Command::new("git")
                .args(&arguments)
                .current_dir(racine)
                .output()
                .expect("git doit être lançable");

            assert!(
                sortie.status.success(),
                "git {arguments:?} a échoué :\n{}",
                String::from_utf8_lossy(&sortie.stderr)
            );
        }
    }

    /// Le contenu qu'un plan projette pour `chemin`.
    fn projete<'plan>(planifiee: &'plan Planifiee, chemin: &str) -> &'plan str {
        &planifiee
            .plan
            .fichiers()
            .iter()
            .find(|fichier| fichier.chemin == chemin)
            .unwrap_or_else(|| panic!("{chemin} absent du plan"))
            .apres
    }

    #[test]
    fn le_plan_de_docker_cree_ses_trois_fichiers_et_inscrit_la_feature() {
        let (_parent, racine) = projet();

        let planifiee = planifier(&options(&racine, "docker")).expect("le plan doit se calculer");

        assert_eq!(
            planifiee.fichiers,
            [".dockerignore", "Dockerfile", "docker-compose.yml"]
        );

        let manifeste = projete(&planifiee, "Cargo.toml");
        assert!(
            manifeste.contains("features = [\"health\", \"docker\"]"),
            "la feature n'est pas inscrite dans le manifeste projeté :\n{manifeste}"
        );
    }

    #[test]
    fn planifier_ne_modifie_pas_le_repertoire_du_projet() {
        let (_parent, racine) = projet();
        let avant = empreinte(&racine);

        planifier(&options(&racine, "docker")).expect("le plan doit se calculer");

        assert_eq!(empreinte(&racine), avant);
    }

    #[test]
    fn relancer_sur_un_projet_deja_dockerise_donne_un_plan_sans_effet() {
        let (_parent, racine) = projet();
        executer(&options(&racine, "docker")).expect("la première pose doit aboutir");

        let planifiee = planifier(&options(&racine, "docker")).expect("le plan doit se recalculer");

        for fichier in planifiee.plan.fichiers() {
            assert_eq!(
                fichier.statut,
                Statut::DejaFait,
                "{} n'est pas sans effet",
                fichier.chemin
            );
        }
    }

    #[test]
    fn hors_d_un_projet_rbs_la_commande_refuse() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let erreur = planifier(&options(ailleurs.path(), "docker"))
            .expect_err("un répertoire quelconque n'est pas un projet rbs");

        assert!(matches!(erreur, Erreur::PasUnProjet), "{erreur}");
    }

    #[test]
    fn un_working_tree_sale_refuse_sans_force_et_passe_avec() {
        let (_parent, racine) = projet();
        commiter(&racine);
        fs::write(racine.join("src/main.rs"), "// modifié").expect("le fichier est écrivable");

        let erreur = planifier(&options(&racine, "docker"))
            .expect_err("un projet sale ne se modifie pas en silence");
        assert!(matches!(erreur, Erreur::WorkingTreeSale { .. }), "{erreur}");

        let mut forcees = options(&racine, "docker");
        forcees.force = true;
        planifier(&forcees).expect("--force doit passer outre");
    }

    #[test]
    fn une_feature_inconnue_est_refusee_en_nommant_celles_qui_existent() {
        let (_parent, racine) = projet();

        let erreur =
            planifier(&options(&racine, "auth")).expect_err("`auth` n'est pas installable en v0.1");

        assert!(matches!(erreur, Erreur::Inconnue(_)), "{erreur}");
        assert!(
            erreur.to_string().contains("docker"),
            "le message n'oriente pas vers ce qui existe : {erreur}"
        );
    }

    #[test]
    fn le_compose_projete_nomme_la_base_du_projet_et_ouvre_l_hote() {
        let (_parent, racine) = projet();

        let planifiee = planifier(&options(&racine, "docker")).expect("le plan doit se calculer");
        let compose = projete(&planifiee, "docker-compose.yml");

        // Le défaut de `config/default.toml` est 127.0.0.1 : sans cette variable, l'API
        // conteneurisée n'est joignable depuis nulle part.
        assert!(
            compose.contains("RBS_SERVER__HOST: 0.0.0.0"),
            "le compose n'ouvre pas l'hôte :\n{compose}"
        );
        assert!(
            compose.contains("POSTGRES_DB: demo_api"),
            "le compose ne nomme pas la base du projet :\n{compose}"
        );
    }
}
