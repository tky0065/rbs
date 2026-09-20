//! `rbs remove <feature>` : défait ce que `rbs add` a posé.
//!
//! Le parcours inverse du manifeste, section par section, vit dans [`desinstallation`].
//! Ce module porte ce qui l'entoure : les refus qui gardent l'entrée de la commande, et
//! rien ne s'écrit tant que l'un d'eux tombe. Un nom qui n'est pas un fragment, un
//! dépendant installé qui l'exige encore, un working tree sale — le quatrième refus, la
//! divergence d'un fichier modifié à la main, ne se calcule pas ici : il naît du
//! `Status::Conflit` que `Builder::supprimer` pose, et c'est `plan::application::apply`
//! qui le refuse, comme pour toute autre commande.

pub(crate) mod desinstallation;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::errors::Codee;
use crate::git;
use crate::manifest;
use crate::metadata;
use crate::migrate;
use crate::plan;
use crate::templates;

/// Ce qu'il faut savoir pour retirer une feature.
pub(crate) struct Options {
    /// Feature à retirer, telle que son répertoire de fragment la nomme.
    pub feature: String,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Retire même si le working tree Git est sale.
    pub force: bool,
    /// Répertoire de templates remplaçant celles embarquées.
    pub template_dir: Option<PathBuf>,
}

/// Ce qu'un retrait fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Ce que le fragment retiré déclarait installer, tel que son manifeste le décrit.
    pub description: String,
    /// Le fragment retirait une migration.
    ///
    /// Un `bool`, et non le chemin qu'`desinstallation::Retires` retrouve : le rapport
    /// n'a besoin que de savoir s'il faut avertir que le schéma garde ses tables. Les deux
    /// types divergent à dessein.
    pub migration: bool,
    /// Ce que le retrait a sciemment laissé en place, à nommer dans le rapport.
    pub laissees: Vec<String>,
    /// Le projet n'inscrivait déjà pas cette feature : le plan est vide et rien ne sera
    /// écrit.
    pub deja_absente: bool,
    /// La zone de l'`AGENTS.md` que le projet ne porte pas, s'il en manque une.
    ///
    /// Symétrique de celle d'`add::Planned` : la section 8 retire la feature du
    /// manifeste, et l'inventaire doit dire la même chose — sans quoi `rbs doctor`
    /// verrait diverger un projet qui vient pourtant de subir un retrait propre.
    pub zone_manquante: Option<crate::agents::MissingZone>,
}

