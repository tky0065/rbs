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
use std::io;
use std::path::{Path, PathBuf};

use minijinja::context;

use crate::dotenv;
use crate::git;
use crate::manifest;
use crate::metadata;
use crate::migrate;
use crate::plan;
use crate::templates;

/// Ce qu'il faut savoir pour retirer une feature.
// Sans appelant avant que `rbs remove` ne soit câblée à cette commande : `-D warnings`
// la dirait morte, alors que les tests en prouvent déjà le contrat.
#[allow(dead_code)]
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
// Idem : construit par les seuls tests avant que `rbs remove` ne soit câblée.
#[allow(dead_code)]
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
}

/// Ce qui peut empêcher de retirer une feature.
// Idem : rendu par les seules fonctions de ce module, sans appelant avant que `rbs
// remove` ne soit câblée.
#[allow(dead_code)]
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
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

/// Calcule ce que le retrait de `options` ferait au projet, sans rien écrire.
///
/// Ordre des contrôles, chacun un refus avant la moindre écriture : le nom désigne-t-il un
/// fragment (`PasUnFragment`) ; le projet l'inscrit-il encore (sinon rien à faire) ; un
/// autre fragment installé l'exige-t-il (`Exigee`) ; le working tree est-il propre
/// (`WorkingTreeSale`). Le plan ne se construit qu'une fois les quatre passés.
// Sans appelant avant que `rbs remove` ne soit câblée à cette commande : `-D warnings`
// la dirait morte, alors que les tests en prouvent déjà le contrat.
#[allow(dead_code)]
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
        return Err(Error::PasUnFragment {
            feature: options.feature.clone(),
            known: templates::feature_names(options.template_dir.as_deref()).join(", "),
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

    // 5. Le plan. Rejouer le rendu que l'installation avait produit exige le même
    // contexte qu'`add` construit — nom du paquet, moteur, URL du `.env` — sans quoi
    // chaque fichier comparerait le disque à un rendu que l'installation n'a jamais écrit.
    let reclamees = desinstallation::reclamees_ailleurs(
        options.template_dir.as_deref(),
        &options.feature,
        &metadonnees.features,
    )?;

    let nom_projet = metadonnees.package_name(&root.join("Cargo.toml"))?;
    let crate_name = nom_projet.replace('-', "_");
    let database = metadonnees.database;

    // L'URL du projet, non une valeur par défaut : c'est elle que l'installation avait
    // décomposée pour rendre `docker`, et un repli silencieux ferait diverger le rendu
    // comparé au disque dès que le mot de passe n'est pas celui de la démonstration.
    let url = match migrate::project_variables(&root) {
        Ok(variables) => dotenv::value(&variables, migrate::URL).map(str::to_string),
        Err(migrate::Error::SansUrl) => None,
        Err(migrate::Error::Env(dotenv::Error::Acces(faute)))
            if faute.source.kind() == io::ErrorKind::NotFound =>
        {
            None
        }
        Err(faute) => return Err(Error::Env(faute)),
    }
    .unwrap_or_else(|| database.default_url(&crate_name));
    let connexion = crate::url::parse(&url);

    if database.a_un_serveur() && connexion.is_none() {
        return Err(Error::UrlIndecomposable { url });
    }

    let utilisateur = connexion
        .as_ref()
        .map(|c| c.user.clone())
        .unwrap_or_default();

    let context = context! {
        project_name => nom_projet,
        crate_name => crate_name.clone(),
        // Par où le binaire principal atteint un module de feature : la bibliothèque du
        // projet, ou `crate::` sur un projet engendré avant qu'elle n'existe.
        crate_path => if root.join("src/lib.rs").exists() {
            crate_name.clone()
        } else {
            "crate".to_string()
        },
        features => metadonnees.features.clone(),
        rust_image => templates::rust_image(),
        database => database.name(),
        database_a_un_serveur => database.a_un_serveur(),
        database_url_compose => crate::url::interne(database, &utilisateur)
            .unwrap_or_else(|| database.compose_url(&crate_name)),
        database_url_par_defaut => database.default_url(&crate_name),
        // `[server] lang` de `config/default.toml`, non la métadonnée : celle-ci ne
        // gouverne plus que `AGENTS.md` depuis 1.5.0.
        lang => crate::lang::Lang::of_project(&root).name(),
    };

    let mut builder = plan::Builder::new(root);
    let fragment = desinstallation::Fragment {
        name: &options.feature,
        manifest: &manifest,
        templates: &fichiers,
        context,
        reclamees: &reclamees,
    };
    let retires = desinstallation::actions(&fragment, &mut builder)?;

    Ok(Planned {
        plan: builder.finir(),
        description: manifest.feature.description,
        migration: retires.migration.is_some(),
        laissees: retires.laissees,
        deja_absente: false,
    })
}

/// Les fragments installés qui exigent `partant`, directement ou par ricochet.
///
/// Lit le manifeste de chaque fragment installé — le partant excepté — et retient ceux
/// dont `feature.requires` le nomme, puis propage jusqu'au point fixe : un dépendant
/// trouvé à un tour peut lui-même être exigé par un autre, encore à découvrir. Un nom
/// installé sans fragment embarqué (un CRUD engendré) est ignoré en silence plutôt que de
/// faire échouer le calcul — la même règle que suit [`desinstallation::reclamees_ailleurs`].
// Sans appelant hors de `plan_for`, elle-même sans appelant avant `rbs remove`.
#[allow(dead_code)]
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
}