/// Ce qui peut empêcher de retirer une feature.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs remove` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le nom ne correspond à aucun fragment embarqué portant un manifeste.
    ///
    /// Deux cas s'y confondent volontairement : une faute de frappe, et un CRUD engendré
    /// par `rbs generate crud`, que `[package.metadata.rbs] features` inscrit sans le
    /// distinguer d'un vrai fragment.
    #[error("`{feature}` n'est pas un fragment : {known}")]
    PasUnFragment {
        /// Feature demandée.
        feature: String,
        /// Les fragments valides, énumérés.
        known: String,
    },

    /// Un autre fragment installé exige encore celui-ci, directement ou par ricochet.
    #[error(
        "`{feature}` est encore exigée par {dependants} : retirez-les d'abord, dans l'ordre \
         de votre choix"
    )]
    Exigee {
        /// Feature dont le retrait est refusé.
        feature: String,
        /// Les dépendants, énumérés.
        dependants: String,
    },

    /// Un fichier du projet, ou le manifeste d'un fragment installé ailleurs, n'a pas pu
    /// être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le manifeste d'un fragment ne se lit pas.
    #[error("{0}")]
    Manifest(#[from] manifest::Error),

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Le projet porte des modifications non commitées, qu'un retrait rendrait
    /// indiscernables des siennes.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    /// Le `.env` du projet est là, mais illisible.
    ///
    /// Rejouer le rendu d'un fragment exige le contexte qu'`add` avait construit pour
    /// l'installer — l'inventer plutôt que de dire la panne ferait comparer le disque à un
    /// rendu que rien n'a jamais produit.
    #[error("{0}")]
    Env(#[from] migrate::Error),

    /// L'URL du projet ne se décompose pas, et le rendu comparé au disque s'appuie
    /// dessus.
    #[error(
        "RBS_DATABASE__URL ne se décompose pas : {url} — encodez les caractères réservés \
         du mot de passe (`/` en %2F, `?` en %3F, `#` en %23, `@` en %40)"
    )]
    UrlIndecomposable {
        /// L'URL, telle que le `.env` du projet la porte.
        url: String,
    },

    /// Le parcours inverse du manifeste n'a pas pu être planifié.
    #[error("{0}")]
    Desinstallation(#[from] desinstallation::Error),

    /// L'inventaire de l'`AGENTS.md` n'a pu être rafraîchi.
    #[error("{0}")]
    Inventaire(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

impl From<crate::contexte::Erreur> for Error {
    fn from(faute: crate::contexte::Erreur) -> Self {
        match faute {
            crate::contexte::Erreur::Env(source) => Error::Env(source),
            crate::contexte::Erreur::UrlIndecomposable { url } => Error::UrlIndecomposable { url },
        }
    }
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Error {
    /// Ce que le développeur peut coller ou déplacer pour réparer, quand la panne se
    /// répare ainsi.
    pub(crate) fn remedy(&self) -> Option<String> {
        self.plan()?.remede()
    }

    /// L'erreur de planification que celle-ci porte, par le parcours inverse du
    /// manifeste.
    fn plan(&self) -> Option<&plan::Error> {
        match self {
            Error::Desinstallation(desinstallation::Error::Plan(error)) => Some(error),
            Error::Inventaire(error) => Some(error),
            _ => None,
        }
    }
}

impl Codee for Error {
    fn code(&self) -> &'static str {
        match self {
            Error::PasUnProjet => "pas_un_projet",
            Error::PasUnFragment { .. } => "fragment_inconnu",
            Error::Exigee { .. } => "feature_exigee",
            Error::Acces(_) => "fichier_inaccessible",
            Error::Manifest(_) => "fragment_invalide",
            Error::Metadata(_) => "manifeste_illisible",
            Error::WorkingTreeSale(_) => "arbre_sale",
            Error::Env(_) => "env_illisible",
            Error::UrlIndecomposable { .. } => "url_indecomposable",
            Error::Desinstallation(cause) => match cause {
                desinstallation::Error::AncreInconnue { .. } => "ancre_inconnue",
                desinstallation::Error::Installation(erreur) => erreur.code(),
                desinstallation::Error::Rendu { .. } => "rendu_impossible",
                desinstallation::Error::Plan(erreur) => erreur.code(),
                desinstallation::Error::Acces(_) => "fichier_inaccessible",
                desinstallation::Error::Manifest(_) => "fragment_invalide",
                desinstallation::Error::MigrationAmbigue { .. } => "migration_ambigue",
            },
            Error::Inventaire(erreur) => erreur.code(),
            Error::Application(erreur) => erreur.code(),
        }
    }

    fn remede(&self) -> Option<String> {
        self.remedy()
    }

    fn bloc(&self) -> Option<String> {
        self.plan().and_then(plan::Error::bloc)
    }
}

impl crate::errors::Classee for Error {
    fn sortie(&self) -> crate::errors::Sortie {
        use crate::errors::Sortie;

        match self {
            Self::PasUnProjet | Self::PasUnFragment { .. } | Self::WorkingTreeSale(_) => {
                Sortie::Usage
            }
            Self::Acces(_) => Sortie::Environnement,
            Self::Exigee { .. } | Self::Manifest(_) | Self::UrlIndecomposable { .. } => {
                Sortie::Faute
            }
            Self::Metadata(cause) => cause.sortie(),
            Self::Env(cause) => cause.sortie(),
            Self::Desinstallation(cause) => match cause {
                desinstallation::Error::Acces(_) => Sortie::Environnement,
                desinstallation::Error::Plan(erreur) => erreur.sortie(),
                desinstallation::Error::AncreInconnue { .. }
                | desinstallation::Error::Installation(_)
                | desinstallation::Error::Rendu { .. }
                | desinstallation::Error::Manifest(_)
                | desinstallation::Error::MigrationAmbigue { .. } => Sortie::Faute,
            },
            Self::Inventaire(cause) => cause.sortie(),
            Self::Application(cause) => cause.sortie(),
        }
    }
}

/// Calcule ce que le retrait de `options` ferait au projet, sans rien écrire.
///
/// Ordre des contrôles, chacun un refus avant la moindre écriture : le nom désigne-t-il un
/// fragment (`PasUnFragment`) ; le projet l'inscrit-il encore (sinon rien à faire) ; un
/// autre fragment installé l'exige-t-il (`Exigee`) ; le working tree est-il propre
/// (`WorkingTreeSale`). Le plan ne se construit qu'une fois les quatre passés.
// `desinstallation::Error` porte des variantes de plus de 128 octets (les messages
// d'ambiguïté de migration, notamment) : la boxer changerait la forme que la tâche 8 a
// arrêtée, pour un chemin d'erreur qui n'est jamais le chemin chaud de la commande.
#[allow(clippy::result_large_err)]
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    let metadata::Cible { root, metadonnees } = metadata::cible::<Error>(&options.directory)?;

    // 1. Connu : le nom doit désigner un fragment embarqué (ou d'un `--template-dir`) qui
    // porte un manifeste. Vérifié avant même de savoir si le projet l'inscrit : un nom
    // qui n'a jamais été un fragment ne devient pas « déjà absent » pour autant, il reste
    // refusé.
    let source = templates::Source::feature(options.template_dir.as_deref(), &options.feature)
        .map_err(|templates::Unknown { feature, known }| Error::PasUnFragment { feature, known })?;
    let (manifeste_texte, fichiers) = source
        .manifest_and_files()
        .map_err(|source| crate::errors::Acces::new(Path::new(&options.feature), source))?;
    let Some(manifeste_texte) = manifeste_texte else {
        // `feature_names` listerait ici le nom qu'on vient de refuser : son répertoire
        // existe bel et bien, seul son manifeste manque. `feature_names_with_manifest`
        // exclut les répertoires qui n'installent rien, pour ne jamais nommer parmi les
        // fragments valides celui que ce message vient de refuser.
        return Err(Error::PasUnFragment {
            feature: options.feature.clone(),
            known: templates::enumerate(templates::feature_names_with_manifest(
                options.template_dir.as_deref(),
            )),
        });
    };
    let manifest = manifest::read(
        &manifeste_texte,
        &format!("{}/feature.toml", options.feature),
    )?;

    // 2. Inscrit : l'idempotence se juge sur le manifeste, comme pour `add` — un fragment
    // que le projet n'a jamais posé, ou a déjà retiré, n'a rien à défaire.
    if !metadonnees.features.contains(&options.feature) {
        return Ok(Planned {
            plan: plan::Builder::new(root).finir(),
            description: String::new(),
            migration: false,
            laissees: Vec::new(),
            deja_absente: true,
            zone_manquante: None,
        });
    }

    // 3. Dépendants : fermeture transitive de ce que les fragments installés exigent
    // encore, jusqu'au point fixe — `webhooks` exige `auth`, qui exige `mail` : retirer
    // `mail` doit nommer les deux d'un coup, sans quoi l'utilisateur retirerait `auth`
    // pour se heurter aussitôt à un second refus qu'on savait déjà venir.
    let dependants = dependants_de(
        options.template_dir.as_deref(),
        &options.feature,
        &metadonnees.features,
    )?;
    if !dependants.is_empty() {
        return Err(Error::Exigee {
            feature: options.feature.clone(),
            dependants: dependants.join(", "),
        });
    }

    // 4. Garde Git : ce qu'un retrait écrira ne doit pas se mêler à des modifications non
    // commitées.
    if !options.force {
        git::garde(&root)?;
    }

    // 5. Le plan. Rejouer le rendu que l'installation avait produit exige le contexte
    // qu'`add` construit — littéralement le même, `contexte::projet` le portant pour les
    // deux commandes — sans quoi chaque fichier comparerait le disque à un rendu que
    // l'installation n'a jamais écrit.
    let reclamees = desinstallation::reclamees_ailleurs(
        options.template_dir.as_deref(),
        &options.feature,
        &metadonnees.features,
    )?;

    let nom_projet = metadonnees.package_name(&root.join("Cargo.toml"))?;
    let context = crate::contexte::projet(
        &root,
        &nom_projet,
        metadonnees.database,
        metadonnees.features.clone(),
    )?;

    let mut builder = plan::Builder::new(root.clone());
    let fragment = desinstallation::Fragment {
        name: &options.feature,
        manifest: &manifest,
        templates: &fichiers,
        context,
        reclamees: &reclamees,
    };
    let retires = desinstallation::actions(&fragment, &mut builder)?;

    // La section 8 vient de retirer la feature du manifeste projeté : l'inventaire de
    // l'`AGENTS.md` doit dire la même chose, ou `rbs doctor` — qui compare la zone au
    // manifeste — verrait diverger un projet qui vient pourtant de subir un retrait
    // propre. `agents::refresh` sait ajouter des features à une liste, jamais en retirer
    // une : la métadonnée passée porte donc déjà l'état d'après, la feature retirée
    // exclue de sa liste, sans rien à ajouter par-dessus.
    let mut metadonnees_apres_retrait = metadonnees.clone();
    metadonnees_apres_retrait
        .features
        .retain(|feature| feature != &options.feature);
    let zone_manquante =
        crate::agents::refresh(&mut builder, &root, &metadonnees_apres_retrait, &[])?;

    Ok(Planned {
        plan: builder.finir(),
        description: manifest.feature.description,
        migration: retires.migration.is_some(),
        laissees: retires.laissees,
        deja_absente: false,
        zone_manquante,
    })
}

/// Les fragments installés qui exigent `partant`, directement ou par ricochet.
///
/// Lit le manifeste de chaque fragment installé — le partant excepté — et retient ceux
/// dont `feature.requires` le nomme, puis propage jusqu'au point fixe : un dépendant
/// trouvé à un tour peut lui-même être exigé par un autre, encore à découvrir. Un nom
/// installé sans fragment embarqué (un CRUD engendré) est ignoré en silence plutôt que de
/// faire échouer le calcul — la même règle que suit [`desinstallation::reclamees_ailleurs`].
#[allow(clippy::result_large_err)]
fn dependants_de(
    template_dir: Option<&Path>,
    partant: &str,
    installees: &[String],
) -> Result<Vec<String>, Error> {
    let mut requiert: Vec<(String, Vec<String>)> = Vec::new();

    for nom in installees {
        if nom == partant {
            continue;
        }

        let Ok(source) = templates::Source::feature(template_dir, nom) else {
            continue;
        };
        let (manifeste, _) = source
            .manifest_and_files()
            .map_err(|source| crate::errors::Acces::new(Path::new(nom.as_str()), source))?;
        let Some(texte) = manifeste else {
            continue;
        };

        let manifest = manifest::read(&texte, &format!("{nom}/feature.toml"))?;
        requiert.push((nom.clone(), manifest.feature.requires));
    }

    let mut cible: BTreeSet<String> = BTreeSet::new();
    cible.insert(partant.to_string());
    let mut trouves: BTreeSet<String> = BTreeSet::new();

    loop {
        let mut ajoute = false;
        for (nom, requires) in &requiert {
            if trouves.contains(nom) {
                continue;
            }
            if requires.iter().any(|requis| cible.contains(requis)) {
                trouves.insert(nom.clone());
                cible.insert(nom.clone());
                ajoute = true;
            }
        }
        if !ajoute {
            break;
        }
    }

    Ok(trouves.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::plan::Status;

    fn options(root: &Path, feature: &str) -> Options {
        Options {
            feature: feature.to_string(),
            directory: root.to_path_buf(),
            force: false,
            template_dir: None,
        }
    }

    /// Un projet dont seul `[package.metadata.rbs] features` est renseigné à la main :
    /// les refus n'ont besoin d'aucun fichier réel, seulement de ce que le manifeste
    /// inscrit — un nom absent de la liste n'a jamais existé pour eux.
    fn projet_avec_features(features: &[&str]) -> TempDir {
        let projet = TempDir::new().expect("le répertoire temporaire se crée");
        let toml = format!(
            "[package]\nname = \"demo-api\"\n\n[package.metadata.rbs]\nversion = \"0.0.0\"\n\
             features = [{}]\n",
            features
                .iter()
                .map(|feature| format!("\"{feature}\""))
                .collect::<Vec<_>>()
                .join(", "),
        );
        std::fs::write(projet.path().join("Cargo.toml"), toml).expect("le manifeste s'écrit");
        projet
    }

    fn commit(root: &Path) {
        for arguments in [
            vec!["config", "user.email", "rbs@example.test"],
            vec!["config", "user.name", "rbs"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "projet neuf"],
        ] {
            let output = std::process::Command::new("git")
                .args(&arguments)
                .current_dir(root)
                .output()
                .expect("git doit être lançable");

            assert!(
                output.status.success(),
                "git {arguments:?} a échoué :\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Hors d'un projet rbs, la commande le dit plutôt que de remonter jusqu'à un
    /// `Cargo.toml` sans rapport.
    #[test]
    fn outside_an_rbs_project_the_command_refuses() {
        let ailleurs = TempDir::new().expect("le répertoire temporaire se crée");

        let faute = plan_for(&options(ailleurs.path(), "cors")).expect_err("aucun projet ici");

        assert!(matches!(faute, Error::PasUnProjet), "{faute}");
    }

    /// Un fragment que le projet n'inscrit pas n'a rien à retirer.
    #[test]
    fn a_fragment_the_project_does_not_record_has_nothing_to_remove() {
        let projet = projet_avec_features(&["health"]);

        let planned = plan_for(&options(projet.path(), "mail")).expect("la commande aboutit");

        assert!(planned.deja_absente);
        assert!(planned.plan.files().is_empty());
    }

    /// Un CRUD engendré n'est pas un fragment, et la commande le dit.
    #[test]
    fn a_generated_crud_is_not_a_fragment() {
        let projet = projet_avec_features(&["health", "posts"]);

        let faute = plan_for(&options(projet.path(), "posts")).expect_err("le retrait est refusé");

        assert!(matches!(faute, Error::PasUnFragment { .. }), "{faute}");
    }

    /// Un nom qui n'a jamais été un fragment est refusé même quand le projet ne l'inscrit
    /// pas : le contrôle « connu » précède celui de l'inscription, il ne s'y substitue
    /// pas.
    #[test]
    fn an_unknown_name_is_refused_even_when_not_recorded() {
        let projet = projet_avec_features(&["health"]);

        let faute =
            plan_for(&options(projet.path(), "n-existe-pas")).expect_err("le retrait est refusé");

        assert!(matches!(faute, Error::PasUnFragment { .. }), "{faute}");
    }

    /// Un répertoire de `--template-dir` qui porte le nom demandé mais aucun
    /// `feature.toml` est refusé sans se contredire : il ne doit pas se citer lui-même
    /// parmi les fragments valides qu'il énumère.
    #[test]
    fn a_directory_without_a_manifest_does_not_list_itself_among_valid_fragments() {
        let repertoire = TempDir::new().expect("le répertoire temporaire se crée");
        std::fs::create_dir_all(repertoire.path().join("sans-manifeste"))
            .expect("le répertoire du fragment se crée");
        let projet = projet_avec_features(&["health"]);

        let mut sans_manifeste = options(projet.path(), "sans-manifeste");
        sans_manifeste.template_dir = Some(repertoire.path().to_path_buf());

        let faute = plan_for(&sans_manifeste).expect_err("le retrait est refusé");

        let Error::PasUnFragment { known, .. } = &faute else {
            panic!("attendu PasUnFragment, reçu {faute:?}");
        };
        assert!(
            !known.split(", ").any(|nom| nom == "sans-manifeste"),
            "le fragment refusé ne doit pas figurer parmi les valides : {known}"
        );
        // Seul répertoire du `--template-dir`, et sans manifeste : aucun fragment valide
        // n'y est disponible, et le message doit le dire plutôt que rendre une chaîne
        // vide.
        assert_eq!(known, "aucune n'est disponible");
    }

    /// Un fragment qu'un autre exige est refusé, et le dépendant est nommé.
    #[test]
    fn a_fragment_another_requires_is_refused_and_the_dependant_is_named() {
        let projet = projet_avec_features(&["health", "rate-limit", "mail", "auth"]);

        let faute = plan_for(&options(projet.path(), "mail")).expect_err("le retrait est refusé");

        let Error::Exigee { dependants, .. } = &faute else {
            panic!("attendu Exigee, reçu {faute:?}");
        };
        assert_eq!(dependants, "auth");
    }

    /// Le dépendant transitif est nommé lui aussi.
    #[test]
    fn a_transitive_dependant_is_named_too() {
        let projet =
            projet_avec_features(&["health", "jobs", "rate-limit", "mail", "auth", "webhooks"]);

        let faute = plan_for(&options(projet.path(), "mail")).expect_err("le retrait est refusé");

        let Error::Exigee { dependants, .. } = &faute else {
            panic!("attendu Exigee, reçu {faute:?}");
        };
        assert_eq!(dependants, "auth, webhooks");
    }

    /// Un second retrait ne fait rien : l'idempotence se juge sur le manifeste.
    #[test]
    fn a_second_removal_does_nothing() {
        let projet = projet_avec_features(&["health"]);

        assert!(
            plan_for(&options(projet.path(), "cors"))
                .expect("la commande aboutit")
                .deja_absente
        );
    }

    /// Un working tree sale arrête le retrait sans `--force`, comme pour `add` : la garde
    /// Git s'applique avant que quoi que ce soit ne soit planifié.
    #[test]
    fn a_dirty_working_tree_refuses_without_force_and_passes_with_it() {
        let (_parent, root) = crate::fixtures::Project::new().features(&["cors"]).create();
        commit(&root);
        std::fs::write(root.join("src/main.rs"), "// modifié").expect("le fichier s'écrit");

        let faute =
            plan_for(&options(&root, "cors")).expect_err("un projet sale ne se modifie pas");
        assert!(matches!(faute, Error::WorkingTreeSale(_)), "{faute}");

        let mut forcees = options(&root, "cors");
        forcees.force = true;
        plan_for(&forcees).expect("--force doit passer outre");
    }

    /// Une fois les quatre refus écartés, le plan réel se construit : la migration
    /// déclarée par le fragment devient le booléen du rapport (R2), et aucun fichier
    /// reconstruit ne diverge du disque que l'installation avait écrit.
    #[test]
    fn a_genuinely_installed_fragment_without_dependants_is_actually_planned() {
        let (_parent, root) = crate::fixtures::Project::new().features(&["jobs"]).create();

        let planned = plan_for(&options(&root, "jobs")).expect("le retrait se planifie");

        assert!(!planned.deja_absente);
        assert!(!planned.plan.files().is_empty());
        assert!(planned.migration, "`jobs` déclare une migration");
        assert!(!planned.description.is_empty());
        assert!(
            planned
                .plan
                .files()
                .iter()
                .all(|file| file.statut != Status::Conflit),
            "un rendu reconstruit à tort diffère du disque : {:?}",
            planned.plan.files()
        );
    }

    /// Symétrique de ce qu'`add` fait pour `agents::refresh` : la section 8 retire la
    /// feature du manifeste, et l'inventaire de l'`AGENTS.md` doit dire la même chose —
    /// sans quoi `rbs doctor` verrait diverger un projet qui vient pourtant de subir un
    /// retrait propre.
    #[test]
    fn the_agents_inventory_no_longer_names_a_removed_feature() {
        let (_parent, root) = crate::fixtures::Project::new().features(&["jobs"]).create();

        let planned = plan_for(&options(&root, "jobs")).expect("le retrait se planifie");

        let agents = planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == "AGENTS.md")
            .expect("AGENTS.md fait partie du plan");
        let after = agents
            .after
            .as_deref()
            .expect("AGENTS.md n'est pas supprimé");

        let debut = after
            .find("<!-- rbs:inventory -->")
            .expect("la zone d'inventaire est présente");
        let fin = after
            .find("<!-- /rbs:inventory -->")
            .expect("la zone se referme");
        let inventaire = &after[debut..fin];

        assert!(
            !inventaire.contains("jobs"),
            "l'inventaire doit avoir perdu `jobs` : {inventaire}"
        );
    }

    /// Le critère de la tâche : ce qu'un fragment a exclu du dépôt, son retrait le rend.
    ///
    /// L'ancre des exclusions passe par le même chemin que les autres — le manifeste relu
    /// à l'envers — mais elle est la première à vivre dans un fichier que le squelette
    /// écrit : une ligne laissée là ferait ignorer un répertoire dont plus rien ne parle.
    #[test]
    fn removing_a_fragment_gives_back_the_lines_it_excluded() {
        let (_parent, root) = crate::fixtures::project();
        let fragments = TempDir::new().expect("répertoire temporaire créable");
        let essai = fragments.path().join("essai");
        std::fs::create_dir(&essai).expect("le fragment se crée");
        std::fs::write(
            essai.join("feature.toml"),
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"ignore\"\ncontent = \"/node_modules\"\n",
        )
        .expect("le manifeste s'écrit");

        let pose = crate::add::plan_for(&crate::add::Options {
            features: vec!["essai".to_string()],
            directory: root.clone(),
            force: true,
            template_dir: Some(fragments.path().to_path_buf()),
        })
        .expect("la pose se planifie");
        crate::plan::application::apply(&pose.plan, false).expect("la pose s'applique");
        assert!(
            std::fs::read_to_string(root.join(".gitignore"))
                .expect(".gitignore lisible")
                .contains("/node_modules"),
            "le test ne prouverait rien"
        );

        let mut retrait = options(&root, "essai");
        retrait.force = true;
        retrait.template_dir = Some(fragments.path().to_path_buf());
        let planned = plan_for(&retrait).expect("le retrait se planifie");
        crate::plan::application::apply(&planned.plan, false).expect("le retrait s'applique");

        let exclusions =
            std::fs::read_to_string(root.join(".gitignore")).expect(".gitignore lisible");
        assert!(!exclusions.contains("/node_modules"), "{exclusions}");
        assert!(
            exclusions.contains("# <rbs:ignore>"),
            "l'ancre reste, seule la ligne part : {exclusions}"
        );
    }

    /// Le retrait du frontend défait ce que la pose a fait : le module, le repli sur le
    /// routeur, la section de configuration et les dépendances devenues orphelines.
    ///
    /// C'est le premier fragment dont le retrait doit rendre une feature à une dépendance
    /// que le squelette déclare — `tower-http` garde sa compression et sa borne de temps,
    /// et ne perd que `fs`.
    #[test]
    fn removing_the_frontend_gives_the_project_back_its_router_and_its_configuration() {
        let (_parent, root) = crate::fixtures::Project::new()
            .features(&["frontend"])
            .create();
        assert!(root.join("src/modules/frontend/mod.rs").exists());

        let mut retrait = options(&root, "frontend");
        retrait.force = true;
        let planned = plan_for(&retrait).expect("le retrait se planifie");
        crate::plan::application::apply(&planned.plan, false).expect("le retrait s'applique");

        assert!(
            !root.join("src/modules/frontend/mod.rs").exists(),
            "le module est resté"
        );

        let lire = |relatif: &str| {
            std::fs::read_to_string(root.join(relatif))
                .unwrap_or_else(|_| panic!("{relatif} doit être lisible"))
        };

        let routeur = lire("src/router.rs");
        assert!(!routeur.contains("modules::frontend"), "{routeur}");
        assert!(
            routeur.contains("// <rbs:routes>"),
            "l'ancre reste, seule la ligne part : {routeur}"
        );

        let config = lire("config/default.toml");
        assert!(!config.contains("[frontend]"), "{config}");

        let cargo = lire("Cargo.toml");
        assert!(
            !cargo.contains("\nfs\"") && !cargo.contains("\"fs\""),
            "la feature `fs` est restée à tower-http : {cargo}"
        );
        assert!(
            cargo.contains("compression-gzip"),
            "les features que le squelette déclare ne se retirent pas : {cargo}"
        );

        // `tower` n'était qu'une dépendance de développement avant la pose : le retrait
        // doit lui rendre ce rang, et non la laisser en dépendance d'exécution.
        let dependances = cargo
            .split("[dev-dependencies]")
            .next()
            .expect("la section des dépendances précède celle de développement");
        assert!(
            !dependances.contains("tower ="),
            "`tower` est restée en dépendance d'exécution : {dependances}"
        );

        // Le client s'en va avec le module qui le servait : le laisser derrière poserait
        // un arbre npm que plus rien ne construit ni ne sert.
        assert!(
            !root.join("frontend/package.json").exists()
                && !root.join("frontend/src/views/Accueil.vue").exists(),
            "le client est resté"
        );

        // Et les exclusions qu'il avait posées repartent : une ligne laissée là ferait
        // ignorer un répertoire dont plus rien ne parle.
        let exclusions = lire(".gitignore");
        assert!(!exclusions.contains("node_modules"), "{exclusions}");
        assert!(!exclusions.contains("frontend/dist"), "{exclusions}");
        assert!(
            exclusions.contains("# <rbs:ignore>"),
            "l'ancre reste, seules les lignes partent : {exclusions}"
        );
    }

    /// Le retrait du shell défait l'administration, lignes insérées comprises, et laisse
    /// le socle intact.
    ///
    /// Le shell est le seul fragment à viser des ancres qu'il dépose lui-même : la table
    /// de routage de l'espace et son rail portent, une fois posés, l'écran de
    /// démonstration qu'il y a monté. Le retrait rejoue donc ces insertions avant de
    /// comparer — sans quoi les deux fichiers passeraient pour modifiés à la main, le plan
    /// les classerait en conflit, et l'administration resterait là, à demi démontée.
    ///
    /// L'autre bord compte autant : un routeur ou un accueil emportés avec
    /// l'administration laisseraient un projet qui ne construit plus.
    #[test]
    fn removing_the_admin_shell_leaves_the_base_it_stood_on() {
        let (_parent, root) = crate::fixtures::Project::new()
            .features(&["frontend", "auth", "frontend-admin"])
            .create();
        assert!(root.join("frontend/src/admin/Shell.vue").exists());

        let mut retrait = options(&root, "frontend-admin");
        retrait.force = true;
        let planned = plan_for(&retrait).expect("le retrait se planifie");

        // `force` ne vaut ici que pour l'arbre de travail : un fichier classé en conflit
        // ne serait pas écrit pour autant, et le retrait s'arrêterait à mi-chemin.
        let conflits: Vec<&str> = planned
            .plan
            .files()
            .iter()
            .filter(|fichier| fichier.statut == crate::plan::Status::Conflit)
            .map(|fichier| fichier.path.as_str())
            .collect();
        assert!(
            conflits.is_empty(),
            "le retrait ne reconnaît pas ce qu'il avait écrit : {conflits:?}"
        );

        crate::plan::application::apply(&planned.plan, false).expect("le retrait s'applique");

        for parti in [
            "frontend/src/admin/Shell.vue",
            "frontend/src/admin/montage.ts",
            "frontend/src/admin/rail.ts",
            "frontend/src/admin/garde.ts",
            "frontend/src/admin/lien.ts",
            "frontend/src/admin/vues/Connexion.vue",
            "frontend/src/admin/vues/Inscription.vue",
            "frontend/src/admin/vues/Reinitialisation.vue",
            "frontend/src/admin/vues/Verification.vue",
            "frontend/src/admin/vues/Demonstration.vue",
            "frontend/src/api/entetes.ts",
            "frontend/src/api/jetons.ts",
            "frontend/src/stores/authentification.ts",
            "frontend/src/stores/interface.ts",
        ] {
            assert!(!root.join(parti).exists(), "{parti} est resté");
        }

        // Le socle, lui, ne bouge pas : son routeur cherche toujours un montage, et n'en
        // trouve plus — c'est exactement l'état d'un projet qui n'a jamais posé le shell.
        let lire = |relatif: &str| {
            std::fs::read_to_string(root.join(relatif))
                .unwrap_or_else(|_| panic!("{relatif} doit être lisible"))
        };
        let routeur = lire("frontend/src/router/index.ts");
        assert!(routeur.contains("montage.ts"), "{routeur}");
        for garde in [
            "frontend/src/views/Accueil.vue",
            "frontend/src/api/index.ts",
            "frontend/src/lib/theme.ts",
            "frontend/src/main.ts",
            "frontend/package.json",
            "src/modules/frontend/mod.rs",
        ] {
            assert!(root.join(garde).exists(), "{garde} est parti avec le shell");
        }

        // `auth` reste : le shell l'exigeait, l'inverse n'est pas vrai, et une table de
        // comptes qui disparaîtrait avec une interface serait une perte de données.
        assert!(root.join("src/auth/mod.rs").exists(), "auth est partie");
    }

    /// Une ancre effacée à la main rend le fichier en conflit, et n'arrête pas la commande.
    ///
    /// Le retrait rejoue les insertions du fragment avant de comparer le disque à ses
    /// templates : c'est ainsi qu'il reconnaît un fichier qu'il a lui-même écrit *et*
    /// visé. Le rejeu part de la template, où les balises sont toujours là, et ce que le
    /// développeur a fait du fichier ne peut donc pas faire lever la commande — seulement
    /// l'écarter de ce que le retrait attendait. C'est un conflit, et le plan le dit.
    #[test]
    fn an_anchor_wiped_by_hand_makes_the_file_a_conflict_rather_than_an_error() {
        let (_parent, root) = crate::fixtures::Project::new()
            .features(&["frontend", "auth", "frontend-admin"])
            .create();

        let montage = root.join("frontend/src/admin/montage.ts");
        let sans_ancre = std::fs::read_to_string(&montage)
            .expect("le montage est lisible")
            .lines()
            .filter(|ligne| !ligne.contains("rbs:admin_routes"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&montage, sans_ancre).expect("le montage est réinscriptible");

        let planned = plan_for(&options(&root, "frontend-admin")).expect("le retrait se planifie");

        let statut = planned
            .plan
            .files()
            .iter()
            .find(|fichier| fichier.path == "frontend/src/admin/montage.ts")
            .map(|fichier| fichier.statut)
            .expect("le montage est au plan");
        assert_eq!(
            statut,
            crate::plan::Status::Conflit,
            "un fichier dont l'ancre a été effacée doit être un conflit"
        );
    }
}
