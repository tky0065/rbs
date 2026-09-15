//! `rbs add <feature>` : le fragment d'une feature déposé dans un projet existant.
//!
//! Rien de propre à `docker` ou à `ci` n'est écrit ici. Le catalogue d'une feature est son
//! répertoire sous `templates/features`, et son contexte de rendu se déduit du projet
//! visé : ajouter une feature qui n'apporte pas de code Rust, c'est ajouter un répertoire.
//!
//! La séquence est celle de `generate` — racine, garde Git, plan, application — pour la
//! même raison : ce qui modifie un projet existant se montre avant de s'écrire, et
//! s'écrit en entier ou pas du tout.

mod installation;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use minijinja::context;

use crate::dotenv;
use crate::errors::Codee;
use crate::git;
use crate::manifest;
use crate::metadata;
use crate::migrate;
use crate::plan;
use crate::templates::{self, Source};

/// Ce qu'il faut savoir pour installer une feature.
pub(crate) struct Options {
    /// Les features demandées, telles que les sous-répertoires de `templates/features` les
    /// nomment.
    ///
    /// `rbs add` en nomme une ; `rbs new --with` les passe toutes à la fois, parce qu'un
    /// fragment rendu doit savoir ce que le plan pose à côté de lui — `rate-limit` compte
    /// dans Redis si `redis` arrive dans la même passe, fût-il posé après.
    pub features: Vec<String>,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Installe même si le projet porte des modifications non commitées.
    pub force: bool,
    /// Répertoire de templates remplaçant celles embarquées.
    pub template_dir: Option<PathBuf>,
}

/// Ce qu'une installation fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Chemins des fichiers que les fragments déposent, relatifs à la racine du projet.
    ///
    /// Seuls les tests les lisent : le bilan de la commande se tire du plan, qui sait
    /// aussi ce qui a été modifié.
    #[cfg(test)]
    pub files: Vec<String>,
    /// Chaque fragment que ce plan pose, dans l'ordre de pose.
    ///
    /// `rbs new` rapporte fragment par fragment ce qu'il a écrit, et `files` ne dit pas
    /// à qui appartient quoi.
    pub poses: Vec<Pose>,
    /// Ce que le fragment annonce installer, tel que son manifeste le décrit.
    ///
    /// Celle de la première feature demandée : `rbs add` n'en nomme qu'une, et c'est lui
    /// seul qui l'affiche.
    pub description: String,
    /// Les fragments que celui demandé entraîne, et que ce plan pose avec lui.
    ///
    /// Dans l'ordre où ils seront posés, la feature demandée exclue : ce que l'utilisateur
    /// n'a pas nommé, il doit le lire avant que le plan ne s'applique.
    pub entrainees: Vec<String>,
    /// Le projet inscrit déjà cette feature : le plan est vide et rien ne sera écrit.
    pub deja_installee: bool,
    /// La zone de l'`AGENTS.md` que le projet ne porte pas, s'il en manque une.
    pub zone_manquante: Option<crate::agents::MissingZone>,
    /// Les CRUD déjà générés qu'`auth` laisse ouverts, quand elle arrive après eux.
    ///
    /// Vide dès que `auth` n'est pas posée par ce plan : le CLI ne réécrit jamais un
    /// fichier existant, et un CRUD généré avant elle resterait grand ouvert sans qu'un
    /// message ne le dise.
    ouverts: Vec<String>,
}

/// Un fragment du plan, et ce qu'il y dépose.
#[derive(Debug)]
pub(crate) struct Pose {
    /// Nom de la feature, tel que `rbs add` l'accepte.
    pub name: String,
    /// Nombre de fichiers que le fragment dépose, sa migration comprise.
    pub files: usize,
    /// Le fragment pose une migration.
    pub migration: bool,
}

impl Planned {
    /// Le message nommant les CRUD que cette installation laisse ouverts, s'il y en a.
    ///
    /// Ne se pose que lorsque `auth` s'installe et que la liste n'est pas vide : les
    /// autres fragments ne ferment aucune route, et un projet neuf n'a aucun CRUD à
    /// fermer.
    pub(crate) fn remedy(&self) -> Option<String> {
        if self.ouverts.is_empty() {
            return None;
        }

        Some(format!(
            "les features déjà générées restent publiques : {}. Le CLI ne réécrit pas un \
             fichier existant — sur chaque handler à fermer, ajoutez le paramètre \
             `identite: Identity`, l'appel `identite.require_role(Role::User)?`, l'entrée \
             `security((\"bearer\" = []))` et les réponses 401 et 403 de son annotation.",
            self.ouverts.join(", ")
        ))
    }
}

/// Ce qui peut empêcher d'installer une feature.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Aucun fragment ne porte ce nom.
    #[error("{0}")]
    Unknown(#[from] templates::Unknown),

    /// Un fichier du projet ou une template n'a pu être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le fragment ne porte pas de manifeste.
    ///
    /// Un fragment sans manifeste ne déclare rien, et s'installerait donc sans rien
    /// faire : mieux vaut le dire que réussir à vide.
    #[error("{feature}/feature.toml est absent : le fragment ne déclare pas son installation")]
    SansManifeste {
        /// Feature demandée.
        feature: String,
    },

    /// Le manifeste du fragment ne se lit pas.
    #[error("{0}")]
    Manifest(#[from] manifest::Error),

    /// Ce que le manifeste déclare n'a pas pu être planifié.
    #[error("{0}")]
    Installation(#[from] installation::Error),

    /// Le projet porte des modifications non commitées, qu'une installation rendrait
    /// indiscernables des siennes.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Le `.env` du projet est là, mais illisible.
    ///
    /// Le fragment se pose sur les identifiants du projet : les inventer plutôt que de
    /// dire la panne engendrerait un compose qui n'atteint pas sa base.
    #[error("{0}")]
    Env(#[from] migrate::Error),

    /// L'URL du projet ne se décompose pas, et le fragment s'appuie dessus.
    ///
    /// Un caractère réservé laissé tel quel dans le mot de passe met fin à l'autorité de
    /// l'URL. Les identifiants tombaient alors à vide plutôt que de le dire : le `.env`
    /// recevait un `POSTGRES_USER=` sans valeur, et le service `db` ne montait pas.
    #[error(
        "RBS_DATABASE__URL ne se décompose pas : {url} — encodez les caractères réservés \
         du mot de passe (`/` en %2F, `?` en %3F, `#` en %23, `@` en %40)"
    )]
    UrlIndecomposable {
        /// L'URL, telle que le `.env` du projet la porte.
        url: String,
    },

    /// Le plan de l'installation n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Error {
    /// Ce que le développeur peut coller ou déplacer pour réparer, quand la panne se
    /// répare ainsi.
    ///
    /// Le texte lui-même vit sur `plan::Error::remede` — porté une seule fois, pour
    /// toutes les commandes qui délèguent à un plan.
    pub(crate) fn remedy(&self) -> Option<String> {
        self.plan()?.remede()
    }

    /// L'erreur de planification que celle-ci porte, directement ou par l'installation.
    fn plan(&self) -> Option<&plan::Error> {
        match self {
            Error::Plan(error) | Error::Installation(installation::Error::Plan(error)) => {
                Some(error)
            }
            _ => None,
        }
    }
}

impl Codee for Error {
    fn code(&self) -> &'static str {
        match self {
            Error::PasUnProjet => "pas_un_projet",
            Error::Unknown(_) => "feature_inconnue",
            Error::Acces(_) => "fichier_inaccessible",
            Error::SansManifeste { .. } => "fragment_sans_manifeste",
            Error::Manifest(_) => "fragment_invalide",
            Error::Installation(erreur) => erreur.code(),
            Error::WorkingTreeSale(_) => "arbre_sale",
            Error::Metadata(_) => "manifeste_illisible",
            Error::Env(_) => "env_illisible",
            Error::UrlIndecomposable { .. } => "url_indecomposable",
            Error::Plan(erreur) => erreur.code(),
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

/// Calcule ce que l'installation de `options` ferait au projet, sans rien écrire.
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    // Une seule lecture pour toute la fonction : son erreur se propage par `?` plutôt
    // que d'être ré-tentée, et `agents::refresh` reçoit ces métadonnées au lieu de les
    // relire elle-même.
    let metadata::Cible { root, metadonnees } = metadata::cible::<Error>(&options.directory)?;

    // L'idempotence se juge sur `[package.metadata.rbs]`, et non sur la présence des
    // fichiers installés : la migration d'un fragment est horodatée, et un projet dont
    // le développeur a supprimé un fichier en recevrait une seconde, datée d'un autre
    // instant. Ce que `rbs add` a posé lui appartient ensuite.
    if options
        .features
        .iter()
        .all(|feature| metadonnees.features.contains(feature))
    {
        return Ok(Planned {
            plan: plan::Builder::new(root).finir(),
            #[cfg(test)]
            files: Vec::new(),
            poses: Vec::new(),
            description: String::new(),
            entrainees: Vec::new(),
            deja_installee: true,
            zone_manquante: None,
            ouverts: Vec::new(),
        });
    }

    if !options.force {
        git::garde(&root)?;
    }

    // Les features demandées et celles qu'elles entraînent partagent un seul plan :
    // l'utilisateur voit ce qui s'écrira, y compris ce qu'il n'a pas nommé, avant que
    // quoi que ce soit ne s'écrive.
    let a_poser = resoudre(
        options.template_dir.as_deref(),
        &options.features,
        &metadonnees.features,
    )?;

    // Ce que le projet portera une fois le plan appliqué, et non ce que le disque porte
    // avant lui : un fragment qui sait qu'un autre est là s'appuie dessus — la limite de
    // débit compte dans Redis quand le cache existe, dans sa mémoire sinon — et le cache
    // posé par le même plan existe bel et bien, même s'il se pose après.
    let mut features = metadonnees.features.clone();
    features.extend(a_poser.iter().map(|fragment| fragment.name.clone()));

    let nom_projet = metadonnees.package_name(&root.join("Cargo.toml"))?;
    let crate_name = nom_projet.replace('-', "_");
    // Le moteur vient du manifeste, seul endroit où le choix de `rbs new` a survécu : un
    // fragment posé six mois plus tard n'a plus les flags de la création.
    let database = metadonnees.database;

    // L'URL du projet, non une valeur par défaut : le compose que le fragment engendre
    // doit se connecter à la base que le projet interroge, avec ses identifiants. Un
    // `.env` qu'on ne sait pas ouvrir en porte peut-être d'autres, et les remplacer en
    // silence poserait un compose qui ne se connecte à rien : seule l'absence se replie,
    // parce qu'un projet neuf n'a rien encore à contredire.
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

    // Refuser plutôt que se replier : des identifiants vides posent un `POSTGRES_USER=`
    // que Compose substitue par rien, et le service `db` ne monte jamais — panne à
    // l'exécution, pour une URL que la commande avait sous les yeux. SQLite n'a pas
    // d'autorité à décomposer, et n'est donc pas concerné.
    if database.a_un_serveur() && connexion.is_none() {
        return Err(Error::UrlIndecomposable { url });
    }

    // Une URL sans chemin rend un nom de base vide, que le repli ne rattraperait pas
    // s'il ne guettait que `None` : le compose porterait un `POSTGRES_DB:` vide, et le
    // service ne deviendrait jamais sain.
    let nom_base = connexion
        .as_ref()
        .map(|c| c.database.clone())
        .filter(|base| !base.is_empty())
        .unwrap_or_else(|| crate_name.clone());
    let utilisateur = connexion
        .as_ref()
        .map(|c| c.user.clone())
        .unwrap_or_default();

    // Les identifiants que `.env.example` documente sont ceux de l'URL de démonstration
    // du moteur, comme le squelette les écrit : les recopier à la main dans le fragment
    // les ferait diverger de `default_url`.
    let demonstration = crate::url::parse(&database.default_url(&crate_name));

    let context = context! {
        project_name => nom_projet.clone(),
        crate_name => crate_name.clone(),
        rust_version => crate::templates::RUST_VERSION,
        features => features,
        // Par où le binaire principal atteint un module de feature : la bibliothèque du
        // projet, ou `crate::` sur un projet engendré avant qu'elle n'existe, où ces
        // modules vivent dans le binaire lui-même.
        crate_path => if root.join("src/lib.rs").exists() {
            crate_name.clone()
        } else {
            "crate".to_string()
        },
        database => database.name(),
        database_a_un_serveur => database.a_un_serveur(),
        // Le moteur décide, et non la lecture de l'URL : une URL que rien ne décompose
        // ferait sinon tomber un moteur à serveur sur `compose_url`, dont les identifiants
        // en dur partiraient dans un fichier versionné.
        database_url_compose => crate::url::interne(database, &utilisateur)
            .unwrap_or_else(|| database.compose_url(&crate_name)),
        database_url_par_defaut => database.default_url(&crate_name),
        database_user => utilisateur.clone(),
        database_password => connexion.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name => nom_base,
        database_port => connexion.as_ref().map(|c| c.port).unwrap_or_default(),
        database_user_par_defaut => demonstration.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
        database_password_par_defaut => demonstration.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name_par_defaut => demonstration.as_ref().map(|c| c.database.clone()).unwrap_or_default(),
        // `[server] lang` de `config/default.toml`, non la métadonnée : celle-ci ne
        // gouverne plus que `AGENTS.md`, et pouvait diverger de la langue des réponses HTTP
        // avant 1.5.0, quand elle se remplissait de la locale.
        lang => crate::lang::Lang::of_project(&root).name(),
    };

    let mut builder = plan::Builder::new(root.clone());
    let timestamp = crate::generate::migration::current_timestamp();
    #[cfg(test)]
    let mut files = Vec::new();
    let mut poses = Vec::new();

    for fragment in &a_poser {
        let deposes = installation::actions(
            &installation::Fragment {
                name: &fragment.name,
                manifest: &fragment.manifest,
                templates: &fragment.templates,
                context: context.clone(),
                timestamp: &timestamp,
            },
            &mut builder,
        )?;

        poses.push(Pose {
            name: fragment.name.clone(),
            files: deposes.len(),
            migration: fragment.manifest.migration.is_some(),
        });
        #[cfg(test)]
        files.extend(deposes);

        builder.patch(plan::PatchToml::InscrireFeature(fragment.name.clone()))?;
    }

    let posees: Vec<String> = a_poser.iter().map(|f| f.name.clone()).collect();

    // Nommé au moment où `auth` arrive, pas seulement quand elle est demandée :
    // `webhooks` l'entraîne, et un CRUD généré avant elle reste ouvert dans les deux cas.
    let ouverts = if posees.iter().any(|nom| nom == "auth") {
        let catalogue = templates::feature_names(options.template_dir.as_deref());
        metadonnees
            .features
            .iter()
            .filter(|feature| feature.as_str() != "health" && !catalogue.contains(feature))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    // L'inventaire décrit le projet tel que ce plan le laissera : les features viennent
    // d'y être inscrites, et le manifeste du disque les ignore encore.
    let zone_manquante = crate::agents::refresh(&mut builder, &root, &metadonnees, &posees)?;

    let description = a_poser
        .iter()
        .find(|fragment| options.features.first() == Some(&fragment.name))
        .map_or_else(String::new, |fragment| {
            fragment.manifest.feature.description.clone()
        });

    Ok(Planned {
        plan: builder.finir(),
        #[cfg(test)]
        files,
        poses,
        description,
        entrainees: posees
            .into_iter()
            .filter(|name| !options.features.contains(name))
            .collect(),
        deja_installee: false,
        zone_manquante,
        ouverts,
    })
}

/// Un fragment lu, prêt à être planifié.
struct Prevu {
    /// Nom de la feature, tel que son répertoire la nomme.
    name: String,
    /// Ce que son manifeste déclare.
    manifest: manifest::Manifest,
    /// Ses templates, telles que la source les a lues.
    templates: Vec<templates::File>,
}

/// Les fragments à poser pour honorer `features` : elles, et ceux qu'elles entraînent.
///
/// L'ordre de pose est celui des dépendances, départagé par le nom : un fragment passe
/// avant ceux qui l'exigent, et avant ceux qui insèrent dans une ancre dont il écrit le
/// fichier porteur — `docker` dépose le compose que `mail` étend par `services`, et sur
/// un projet qui n'en a pas l'inverse échoue sur une ancre introuvable. Le nom tranche le
/// reste, pour que deux demandes équivalentes laissent le même projet : les ancres non
/// triées empilent dans l'ordre de pose. Les `pub mod` du squelette n'en dépendent pas,
/// leurs ancres se maintenant triées d'elles-mêmes.
///
/// Un fragment que `[package.metadata.rbs]` inscrit déjà n'est pas reposé : l'entraînement
/// obéit à la même idempotence que l'installation directe.
fn resoudre(
    template_dir: Option<&Path>,
    features: &[String],
    installees: &[String],
) -> Result<Vec<Prevu>, Error> {
    let mut resolution = Resolution {
        template_dir,
        installees,
        poses: Vec::new(),
        en_cours: Vec::new(),
    };
    for feature in features {
        resolution.resoudre(feature)?;
    }

    Ok(ordonner(resolution.poses))
}

/// Trie `poses` par dépendances, le plus petit nom d'abord à chaque égalité.
///
/// Un cycle — deux manifestes d'un `--template-dir` qui s'exigent l'un l'autre — ne
/// libère jamais ses membres : le plus petit nom sort alors d'office, ce qui reste
/// déterministe et ne laisse rien derrière.
fn ordonner(poses: Vec<Prevu>) -> Vec<Prevu> {
    let mut restants: BTreeMap<String, Prevu> = poses
        .into_iter()
        .map(|pose| (pose.name.clone(), pose))
        .collect();

    // Le fichier qu'un fragment écrit porte peut-être une ancre du registre ; c'est
    // alors lui qui l'apporte aux fragments qui la visent.
    let porteurs: BTreeMap<&str, Vec<String>> = crate::anchors::ANCRES
        .iter()
        .map(|anchor| {
            let ecrivent = restants
                .values()
                .filter(|pose| {
                    pose.manifest
                        .files
                        .iter()
                        .any(|file| file.destination == anchor.file)
                })
                .map(|pose| pose.name.clone())
                .collect();
            (anchor.name.as_ref(), ecrivent)
        })
        .collect();

    let mut avant: BTreeMap<String, BTreeSet<String>> = restants
        .values()
        .map(|pose| {
            let exigees = pose.manifest.feature.requires.iter().cloned();
            let apportees = pose.manifest.anchors.iter().flat_map(|insertion| {
                porteurs
                    .get(insertion.anchor.as_str())
                    .into_iter()
                    .flatten()
                    .cloned()
            });
            let precedents = exigees
                .chain(apportees)
                .filter(|nom| nom != &pose.name && restants.contains_key(nom))
                .collect();

            (pose.name.clone(), precedents)
        })
        .collect();

    let mut ordre = Vec::with_capacity(restants.len());
    while !restants.is_empty() {
        let suivant = avant
            .iter()
            .find(|(_, precedents)| precedents.is_empty())
            .or_else(|| avant.iter().next())
            .map(|(nom, _)| nom.clone())
            .expect("`avant` compte un nom par fragment restant");

        avant.remove(&suivant);
        for precedents in avant.values_mut() {
            precedents.remove(&suivant);
        }
        ordre.push(
            restants
                .remove(&suivant)
                .expect("chaque nom d'`avant` est un fragment restant"),
        );
    }

    ordre
}

/// L'état d'un parcours des `requires`, du fragment demandé vers ceux qu'il entraîne.
struct Resolution<'a> {
    template_dir: Option<&'a Path>,
    installees: &'a [String],
    poses: Vec<Prevu>,
    /// Les fragments dont les exigences sont en cours d'exploration.
    ///
    /// Deux fragments qui s'exigent l'un l'autre feraient sinon descendre la récursion
    /// jusqu'au débordement de pile — un manifeste de `--template-dir` peut l'écrire.
    en_cours: Vec<String>,
}

impl Resolution<'_> {
    fn resoudre(&mut self, feature: &str) -> Result<(), Error> {
        let connu = self.installees.iter().chain(self.en_cours.iter());
        if connu
            .chain(self.poses.iter().map(|pose| &pose.name))
            .any(|nom| nom == feature)
        {
            return Ok(());
        }

        let source = Source::feature(self.template_dir, feature)?;
        // Le fragment est lu une fois pour ses deux usages : son manifeste dit ce que
        // l'installation fait au projet, ses fichiers sont ce qu'elle y dépose.
        let (manifeste, templates) = source
            .manifest_and_files()
            .map_err(|source| crate::errors::Acces::new(Path::new(feature), source))?;
        let manifest = read_manifest(manifeste, feature)?;

        self.en_cours.push(feature.to_string());
        for requise in manifest.feature.requires.clone() {
            self.resoudre(&requise)?;
        }
        self.en_cours.pop();

        self.poses.push(Prevu {
            name: feature.to_string(),
            manifest,
            templates,
        });

        Ok(())
    }
}

/// Analyse le manifeste du fragment, qui dit ce que son installation fait au projet.
///
/// La source lui est passée plutôt que relue : elle sort de la même lecture que les
/// fichiers du fragment.
fn read_manifest(source: Option<String>, feature: &str) -> Result<manifest::Manifest, Error> {
    let text = source.ok_or_else(|| Error::SansManifeste {
        feature: feature.to_string(),
    })?;

    Ok(manifest::read(&text, &format!("{feature}/feature.toml"))?)
}

impl crate::errors::Classee for Error {
    fn sortie(&self) -> crate::errors::Sortie {
        use crate::errors::Sortie;

        match self {
            Self::PasUnProjet | Self::Unknown(_) | Self::WorkingTreeSale(_) => Sortie::Usage,
            Self::Acces(_) => Sortie::Environnement,
            Self::SansManifeste { .. }
            | Self::Manifest(_)
            | Self::Installation(_)
            | Self::UrlIndecomposable { .. } => Sortie::Faute,
            Self::Metadata(cause) => cause.sortie(),
            Self::Plan(cause) => cause.sortie(),
            Self::Env(cause) => cause.sortie(),
            Self::Application(cause) => cause.sortie(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::database::Database;
    use crate::plan::Status;

    /// La template du garde, lue telle qu'elle est embarquée.
    ///
    /// Le garde vit dans le projet de l'utilisateur : aucun test Rust de cette crate ne peut
    /// l'exécuter. Ce qui se vérifie ici est que la comparaison reste un seuil — une égalité
    /// rendue à sa place ferait refuser un admin par la garde que `generate crud` pose par
    /// défaut, et seule la suite Docker le dirait.
    const GUARD: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/features/auth/guard.rs.jinja"
    ));

    const ROLE_MODEL: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/features/auth/model.rs.jinja"
    ));

    #[test]
    fn the_role_guard_compares_a_threshold_and_the_enum_is_ordered() {
        assert!(
            GUARD.contains("porte >= minimum"),
            "le garde doit comparer un seuil, non une égalité :\n{GUARD}"
        );
        assert!(
            !GUARD.contains("porte == expected"),
            "l'égalité stricte doit avoir disparu :\n{GUARD}"
        );
        assert!(
            ROLE_MODEL.contains("PartialOrd, Ord"),
            "l'enum Role doit être ordonné pour que le seuil ait un sens :\n{ROLE_MODEL}"
        );
    }

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    fn fingerprint(root: &Path) -> BTreeMap<PathBuf, String> {
        let mut vue = BTreeMap::new();
        let mut a_visiter = vec![root.to_path_buf()];

        while let Some(directory) = a_visiter.pop() {
            for input in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = input.expect("l'entrée se lit").path();
                let relatif = path
                    .strip_prefix(root)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();

                if path.is_dir() {
                    vue.insert(relatif, String::new());
                    a_visiter.push(path);
                } else {
                    vue.insert(relatif, fs::read_to_string(&path).unwrap_or_default());
                }
            }
        }

        vue
    }

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
    ///
    /// L'URL porte des identifiants distincts du moteur (`rbs`, non `postgres`) — la même
    /// qu'utilise `doctor/anchors.rs` — pour que le compose interne ne puisse pas passer
    /// pour correct en confondant les deux.
    fn project() -> (TempDir, PathBuf) {
        project_with(
            Database::default(),
            "postgres://rbs:rbs@localhost:5432/demo_api",
        )
    }

    /// Le même, sur le moteur demandé — ce que `rbs new --database` produit.
    fn project_on(database: Database) -> (TempDir, PathBuf) {
        project_with(database, &database.default_url("demo_api"))
    }

    /// Le même, sur l'URL demandée : les identifiants du projet sont ce que le fragment
    /// `docker` doit retrouver, et un test qui les choisit peut les reconnaître ailleurs.
    fn project_with(database: Database, database_url: &str) -> (TempDir, PathBuf) {
        crate::fixtures::Project::new()
            .database(database)
            .url(database_url)
            .create()
    }

    /// Ramène le projet à ce qu'était un projet créé avant la 1.1.0 : ni compose, ni
    /// clés du service `db` dans son `.env`.
    ///
    /// C'est l'état sur lequel `rbs add docker` doit encore rendre un compose qui démarre :
    /// les clés qu'il interpole, personne ne les y a écrites.
    fn avant_les_cles_du_compose(root: &Path) {
        let env = fs::read_to_string(root.join(".env")).expect("le .env doit exister");
        let ancien: String = env
            .lines()
            .filter(|ligne| !ligne.starts_with("POSTGRES_") && !ligne.starts_with("MYSQL_"))
            .map(|ligne| format!("{ligne}\n"))
            .collect();

        fs::write(root.join(".env"), ancien).expect("le .env doit se réécrire");
        let _ = fs::remove_file(root.join("docker-compose.yml"));
    }

    /// Fait viser `url` au projet, sans toucher au reste de son `.env`.
    fn viser(root: &Path, url: &str) {
        let env = root.join(".env");
        let source = fs::read_to_string(&env).expect("le .env doit exister");
        let reecrit: String = source
            .lines()
            .map(|ligne| match ligne.starts_with("RBS_DATABASE__URL=") {
                true => format!("RBS_DATABASE__URL={url}\n"),
                false => format!("{ligne}\n"),
            })
            .collect();

        assert!(
            reecrit.contains(url),
            "le .env ne porte pas d'URL :\n{source}"
        );
        fs::write(&env, reecrit).expect("le .env doit se réécrire");
    }

    /// Change `[package.metadata.rbs] lang` sans toucher au reste du manifeste : le
    /// projet neuf en porte déjà une, `fr`, que la substitution retrouve telle quelle.
    fn viser_lang_metadata(root: &Path, lang: &str) {
        let manifest = root.join("Cargo.toml");
        let source = fs::read_to_string(&manifest).expect("le Cargo.toml doit exister");

        assert!(
            source.contains("lang = \"fr\""),
            "la clé `lang` est introuvable :\n{source}"
        );
        let reecrit = source.replacen("lang = \"fr\"", &format!("lang = \"{lang}\""), 1);
        fs::write(&manifest, reecrit).expect("le Cargo.toml doit se réécrire");
    }

    /// Change `[server] lang` de `config/default.toml`, déjà présente à `fr` dans un
    /// projet neuf.
    fn viser_lang_server(root: &Path, lang: &str) {
        let config = root.join("config/default.toml");
        let source = fs::read_to_string(&config).expect("config/default.toml doit exister");

        assert!(
            source.contains("lang = \"fr\""),
            "la clé `lang` est introuvable :\n{source}"
        );
        let reecrit = source.replacen("lang = \"fr\"", &format!("lang = \"{lang}\""), 1);
        fs::write(&config, reecrit).expect("config/default.toml doit se réécrire");
    }

    /// Retire la ligne `lang` de `[server]`, comme un projet créé avant qu'elle n'existe :
    /// la clé est absente, non vide.
    fn retirer_lang_server(root: &Path) {
        let config = root.join("config/default.toml");
        let source = fs::read_to_string(&config).expect("config/default.toml doit exister");

        assert!(
            source.contains("lang = \"fr\""),
            "la clé `lang` est introuvable :\n{source}"
        );
        let reecrit: String = source
            .lines()
            .filter(|ligne| ligne.trim() != "lang = \"fr\"")
            .map(|ligne| format!("{ligne}\n"))
            .collect();
        fs::write(&config, reecrit).expect("config/default.toml doit se réécrire");
    }

    fn options(root: &Path, feature: &str) -> Options {
        options_multi(root, &[feature])
    }

    /// Plusieurs features dans une seule demande — ce que `rbs new --with` transmet.
    fn options_multi(root: &Path, features: &[&str]) -> Options {
        Options {
            features: features.iter().map(|f| f.to_string()).collect(),
            directory: root.to_path_buf(),
            force: false,
            template_dir: None,
        }
    }

    /// Les fragments du plan, dans l'ordre où ils seront posés.
    fn ordre_de_pose(planned: &Planned) -> Vec<&str> {
        planned
            .poses
            .iter()
            .map(|pose| pose.name.as_str())
            .collect()
    }

    /// Planifie puis applique, comme la commande le fait.
    fn run(options: &Options) -> Result<Planned, Error> {
        let planned = plan_for(options)?;
        crate::plan::application::apply(&planned.plan, options.force)?;

        Ok(planned)
    }

    /// Fait du projet un dépôt dont tout est commité.
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

    // SQLite n'a pas de serveur : un service `db` que rien ne peut atteindre ferait
    // échouer `docker compose up` sur une image qui n'a rien à faire là. Docker garde
    // son autre rôle, conteneuriser l'application.
    #[test]
    fn a_sqlite_project_gets_a_compose_without_a_database_service() {
        let (_parent, root) = project_on(Database::Sqlite);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(
            !compose.contains("  db:"),
            "le compose monte une base pour SQLite :\n{compose}"
        );
        assert!(
            !compose.contains("condition: service_healthy"),
            "le compose fait attendre un service sur une base absente :\n{compose}"
        );
        assert!(
            compose.contains("sqlite://"),
            "le compose ne porte pas l'URL SQLite :\n{compose}"
        );
    }

    /// SQLite n'a pas de serveur : `migrate` et `api` se partagent un fichier, et sans ce
    /// volume chacun travaillerait sur le sien — la migration n'atteindrait jamais la base
    /// que l'API ouvre.
    #[test]
    fn a_sqlite_project_shares_its_database_file_between_migrate_and_api() {
        let (_parent, root) = project_on(Database::Sqlite);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert_eq!(
            compose.matches("- sqlitedata:/data").count(),
            2,
            "migrate et api doivent monter le même volume :\n{compose}"
        );
    }

    #[test]
    fn a_mysql_project_gets_a_mysql_service() {
        let (_parent, root) = project_on(Database::Mysql);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(
            compose.contains("image: mysql:"),
            "le compose ne monte pas MySQL :\n{compose}"
        );
        assert!(
            !compose.contains("postgres"),
            "le compose nomme encore PostgreSQL :\n{compose}"
        );
    }

    // Les services de GitHub Actions sont des conteneurs : en réclamer un pour SQLite
    // ferait attendre la CI sur une image qui n'a rien à servir.
    #[test]
    fn a_sqlite_project_gets_a_workflow_without_a_database_service() {
        let (_parent, root) = project_on(Database::Sqlite);

        let planned = plan_for(&options(&root, "ci")).expect("le plan doit se calculer");
        let workflow = projected(&planned, ".github/workflows/ci.yml");

        assert!(
            !workflow.contains("services:"),
            "le workflow réclame un service pour SQLite :\n{workflow}"
        );
        assert!(
            workflow.contains("sqlite://"),
            "le workflow ne porte pas l'URL SQLite :\n{workflow}"
        );
    }

    #[test]
    fn a_mysql_project_gets_a_workflow_service_on_mysql() {
        let (_parent, root) = project_on(Database::Mysql);

        let planned = plan_for(&options(&root, "ci")).expect("le plan doit se calculer");
        let workflow = projected(&planned, ".github/workflows/ci.yml");

        assert!(
            workflow.contains("image: mysql:"),
            "le workflow ne monte pas MySQL :\n{workflow}"
        );
        assert!(
            !workflow.contains("postgres"),
            "le workflow nomme encore PostgreSQL :\n{workflow}"
        );
    }

    // Tout test qui joint la base est `#[ignore]` pour qu'un `cargo test` reste rapide sur
    // le poste ; la CI, elle, vient de monter cette base et de la migrer : sans
    // `--include-ignored`, elle passait verte sans exécuter un seul test de CRUD.
    #[test]
    fn the_workflow_runs_the_tests_that_reach_the_database() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "ci")).expect("le plan doit se calculer");
        let workflow = projected(&planned, ".github/workflows/ci.yml");

        assert!(
            workflow.contains("-- --include-ignored"),
            "le workflow laisse de côté les tests ignorés :\n{workflow}"
        );
        assert!(
            workflow.contains("--no-fail-fast"),
            "un binaire de test rouge masquerait les suivants :\n{workflow}"
        );
    }

    /// L'inventaire est ce que l'agent lit pour savoir ce que le projet porte : une
    /// feature installée qui n'y figure pas le renvoie explorer le disque.
    #[test]
    fn installing_a_feature_names_it_in_the_agents_inventory() {
        let (_parent, root) = project();

        let planned = run(&Options {
            features: vec!["redis".to_string()],
            directory: root.clone(),
            force: false,
            template_dir: None,
        })
        .expect("le plan doit se calculer");

        let agents = projected(&planned, "AGENTS.md");

        assert!(agents.contains("redis"), "{agents}");
        assert!(
            agents.contains("## Notes du projet"),
            "l'écriture a débordé de la zone"
        );
    }

    /// L'inventaire décrit le projet tel que le plan le laissera, ancres comprises.
    ///
    /// Le fragment `docker` écrit le compose d'un projet qui n'en a pas, et c'est ce
    /// fichier qui porte l'ancre `services`. Interrogé sur le disque, l'inventaire
    /// l'omettait, quand `rbs doctor` — qui relit le disque après écriture — l'y attendait
    /// aussitôt : la commande suivant `rbs new` rendait déjà un rapport rouge.
    #[test]
    fn installing_docker_on_a_project_without_a_compose_names_the_services_anchor() {
        let (_parent, root) = project();
        std::fs::remove_file(root.join("docker-compose.yml")).expect("le compose existe");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let agents = projected(&planned, "AGENTS.md");

        assert!(
            agents.contains("services (docker-compose.yml)"),
            "l'inventaire ignore l'ancre que ce plan apporte :\n{agents}"
        );
    }

    /// Un projet SQLite n'a pas de compose, et `auth` entraîne `mail`, dont le service
    /// `mailpit` n'a nulle part où aller. L'installation refusait tout net — treize routes
    /// inaccessibles au preset SQLite — là où seul le service manque : le plan se calcule,
    /// n'invente pas de compose, et dit ce qu'il reste à monter.
    #[test]
    fn adding_auth_to_a_sqlite_project_without_a_compose_plans_and_names_the_service() {
        let (_parent, root) = project_on(Database::Sqlite);
        assert!(
            !root.join("docker-compose.yml").exists(),
            "un projet SQLite n'a pas de compose"
        );

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert!(
            !planned
                .plan
                .files()
                .iter()
                .any(|f| f.path == "docker-compose.yml"),
            "le plan ne doit pas inventer de compose"
        );
        let sautees = planned.plan.sautees();
        assert_eq!(sautees.len(), 1, "{sautees:?}");
        assert_eq!(sautees[0].anchor, crate::anchors::SERVICES);

        let rendered = plan::render::plan(&planned.plan);
        assert!(
            rendered.contains("docker-compose.yml absent"),
            "le rendu ne nomme pas le fichier absent :\n{rendered}"
        );
        assert!(
            rendered.contains("mailpit:"),
            "le rendu ne nomme pas le service à monter :\n{rendered}"
        );
    }

    /// Un fichier de documentation supprimé ne doit pas empêcher d'installer une feature.
    #[test]
    fn a_missing_agents_file_does_not_stop_the_installation() {
        let (_parent, root) = project();
        std::fs::remove_file(root.join("AGENTS.md")).expect("le fichier existe");

        let planned = run(&Options {
            features: vec!["redis".to_string()],
            directory: root,
            force: false,
            template_dir: None,
        });

        assert!(planned.is_ok(), "{:?}", planned.err());
    }

    /// Le contenu qu'un plan projette pour `path`.
    fn projected<'plan>(planned: &'plan Planned, path: &str) -> &'plan str {
        &planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} absent du plan"))
            .after
    }

    /// Le compose du squelette existe déjà : seuls `Dockerfile` et `.dockerignore` sont
    /// déposés, le compose recevant ses services par insertion (voir plus bas).
    #[test]
    fn the_docker_plan_creates_its_two_files_and_records_the_feature() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        assert_eq!(planned.files, ["Dockerfile", ".dockerignore"]);

        let manifest = projected(&planned, "Cargo.toml");
        assert!(
            manifest.contains("features = [\"health\", \"docker\"]"),
            "la feature n'est pas inscrite dans le manifeste projeté :\n{manifest}"
        );
    }

    /// Le fragment `mail` lit `templates/mail` à l'exécution : une image qui ne l'embarque
    /// pas n'envoie aucun courriel, et rien ne le dit avant le premier `register`.
    #[test]
    fn the_dockerfile_ships_the_templates_directory() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        let dockerfile = projected(&planned, "Dockerfile");
        assert!(
            dockerfile.contains("COPY --from=builder /build/templates* ./templates/"),
            "l'étage runtime n'embarque pas templates/ :\n{dockerfile}"
        );
    }

    /// Le manifeste de `ci` désigne sa template par son chemin dans le fragment, répertoire
    /// caché compris : une déclaration fautive rendrait `TemplateAbsente` au lieu du plan.
    #[test]
    fn the_ci_plan_creates_the_workflow_its_manifest_declares() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "ci")).expect("le plan doit se calculer");

        assert_eq!(
            planned.files,
            [".github/workflows/ci.yml", ".github/dependabot.yml"]
        );
    }

    /// Trois états, trois comportements. Le premier : le projet a son compose, `add
    /// docker` n'y ajoute que ce qui manque.
    #[test]
    fn adding_docker_to_a_project_with_a_compose_inserts_its_services() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert_eq!(
            compose.matches("image: postgres:18-alpine").count(),
            1,
            "le service de base ne doit pas être doublé :\n{compose}"
        );
        assert!(compose.contains("profiles: [\"app\"]"), "{compose}");
        assert!(
            compose.contains("command: [\"migration\", \"up\"]"),
            "{compose}"
        );
        assert!(
            !planned.files.iter().any(|f| f == "docker-compose.yml"),
            "le compose n'est pas déposé mais inséré : {:?}",
            planned.files
        );
    }

    /// Le deuxième : un projet créé avant la 1.1.0 n'a pas de compose. Le fragment lui en
    /// écrit un entier, ancre comprise, sans quoi il n'aurait aucun moyen d'en obtenir un.
    #[test]
    fn adding_docker_to_a_project_without_a_compose_writes_the_whole_file() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("image: postgres:18-alpine"), "{compose}");
        assert!(compose.contains("# <rbs:services>"), "{compose}");
        assert_eq!(
            compose.matches("profiles: [\"app\"]").count(),
            2,
            "api et migrate, une fois chacun :\n{compose}"
        );
        assert!(planned.files.iter().any(|f| f == "docker-compose.yml"));
    }

    /// Le troisième : un compose réécrit à la main a perdu son ancre. Le CLI n'écrit
    /// rien et affiche le bloc à recoller — la convention du projet.
    #[test]
    fn adding_docker_to_a_compose_without_its_anchor_refuses_and_shows_the_block() {
        let (_parent, root) = project();
        fs::write(
            root.join("docker-compose.yml"),
            "services:\n  db:\n    image: postgres\n",
        )
        .expect("écriture possible");

        let error = plan_for(&options(&root, "docker")).expect_err("l'ancre manque");

        let message = error.to_string();
        assert!(message.contains("docker-compose.yml"), "{message}");
        assert!(
            error
                .remedy()
                .is_some_and(|r| r.contains("# <rbs:services>")),
            "le bloc à coller doit être affiché : {error:?}"
        );
    }

    /// L'URL que `migrate` et `api` reçoivent nomme les identifiants au lieu de les
    /// écrire : le compose est versionné, et Compose les substitue à l'exécution depuis le
    /// `.env`, qui ne l'est pas. Les mêmes clés que celles du service `db`, et pas d'autres.
    #[test]
    fn the_internal_url_names_the_credentials_rather_than_carrying_them() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert_eq!(
            compose
                .matches(
                    "RBS_DATABASE__URL: \
                     \"postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@db:5432/${POSTGRES_DB}\""
                )
                .count(),
            2,
            "migrate et api, une fois chacun :\n{compose}"
        );
        assert!(
            !compose.contains("postgres://rbs:rbs@db"),
            "l'URL interne porte encore les identifiants du projet :\n{compose}"
        );
    }

    /// Le compose que le fragment écrit en entier — celui d'un projet qui n'en a pas —
    /// porte la même URL que celui qu'il complète par l'ancre : ce sont deux textes
    /// distincts, et le mot de passe ne doit sortir par aucun des deux.
    #[test]
    fn the_whole_compose_names_the_credentials_too() {
        let (_parent, root) = project();
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert_eq!(
            compose
                .matches(
                    "RBS_DATABASE__URL: \
                     \"postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@db:5432/${POSTGRES_DB}\""
                )
                .count(),
            2,
            "migrate et api, une fois chacun :\n{compose}"
        );
    }

    /// Le trou que ce fragment laissait : un projet créé avant que `rbs new` écrive les
    /// clés du service `db` n'en porte aucune. Le compose les interpole ; sans cet ajout,
    /// Compose y substitue une chaîne vide et la base monte sans mot de passe.
    #[test]
    fn installing_docker_writes_the_compose_credentials_into_an_older_env() {
        let (_parent, root) = project();
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);

        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_USER"),
            Some("rbs"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_PASSWORD"),
            Some("rbs"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_DB"),
            Some("demo_api"),
            "{env}"
        );
    }

    /// L'exemple versionné documente les mêmes clés — `doctor` compare l'un à l'autre —
    /// avec les valeurs de démonstration du moteur, jamais celles du projet.
    #[test]
    fn the_versioned_example_documents_the_keys_with_demonstration_values() {
        let (_parent, root) = project();
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let exemple = projected(&planned, ".env.example");
        let paires = crate::dotenv::parse(exemple);

        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_USER"),
            Some("postgres"),
            "{exemple}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_PASSWORD"),
            Some("postgres"),
            "{exemple}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_DB"),
            Some("demo_api"),
            "{exemple}"
        );
    }

    /// Une clé que le `.env` porte déjà n'est ni redéclarée ni réécrite : deux
    /// `POSTGRES_PASSWORD` dans un même fichier, et c'est la dernière ligne qui gagne —
    /// l'installation écraserait le mot de passe que le développeur y a mis.
    #[test]
    fn credentials_already_in_the_env_are_neither_duplicated_nor_overwritten() {
        let (_parent, root) = project();
        let env = fs::read_to_string(root.join(".env")).expect("le .env doit exister");
        fs::write(
            root.join(".env"),
            env.replace("POSTGRES_PASSWORD=rbs", "POSTGRES_PASSWORD=le-mien"),
        )
        .expect("le .env doit se réécrire");
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let after = projected(&planned, ".env");

        assert_eq!(
            after.matches("POSTGRES_PASSWORD=").count(),
            1,
            "la clé est déclarée deux fois :\n{after}"
        );
        assert_eq!(
            crate::dotenv::value(&crate::dotenv::parse(after), "POSTGRES_PASSWORD"),
            Some("le-mien"),
            "{after}"
        );
    }

    /// Le mot de passe du projet ne doit ressortir par aucune porte : ni les variables du
    /// service `db`, ni l'URL de `migrate` et d'`api`, ni l'exemple. Le seul fichier qui a
    /// le droit de le porter est le `.env`, que le `.gitignore` du projet couvre.
    #[test]
    fn the_project_password_reaches_no_versioned_file() {
        let (_parent, root) = project_with(
            Database::Postgres,
            "postgres://u:a'b:c$(id)@localhost:5432/demo_api",
        );
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        for file in planned.plan.files() {
            if file.path == ".env" {
                continue;
            }
            assert!(
                !file.after.contains("a'b:c$(id)"),
                "{} porte le mot de passe du projet :\n{}",
                file.path,
                file.after
            );
        }

        let env = projected(&planned, ".env");
        assert_eq!(
            crate::dotenv::value(&crate::dotenv::parse(env), "POSTGRES_PASSWORD"),
            Some("a'b:c$(id)"),
            "le .env doit porter le mot de passe réel :\n{env}"
        );
    }

    /// MySQL ne nomme pas ses clés comme PostgreSQL, et son image ne crée `MYSQL_USER`
    /// que pour un compte autre que `root` : en déclarer un ici ferait échouer l'image,
    /// qui refuse qu'on lui redemande le compte d'administration.
    #[test]
    fn a_mysql_project_on_root_gets_the_root_password_and_no_second_account() {
        let (_parent, root) = project_on(Database::Mysql);
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);

        assert_eq!(
            crate::dotenv::value(&paires, "MYSQL_ROOT_PASSWORD"),
            Some("root"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "MYSQL_DATABASE"),
            Some("demo_api"),
            "{env}"
        );
        assert_eq!(crate::dotenv::value(&paires, "MYSQL_USER"), None, "{env}");

        let compose = projected(&planned, "docker-compose.yml");
        assert!(
            compose.contains(
                "RBS_DATABASE__URL: \"mysql://root:${MYSQL_ROOT_PASSWORD}@db:3306/${MYSQL_DATABASE}\""
            ),
            "{compose}"
        );
    }

    /// Un projet MySQL qui se connecte sous un autre compte : c'est celui-là que l'image
    /// doit créer, et celui-là que l'URL interne nomme.
    #[test]
    fn a_mysql_project_on_another_account_gets_that_account_created() {
        let (_parent, root) = project_with(
            Database::Mysql,
            "mysql://app:s3cr3t@localhost:3306/demo_api",
        );
        avant_les_cles_du_compose(&root);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);

        assert_eq!(
            crate::dotenv::value(&paires, "MYSQL_USER"),
            Some("app"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "MYSQL_PASSWORD"),
            Some("s3cr3t"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "MYSQL_ROOT_PASSWORD"),
            Some("s3cr3t"),
            "l'image refuse de s'initialiser sans mot de passe root :\n{env}"
        );

        let compose = projected(&planned, "docker-compose.yml");
        assert!(
            compose.contains(
                "RBS_DATABASE__URL: \"mysql://${MYSQL_USER}:${MYSQL_PASSWORD}@db:3306/${MYSQL_DATABASE}\""
            ),
            "{compose}"
        );
        assert!(
            !compose.contains("s3cr3t"),
            "le compose versionné porte le mot de passe :\n{compose}"
        );
    }

    /// SQLite n'a pas de serveur : aucune clé d'identifiants n'a de sens, et en écrire
    /// une ferait croire à un service que le compose ne monte pas.
    #[test]
    fn a_sqlite_project_gets_no_database_credentials() {
        let (_parent, root) = project_on(Database::Sqlite);

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        for file in planned.plan.files() {
            for cle in ["POSTGRES_", "MYSQL_"] {
                assert!(
                    !file.after.contains(cle),
                    "{} porte une clé `{cle}` sur un projet SQLite :\n{}",
                    file.path,
                    file.after
                );
            }
        }
    }

    /// Le worker de la file ne peut se détacher que d'un endroit du squelette, et le
    /// fragment doit l'y viser : sans cette ligne, la file se remplit et rien ne la vide.
    #[test]
    fn the_jobs_plan_lands_its_worker_in_the_startup_anchor() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "jobs")).expect("le plan doit se calculer");

        let main = projected(&planned, "src/main.rs");
        let startup = main
            .split_once("// <rbs:startup>")
            .and_then(|(_, apres)| apres.split_once("// </rbs:startup>"))
            .map(|(dedans, _)| dedans)
            .expect("le squelette doit porter l'ancre de démarrage");

        assert!(
            startup.contains("demo_api::modules::jobs::worker::spawn(state.clone());"),
            "le worker n'est pas détaché au démarrage :\n{main}"
        );

        let configuration = projected(&planned, "config/default.toml");
        for cle in [
            "max_attempts",
            "retry_delay_secs",
            "retry_max_delay_secs",
            "poll_interval_secs",
            "lease_secs",
            "concurrency",
        ] {
            assert!(
                configuration.contains(cle),
                "`{cle}` manque à la section [jobs] :\n{configuration}"
            );
        }
    }

    /// Le critère de la tâche : le mot de passe SMTP n'entre dans le projet que par
    /// l'environnement.
    ///
    /// `config/default.toml` est versionné et `.env.example` ne porte que des valeurs
    /// d'exemple : un secret qui atterrirait dans le premier serait commité par le
    /// développeur sans qu'il l'ait décidé.
    #[test]
    fn the_smtp_password_lives_in_the_environment_and_in_no_configuration() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "mail")).expect("le plan doit se calculer");

        let env = projected(&planned, ".env.example");
        assert!(
            env.contains("RBS_MAIL__SMTP_PASSWORD="),
            "le secret n'est pas déclaré dans .env.example :\n{env}"
        );

        let configurations: Vec<&crate::plan::File> = planned
            .plan
            .files()
            .iter()
            .filter(|file| file.path.starts_with("config/"))
            .collect();

        assert!(
            !configurations.is_empty(),
            "le fragment n'écrit aucune configuration : le test ne prouverait rien"
        );

        for file in configurations {
            // Les commentaires sont exclus : ce que figment lit, ce sont les clés, et
            // renvoyer le lecteur vers la variable d'environnement est précisément le
            // rôle d'un commentaire de `config/default.toml`.
            let keys: String = file
                .after
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();

            assert!(
                !keys.contains("password"),
                "{} porte le secret en clé de configuration :\n{}",
                file.path,
                file.after
            );
        }
    }

    /// Le critère de la tâche : `add auth` ne publie pas le secret qu'il installe.
    ///
    /// L'exemple versionné garde son placeholder — c'est à lui que `doctor` compare le
    /// `.env` — pendant que le `.env`, gitignoré, reçoit une valeur propre au projet.
    #[test]
    fn adding_auth_draws_the_signing_secret_into_the_env() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        let exemple = projected(&planned, ".env.example");
        assert!(
            exemple.contains("RBS_AUTH__SECRET="),
            "le secret n'est pas déclaré dans .env.example :\n{exemple}"
        );

        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);
        let tire = crate::dotenv::value(&paires, "RBS_AUTH__SECRET")
            .expect("le .env doit porter le secret");

        assert_eq!(tire.len(), 64, "{env}");
        assert!(
            !exemple.contains(tire),
            "la valeur du .env est celle que l'exemple versionné publie :\n{exemple}"
        );
    }

    /// Inscrit `feature` dans le manifeste comme le ferait `rbs generate crud`, sans
    /// engendrer de CRUD complet : `add::plan_for` ne lit que la liste, jamais le disque
    /// sous `src/`.
    fn inscrire_feature(root: &Path, feature: &str) {
        let cargo = root.join("Cargo.toml");
        let source = fs::read_to_string(&cargo).expect("le manifeste doit exister");
        let reecrit = source.replacen(
            "features = [\"health\"]",
            &format!("features = [\"health\", \"{feature}\"]"),
            1,
        );

        assert_ne!(
            source, reecrit,
            "le manifeste ne porte pas la liste attendue :\n{source}"
        );
        fs::write(&cargo, reecrit).expect("le manifeste doit se réécrire");
    }

    /// Le critère de la tâche : un CRUD généré avant `auth` reste ouvert — le CLI ne
    /// réécrit pas un fichier existant — et se taire ferait croire l'API fermée.
    #[test]
    fn installing_auth_names_the_cruds_that_stay_open() {
        let (_parent, root) = project();
        inscrire_feature(&root, "posts");

        let planned = plan_for(&options(&root, "auth")).expect("l'installation doit se planifier");

        let message = planned.remedy().expect("un avertissement doit être rendu");

        assert!(
            message.contains("posts"),
            "le module déjà présent doit être nommé : {message}"
        );
        assert!(
            !message.contains("health"),
            "un fragment posé par `rbs new` n'est pas un CRUD : {message}"
        );
    }

    /// `auth` entraîne parfois d'autres fragments (`rate-limit`), mais ceux-ci ne sont
    /// pas des CRUD : les nommer ferait croire à un handler à fermer qui n'existe pas.
    #[test]
    fn installing_auth_does_not_name_the_fragments_it_drags_in() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("l'installation doit se planifier");

        assert!(
            planned.remedy().is_none(),
            "aucun CRUD n'est présent, aucun message ne doit sortir : {:?}",
            planned.remedy()
        );
    }

    /// Le message ne concerne qu'`auth` : les autres fragments ne ferment aucune route,
    /// et nommer un CRUD à leur installation induirait le développeur en erreur.
    #[test]
    fn installing_another_feature_names_no_open_crud() {
        let (_parent, root) = project();
        inscrire_feature(&root, "posts");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        assert!(
            planned.remedy().is_none(),
            "docker ne ferme aucune route : {:?}",
            planned.remedy()
        );
    }

    /// Ramène `src/state.rs` à ce qu'était un projet créé avant la 1.5.0 : l'ancre
    /// `state_init` sous `core: CoreState::new(db, config)`, qui a déjà consommé `config`.
    fn ancre_state_init_sous_core(root: &Path) {
        let state = root.join("src/state.rs");
        let source = fs::read_to_string(&state).expect("state.rs doit exister");
        let bloc = "            // <rbs:state_init>\n            // </rbs:state_init>\n";
        let core = "            core: CoreState::new(db, config),\n";
        let courant = format!("{bloc}{core}");

        assert!(
            source.contains(&courant),
            "le squelette doit poser l'ancre juste au-dessus de `core:` :\n{source}"
        );
        fs::write(&state, source.replace(&courant, &format!("{core}{bloc}")))
            .expect("state.rs doit se réécrire");
    }

    /// Sur un projet d'avant 1.5.0, `webhooks` lirait `config` après que `core:` l'a
    /// consommé : le projet ne compilerait plus, et seul `cargo build` le dirait. Le plan
    /// refuse, en nommant la ligne à faire précéder et le bloc à remonter.
    #[test]
    fn webhooks_refuses_a_state_init_anchor_left_below_core_and_shows_the_block_to_move() {
        let (_parent, root) = project();
        ancre_state_init_sous_core(&root);

        let error = plan_for(&options(&root, "webhooks")).expect_err("le plan doit refuser");

        assert!(
            matches!(
                error,
                Error::Installation(installation::Error::Plan(plan::Error::MalPlacee(_)))
            ),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            "ancre // <rbs:state_init> placée sous `core: CoreState::new(` dans src/state.rs, \
             qu'elle doit précéder"
        );
        let remedy = error.remedy().expect("le remède tient en un bloc");
        assert_eq!(
            remedy,
            "dans src/state.rs, remontez ce bloc au-dessus de `core: CoreState::new(` :\n\
             // <rbs:state_init>\n// </rbs:state_init>"
        );
    }

    /// Le même projet, l'ancre à sa place : rien ne change pour les fragments qui ne
    /// lisent pas `config`, ni pour `webhooks` sur un projet courant.
    #[test]
    fn webhooks_plans_on_a_project_whose_state_init_anchor_precedes_core() {
        let (_parent, root) = project();

        plan_for(&options(&root, "webhooks")).expect("le plan doit se calculer");
    }

    /// `webhooks` entraîne `auth` : un CRUD déjà présent reste ouvert par ce chemin
    /// comme par l'installation directe, et le message doit le dire dans les deux cas.
    #[test]
    fn auth_entrained_by_another_fragment_still_names_open_cruds() {
        let (_parent, root) = project();
        inscrire_feature(&root, "posts");

        let planned = plan_for(&options(&root, "webhooks")).expect("le plan doit se calculer");

        let message = planned
            .remedy()
            .expect("auth arrive par entraînement, l'avertissement doit sortir");
        assert!(message.contains("posts"), "{message}");
    }

    /// Le fragment annonçait redis://127.0.0.1:6379 dans config/default.toml sans que
    /// rien y réponde. Le service le sert, et sans profil : c'est une dépendance de
    /// développement, que `rbs dev` doit monter.
    #[test]
    fn adding_redis_serves_the_url_its_config_announces() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "redis")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("redis:8-alpine"), "{compose}");
        assert!(compose.contains("- \"6379:6379\""), "{compose}");
        assert!(
            !compose.contains("profiles"),
            "un service de développement n'a pas de profil :\n{compose}"
        );
    }

    #[test]
    fn adding_mail_serves_the_smtp_port_its_config_announces() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "mail")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("axllent/mailpit"), "{compose}");
        assert!(compose.contains("- \"1025:1025\""), "{compose}");
        assert!(compose.contains("- \"8025:8025\""), "{compose}");
    }

    /// Deux fragments dans un même compose ne se marchent pas dessus : chacun a son
    /// service, et le fichier reste du YAML.
    #[test]
    fn two_fragments_share_the_same_anchor_without_colliding() {
        let (_parent, root) = project();

        run(&options(&root, "redis")).expect("la première pose doit aboutir");
        let planned = plan_for(&options(&root, "mail")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("redis:8-alpine"), "{compose}");
        assert!(compose.contains("axllent/mailpit"), "{compose}");
        assert_eq!(
            compose.matches("image: postgres:18-alpine").count(),
            1,
            "{compose}"
        );
        // `db`, `redis` et `mailpit` ouvrent chacun un `ports:` : une ligne nue déjà posée
        // par redis ne doit pas faire disparaître celle de mail, laissant sa liste de
        // ports orpheline.
        assert_eq!(
            compose.matches("ports:").count(),
            3,
            "un des trois services a perdu son en-tête ports: :\n{compose}"
        );
        assert!(compose.contains("- \"1025:1025\""), "{compose}");
    }

    /// Le corps d'une ancre dans le contenu que le plan projette pour son fichier.
    fn anchor_body<'plan>(planned: &'plan Planned, anchor: &crate::anchors::Anchor) -> &'plan str {
        let source = projected(planned, anchor.file.as_ref());

        source
            .split_once(&anchor.opening())
            .and_then(|(_, apres)| apres.split_once(&anchor.closing()))
            .map(|(dedans, _)| dedans)
            .unwrap_or_else(|| panic!("{} ne porte pas {}", anchor.file, anchor.name))
    }

    /// Une dépendance installée doit être contrôlée : sans sa sonde, `GET /health`
    /// répondrait `ok` sur un cache ou un bucket injoignable, et l'orchestrateur
    /// garderait le pod en rotation.
    #[test]
    fn the_redis_and_storage_plans_land_their_probe_in_the_health_anchor() {
        for (fragment, sonde) in [
            (
                "redis",
                r#"rbs_core::health::Probe::new("cache", state.cache().ping()),"#,
            ),
            (
                "storage",
                r#"crate::modules::storage::probe(state.storage()),"#,
            ),
        ] {
            let (_parent, root) = project();

            let planned = plan_for(&options(&root, fragment)).expect("le plan doit se calculer");

            assert!(
                anchor_body(&planned, &crate::anchors::HEALTH_PROBES).contains(sonde),
                "{}",
                projected(&planned, "src/health/controller.rs")
            );
        }
    }

    /// La file de `jobs` est une table de la base, et sonder un relais SMTP à chaque
    /// contrôle coûterait cher pour un envoi que rien ne rend synchrone : ni l'un ni
    /// l'autre n'a de sonde, et l'absence se teste comme la présence.
    #[test]
    fn the_jobs_and_mail_fragments_declare_no_probe() {
        for fragment in ["jobs", "mail"] {
            let (_parent, root) = project();

            let planned = plan_for(&options(&root, fragment)).expect("le plan doit se calculer");

            assert!(
                !planned
                    .plan
                    .files()
                    .iter()
                    .any(|file| file.path == crate::anchors::HEALTH_PROBES.file),
                "`{fragment}` a touché au contrôle de santé"
            );
        }
    }

    /// Une couche se pose dans `layers`, jamais dans `routes` : montée parmi les routes,
    /// elle n'envelopperait rien.
    #[test]
    fn the_cors_plan_lands_its_layer_in_the_layers_anchor() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "cors")).expect("le plan doit se calculer");

        assert_eq!(
            planned.files,
            [
                "src/modules/cors/mod.rs",
                "src/modules/cors/config.rs",
                "src/modules/cors/tests.rs"
            ]
        );
        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS)
                .contains(".layer(crate::modules::cors::layer())"),
            "{}",
            projected(&planned, "src/router.rs")
        );
        assert!(
            !anchor_body(&planned, &crate::anchors::ROUTES).contains("cors"),
            "la couche s'est montée parmi les routes"
        );

        // Le squelette déclare déjà `tower-http` pour la borne de durée et la compression :
        // le fragment ajoute sa feature à celles qui sont là plutôt qu'une seconde
        // déclaration.
        let manifeste = projected(&planned, "Cargo.toml");
        assert!(
            manifeste.contains(
                "tower-http = { version = \"0.7\", features = [\"timeout\", \
                 \"compression-gzip\", \"cors\"] }"
            ),
            "{manifeste}"
        );
    }

    /// Le critère de la tâche : le défaut n'ouvre l'API à personne. Un `Any` en dur serait
    /// le pendant exact du trou que la limite de débit vient boucher.
    #[test]
    fn the_cors_plan_authorises_no_origin_by_default() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "cors")).expect("le plan doit se calculer");
        let configuration = projected(&planned, "config/default.toml");

        assert!(configuration.contains("origins = []"), "{configuration}");
        assert!(
            configuration.contains("credentials = false"),
            "{configuration}"
        );
    }

    /// Le fragment apporte son état, sa couche et sa section : trois points d'entrée
    /// distincts, qu'un manifeste incomplet laisserait passer sans que rien ne compile
    /// de travers.
    #[test]
    fn the_rate_limit_plan_lands_its_layer_its_state_and_its_section() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");

        assert_eq!(
            planned.files,
            [
                "src/modules/rate_limit/mod.rs",
                "src/modules/rate_limit/config.rs",
                "src/modules/rate_limit/counter.rs",
                "src/modules/rate_limit/tests.rs",
            ]
        );

        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS)
                .contains("crate::modules::rate_limit::middleware"),
            "{}",
            projected(&planned, "src/router.rs")
        );
        assert!(
            anchor_body(&planned, &crate::anchors::STATE_INIT)
                .contains("RateLimiter::from_config()?"),
            "{}",
            projected(&planned, "src/state.rs")
        );

        let configuration = projected(&planned, "config/default.toml");
        assert!(configuration.contains("[rate_limit]"), "{configuration}");
        assert!(
            configuration.contains("trust_forwarded_for"),
            "{configuration}"
        );
    }

    /// `[package.metadata.rbs] lang` ne gouverne plus que `AGENTS.md` : un projet dont la
    /// clé vaut `en` mais dont `config/default.toml` ne porte pas de `[server] lang`
    /// reste français, comme le serveur qui répondrait avec la même config.
    #[test]
    fn a_metadata_lang_without_a_server_lang_still_renders_french_messages() {
        let (_parent, root) = project();
        viser_lang_metadata(&root, "en");
        retirer_lang_server(&root);

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");
        let module = projected(&planned, "src/modules/rate_limit/mod.rs");

        assert!(
            module.contains("trop de requêtes : réessayez plus tard"),
            "{module}"
        );
        assert!(
            !module.contains("too many requests: try again later"),
            "{module}"
        );
    }

    /// `[server] lang` décide seul, même contredite par la métadonnée : c'est elle que
    /// `rbs-core` lira au démarrage pour la même réponse 429.
    #[test]
    fn a_server_lang_overrides_a_diverging_metadata_lang() {
        let (_parent, root) = project();
        viser_lang_metadata(&root, "fr");
        viser_lang_server(&root, "en");

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");
        let module = projected(&planned, "src/modules/rate_limit/mod.rs");

        assert!(
            module.contains("too many requests: try again later"),
            "{module}"
        );
        assert!(
            !module.contains("trop de requêtes : réessayez plus tard"),
            "{module}"
        );
    }

    /// Le critère de la tâche 12 : la route qui hache un Argon2 par requête anonyme est
    /// limitée bien plus serré que le reste de l'API.
    #[test]
    fn the_rate_limit_plan_holds_the_login_route_stricter_than_the_global_limit() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");
        let configuration = projected(&planned, "config/default.toml");

        assert!(
            configuration.contains("{ path = \"/auth/login\", limit = 5, window_secs = 60 }"),
            "{configuration}"
        );
        assert!(
            configuration.contains("limit = 120"),
            "la limite globale doit rester bien plus large :\n{configuration}"
        );
    }

    /// Le fragment a trois points d'entrée distincts — un module, une couche, un second
    /// listener — plus sa section. Un manifeste incomplet en laisserait passer un sans
    /// que rien ne compile de travers : la feature s'installerait et ne compterait rien.
    #[test]
    fn the_observability_plan_lands_its_layer_its_listener_and_its_section() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "observability")).expect("le plan doit se calculer");

        assert_eq!(
            planned.files,
            [
                "src/modules/observability/mod.rs",
                "src/modules/observability/config.rs",
                "src/modules/observability/metrics.rs",
                "src/modules/observability/tests.rs",
            ]
        );

        let bibliotheque = projected(&planned, "src/lib.rs");
        assert!(bibliotheque.contains("pub mod modules;"), "{bibliotheque}");
        let montage = projected(&planned, "src/modules/mod.rs");
        assert!(montage.contains("pub mod observability;"), "{montage}");
        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS)
                .contains("crate::modules::observability::metrics::middleware"),
            "{}",
            projected(&planned, "src/router.rs")
        );
        assert!(
            anchor_body(&planned, &crate::anchors::STARTUP)
                .contains("demo_api::modules::observability::serve(&state).await?;"),
            "{}",
            projected(&planned, "src/main.rs")
        );

        let configuration = projected(&planned, "config/default.toml");
        assert!(configuration.contains("[observability]"), "{configuration}");
        assert!(
            configuration.contains("metrics_port = 9090"),
            "{configuration}"
        );
    }

    /// Les helpers du fragment portent les noms que rend le gabarit d'une feature.
    ///
    /// Les deux fichiers cohabitent dans le même projet dès qu'une entité est engendrée :
    /// deux conventions de nommage y feraient croire que l'un des deux a été écrit à la
    /// main, quand les deux sortent du CLI.
    #[test]
    fn the_observability_tests_name_their_helpers_as_the_feature_template_does() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "observability")).expect("le plan doit se calculer");

        let tests = projected(&planned, "src/modules/observability/tests.rs");
        for helper in ["fn registry()", "async fn call(uri: &str)"] {
            assert!(tests.contains(helper), "`{helper}` manque :\n{tests}");
        }
        for francais in ["appeler(", "registre(", "REGISTRE"] {
            assert!(
                !tests.contains(francais),
                "`{francais}` subsiste :\n{tests}"
            );
        }
    }

    /// `/metrics` publie la topologie du service : monté sur le routeur public, chaque
    /// déploiement devrait le cacher par une règle de reverse-proxy, et celui qui
    /// l'oublie fuit sans le savoir.
    #[test]
    fn the_metrics_route_is_never_mounted_on_the_public_router() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "observability")).expect("le plan doit se calculer");

        let routeur = projected(&planned, "src/router.rs");
        assert!(!routeur.contains("/metrics"), "{routeur}");
    }

    /// Le test qui garde la cardinalité du collecteur : le compteur prend le gabarit de
    /// route et jamais l'URL demandée. Une série par article ferait tomber le collecteur
    /// en quelques heures, et la feature deviendrait nuisible en production.
    #[test]
    fn the_observability_middleware_labels_requests_with_the_route_template() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "observability")).expect("le plan doit se calculer");
        let metriques = projected(&planned, "src/modules/observability/metrics.rs");

        assert!(metriques.contains("MatchedPath"), "{metriques}");
        assert!(
            !metriques.contains("uri().path()"),
            "le chemin demandé sert d'étiquette :\n{metriques}"
        );

        let tests = projected(&planned, "src/modules/observability/tests.rs");
        assert!(
            tests.contains("path=\\\"/articles/{id}\\\""),
            "les tests engendrés ne gardent pas la cardinalité :\n{tests}"
        );
    }

    /// Sans le fragment `redis`, le compteur vit dans le processus : rien à joindre, et
    /// aucune crate de plus.
    #[test]
    fn without_redis_the_counter_is_the_in_memory_one() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");
        let counter = projected(&planned, "src/modules/rate_limit/counter.rs");

        assert!(counter.contains("HashMap"), "{counter}");
        assert!(!counter.contains("deadpool_redis"), "{counter}");
    }

    /// Le fragment `redis` installé, le compteur passe sur son serveur : deux instances
    /// derrière un répartiteur doivent compter ensemble.
    #[test]
    fn with_redis_the_counter_becomes_the_shared_one() {
        let (_parent, root) = project();
        run(&options(&root, "redis")).expect("la pose du cache doit aboutir");

        let planned = plan_for(&options(&root, "rate-limit")).expect("le plan doit se calculer");
        let counter = projected(&planned, "src/modules/rate_limit/counter.rs");

        assert!(counter.contains("deadpool_redis"), "{counter}");
        assert!(
            counter.contains("crate::modules::cache::Config::load()"),
            "{counter}"
        );
        assert!(!counter.contains("HashMap"), "{counter}");
    }

    /// Des fragments planifiés ensemble se voient : `rate-limit` compte dans Redis dès que
    /// `redis` est du même plan, quel que soit l'ordre où la demande les nomme. Lu sur le
    /// disque plutôt que dans le plan, `features` ignorait `redis` tant que `rate-limit`
    /// passait avant lui — et `rbs new --with rate-limit,redis` rendait un compteur par
    /// réplica.
    #[test]
    fn rate_limit_planned_with_redis_renders_the_shared_counter_whatever_the_order() {
        for demande in [["rate-limit", "redis"], ["redis", "rate-limit"]] {
            let (_parent, root) = project();

            let planned =
                plan_for(&options_multi(&root, &demande)).expect("le plan doit se calculer");
            let counter = projected(&planned, "src/modules/rate_limit/counter.rs");

            assert!(
                counter.contains("deadpool_redis"),
                "{demande:?} rend un compteur en mémoire :\n{counter}"
            );
            assert!(!counter.contains("HashMap"), "{demande:?} :\n{counter}");
        }
    }

    /// `docker` écrit le compose que `mail` étend par l'ancre `services` : sur un projet
    /// SQLite, qui n'en porte pas, `auth` — qui entraîne `mail` — posée avant lui
    /// échouait sur une ancre introuvable.
    #[test]
    fn docker_is_laid_down_before_the_fragments_that_extend_its_compose() {
        let (_parent, root) = project_on(Database::Sqlite);
        assert!(!root.join("docker-compose.yml").exists());

        let planned =
            plan_for(&options_multi(&root, &["auth", "docker"])).expect("le plan doit se calculer");

        assert_eq!(
            ordre_de_pose(&planned),
            ["docker", "mail", "rate-limit", "auth"]
        );
        let compose = projected(&planned, "docker-compose.yml");
        assert!(
            compose.contains("mailpit:"),
            "le compose que docker apporte ne reçoit pas le service de mail :\n{compose}"
        );
    }

    /// Un fragment passe avant ceux qui l'exigent : ce qu'`auth` s'attend à trouver est
    /// déjà dans le plan quand elle se rend.
    #[test]
    fn auth_alone_lays_down_mail_and_rate_limit_before_itself() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert_eq!(ordre_de_pose(&planned), ["mail", "rate-limit", "auth"]);
    }

    /// Les ancres non triées empilent dans l'ordre de pose : cet ordre ne doit tenir qu'aux
    /// fragments, jamais à la frappe, pour que deux demandes équivalentes rendent le même
    /// projet.
    #[test]
    fn two_equivalent_requests_render_the_same_project() {
        let rendu = |demande: &[&str]| {
            let (_parent, root) = project();
            let planned =
                plan_for(&options_multi(&root, demande)).expect("le plan doit se calculer");

            (
                ordre_de_pose(&planned)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                projected(&planned, "src/state.rs").to_string(),
                projected(&planned, "docker-compose.yml").to_string(),
            )
        };

        let attendu = rendu(&["storage", "auth", "docker"]);
        assert_eq!(
            attendu.0,
            ["docker", "mail", "rate-limit", "auth", "storage"]
        );
        assert_eq!(attendu, rendu(&["docker", "auth", "storage"]));
        assert_eq!(attendu, rendu(&["auth", "storage", "docker"]));
    }

    /// Le critère de la tâche 12 : `rbs add auth` ne laisse pas `/auth/login` sans limite,
    /// et l'utilisateur le lit avant que quoi que ce soit ne s'écrive. `mail` s'y ajoute
    /// depuis qu'auth envoie des courriels : les deux entraînements sont vérifiés ici.
    #[test]
    fn adding_auth_announces_and_lays_down_the_rate_limit_and_mail_fragments() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert_eq!(planned.entrainees, ["mail", "rate-limit"]);
        for fichier in ["src/modules/rate_limit/mod.rs", "src/modules/mail/mod.rs"] {
            assert!(
                planned.files.iter().any(|file| file == fichier),
                "{:?}",
                planned.files
            );
        }

        let manifeste = projected(&planned, "Cargo.toml");
        assert!(
            manifeste.contains("features = [\"health\", \"mail\", \"rate-limit\", \"auth\"]"),
            "les trois features doivent être inscrites :\n{manifeste}"
        );
        assert!(
            projected(&planned, "config/default.toml").contains("/auth/login"),
            "la règle stricte de la route de connexion manque"
        );
    }

    /// Un fragment entraîné que le projet porte déjà n'est pas reposé : l'entraînement
    /// obéit à la même idempotence que l'installation directe.
    #[test]
    fn an_already_installed_requirement_is_not_laid_down_twice() {
        let (_parent, root) = project();
        run(&options(&root, "rate-limit")).expect("la première pose doit aboutir");
        run(&options(&root, "mail")).expect("la première pose doit aboutir");

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert!(planned.entrainees.is_empty(), "{:?}", planned.entrainees);
        assert!(
            !planned
                .files
                .iter()
                .any(|file| file.starts_with("src/modules/rate_limit/")),
            "{:?}",
            planned.files
        );
    }

    /// Deux fragments qui s'exigent l'un l'autre — ce qu'un `--template-dir` peut écrire —
    /// ne doivent pas faire descendre la résolution jusqu'au débordement de pile.
    #[test]
    fn two_fragments_requiring_each_other_do_not_loop() {
        let (_parent, root) = project();
        let fragments = TempDir::new().expect("répertoire temporaire créable");
        for (nom, exige) in [("essai", "autre"), ("autre", "essai")] {
            fs::create_dir(fragments.path().join(nom)).expect("le fragment se crée");
            fs::write(
                fragments.path().join(nom).join("feature.toml"),
                format!(
                    "[feature]\ndescription = \"{nom}\"\nrequires = [\"{exige}\"]\n\n\
                     [[anchors]]\nanchor = \"features\"\ncontent = \"pub mod {nom};\"\n"
                ),
            )
            .expect("le manifeste s'écrit");
        }

        let planned =
            plan_for(&fragment_options(&root, &fragments)).expect("le plan doit se calculer");

        assert_eq!(planned.entrainees, ["autre"]);
    }

    /// Le critère du lot : une ancre absente n'est pas contournée, et le bloc à recoller
    /// s'affiche. `layers` est neuve, et tout projet antérieur en est dépourvu.
    #[test]
    fn a_project_without_the_layers_anchor_refuses_and_shows_the_block() {
        let (_parent, root) = project();
        let router = root.join("src/router.rs");
        let ampute: String = fs::read_to_string(&router)
            .expect("router.rs lisible")
            .lines()
            .filter(|line| !line.contains("rbs:layers"))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(&router, ampute).expect("router.rs inscriptible");
        let before = fingerprint(&root);

        let error = run(&options(&root, "cors")).expect_err("l'ancre manque : refuser");

        let remedy = error
            .remedy()
            .unwrap_or_else(|| panic!("aucun bloc à coller pour : {error}"));
        assert!(remedy.contains("// <rbs:layers>"), "{remedy}");
        assert!(remedy.contains("src/router.rs"), "{remedy}");
        assert_eq!(fingerprint(&root), before, "rien ne devait s'écrire");
    }

    #[test]
    fn planning_does_not_modify_the_project_directory() {
        let (_parent, root) = project();
        let before = fingerprint(&root);

        plan_for(&options(&root, "docker")).expect("le plan doit se calculer");

        assert_eq!(fingerprint(&root), before);
    }

    #[test]
    fn rerunning_on_an_already_dockerised_project_gives_a_no_op_plan() {
        let (_parent, root) = project();
        run(&options(&root, "docker")).expect("la première pose doit aboutir");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se recalculer");

        assert!(
            planned.deja_installee,
            "le manifeste inscrit la feature : la relance n'a rien à planifier"
        );
        for file in planned.plan.files() {
            assert_eq!(
                file.statut,
                Status::DejaFait,
                "{} n'est pas sans effet",
                file.path
            );
        }
    }

    #[test]
    fn outside_an_rbs_project_the_command_refuses() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = plan_for(&options(ailleurs.path(), "docker"))
            .expect_err("un répertoire quelconque n'est pas un projet rbs");

        assert!(matches!(error, Error::PasUnProjet), "{error}");
    }

    #[test]
    fn a_dirty_working_tree_refuses_without_force_and_passes_with_it() {
        let (_parent, root) = project();
        commit(&root);
        fs::write(root.join("src/main.rs"), "// modifié").expect("le fichier est écrivable");

        let error = plan_for(&options(&root, "docker"))
            .expect_err("un projet sale ne se modifie pas en silence");
        assert!(matches!(error, Error::WorkingTreeSale(_)), "{error}");

        let mut forcees = options(&root, "docker");
        forcees.force = true;
        plan_for(&forcees).expect("--force doit passer outre");
    }

    #[test]
    fn an_unknown_feature_is_rejected_naming_the_existing_ones() {
        let (_parent, root) = project();

        let error = plan_for(&options(&root, "_aucune_feature_de_ce_nom_"))
            .expect_err("aucun fragment ne porte ce nom");

        assert!(matches!(error, Error::Unknown(_)), "{error}");
        assert!(
            error.to_string().contains("docker"),
            "le message n'oriente pas vers ce qui existe : {error}"
        );
    }

    /// Un fragment de test, posé sur le disque et prêt pour `--template-dir`.
    ///
    /// Le lot n'a pas de fragment à code Rust — `auth` est le lot suivant — et le moule
    /// ne s'éprouve que sur un fragment qui l'exerce.
    fn fragment(manifest: &str, templates: &[(&str, &str)]) -> TempDir {
        let directory = TempDir::new().expect("répertoire temporaire créable");
        let essai = directory.path().join("essai");
        fs::create_dir(&essai).expect("le fragment se crée");
        fs::write(essai.join("feature.toml"), manifest).expect("le manifeste s'écrit");

        for (path, content) in templates {
            let destination = essai.join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("le répertoire se crée");
            }
            fs::write(destination, content).expect("la template s'écrit");
        }

        directory
    }

    /// Les options d'installation du fragment de test posé dans `fragments`.
    fn fragment_options(root: &Path, fragments: &TempDir) -> Options {
        let mut options = options(root, "essai");
        options.template_dir = Some(fragments.path().to_path_buf());
        options
    }

    /// Écrit dans `fragments` un fragment nommé `nom` qui se déclare dans les modules.
    fn fragment_module(fragments: &TempDir, nom: &str) {
        fs::create_dir(fragments.path().join(nom)).expect("le fragment se crée");
        fs::write(
            fragments.path().join(nom).join("feature.toml"),
            format!(
                "[feature]\ndescription = \"{nom}\"\n\n\
                 [[anchors]]\nanchor = \"modules\"\ncontent = \"pub mod {nom};\"\n"
            ),
        )
        .expect("le manifeste s'écrit");
    }

    /// Le squelette ne pose pas `src/modules/mod.rs` : c'est le premier fragment qui s'y
    /// déclare qui l'ouvre, et qui inscrit le module dans la bibliothèque du projet.
    #[test]
    fn the_first_fragment_that_targets_modules_opens_the_mount_point() {
        let (_parent, root) = project();
        let fragments = TempDir::new().expect("répertoire temporaire créable");
        fragment_module(&fragments, "essai");

        run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

        let montage = fs::read_to_string(root.join("src/modules/mod.rs"))
            .expect("le point de montage doit être posé");
        assert!(montage.contains("// <rbs:modules>"), "{montage}");
        assert!(montage.contains("pub mod essai;"), "{montage}");

        let lib = fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs lisible");
        assert!(lib.contains("pub mod modules;"), "{lib}");
        assert!(
            !lib.contains("pub mod essai;"),
            "le fragment se déclare dans le point de montage, pas dans la bibliothèque : {lib}"
        );
    }

    /// Le second fragment trouve le point de montage ouvert : il s'y ajoute sans redéclarer
    /// `pub mod modules;`, qu'un doublon ferait refuser à la compilation.
    #[test]
    fn a_second_fragment_does_not_declare_the_mount_point_twice() {
        let (_parent, root) = project();
        let fragments = TempDir::new().expect("répertoire temporaire créable");
        fragment_module(&fragments, "essai");
        fragment_module(&fragments, "autre");
        run(&fragment_options(&root, &fragments)).expect("la première installation aboutit");

        let mut options = fragment_options(&root, &fragments);
        options.features = vec!["autre".to_string()];
        options.force = true;
        run(&options).expect("la seconde installation doit aboutir");

        let lib = fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs lisible");
        assert_eq!(
            lib.matches("pub mod modules;").count(),
            1,
            "le point de montage ne se déclare qu'une fois : {lib}"
        );
        let montage = fs::read_to_string(root.join("src/modules/mod.rs"))
            .expect("le point de montage existe");
        assert!(montage.contains("pub mod essai;"), "{montage}");
        assert!(montage.contains("pub mod autre;"), "{montage}");
    }

    /// La ligne qui précède immédiatement la balise fermante de `anchor`.
    fn last_line_of(root: &Path, anchor: &crate::anchors::Anchor) -> String {
        let source = fs::read_to_string(root.join(anchor.file.as_ref()))
            .expect("le fichier de l'ancre se lit");
        let closing = anchor.closing();

        source
            .lines()
            .take_while(|line| line.trim() != closing)
            .last()
            .unwrap_or_else(|| panic!("{} ne referme pas {}", anchor.file, anchor.name))
            .trim()
            .to_string()
    }

    /// Le critère de la tâche : ce qu'un fragment déclare arrive dans l'ancre nommée.
    #[test]
    fn the_declared_content_is_inserted_into_each_of_the_four_anchors() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"features\"\ncontent = \"mod essai;\"\n\n\
             [[anchors]]\nanchor = \"routes\"\ncontent = \".merge(crate::essai::routes())\"\n\n\
             [[anchors]]\nanchor = \"openapi\"\ncontent = \"crate::essai::controller::list,\"\n\n\
             [[anchors]]\nanchor = \"migrations\"\ncontent = \"Box::new(m0_essai::Migration),\"\n",
            &[],
        );

        run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

        for (anchor, expected) in [
            // Le projet de `project()` porte une bibliothèque : c'est là que l'ancre
            // résolue par repli atterrit, non plus dans `src/main.rs`.
            (crate::anchors::FEATURES.in_file("src/lib.rs"), "mod essai;"),
            (crate::anchors::ROUTES, ".merge(crate::essai::routes())"),
            (crate::anchors::OPENAPI, "crate::essai::controller::list,"),
            (crate::anchors::MIGRATIONS, "Box::new(m0_essai::Migration),"),
        ] {
            assert_eq!(
                last_line_of(&root, &anchor),
                expected,
                "l'ancre `{}` ne porte pas la ligne déclarée",
                anchor.name
            );
        }
    }

    /// Le critère de la tâche : ancre absente, rien d'écrit, et le bloc sous la main.
    #[test]
    fn a_missing_anchor_writes_nothing_and_prints_the_block() {
        let (_parent, root) = project();
        let router = root.join("src/router.rs");
        let ampute: String = fs::read_to_string(&router)
            .expect("router.rs lisible")
            .lines()
            .filter(|line| !line.contains("// <rbs:routes>"))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(&router, ampute).expect("router.rs inscriptible");

        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[files]]\nsource = \"note.md.jinja\"\ndestination = \"NOTE.md\"\n\n\
             [[anchors]]\nanchor = \"routes\"\ncontent = \".merge(crate::essai::routes())\"\n",
            &[("note.md.jinja", "une note\n")],
        );
        let before = fingerprint(&root);

        let error = run(&fragment_options(&root, &fragments))
            .expect_err("l'ancre manque : l'installation doit refuser");

        let remedy = error
            .remedy()
            .unwrap_or_else(|| panic!("aucun bloc à coller pour : {error}"));
        assert!(remedy.contains("// <rbs:routes>"), "{remedy}");
        assert!(remedy.contains("// </rbs:routes>"), "{remedy}");
        assert!(remedy.contains("src/router.rs"), "{remedy}");

        assert_eq!(
            fingerprint(&root),
            before,
            "l'ancre absente n'a pas empêché l'écriture"
        );
    }

    /// Le fragment de test qui apporte une migration, et son manifeste.
    fn fragment_has_migration() -> TempDir {
        fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [migration]\nsource = \"users.rs.jinja\"\nname = \"create_users\"\n",
            &[("users.rs.jinja", "// la migration de {@ crate_name @}\n")],
        )
    }

    /// Le nom du seul fichier de migration que le fragment a déposé.
    fn written_migration(root: &Path) -> String {
        let deposees: Vec<String> = fs::read_dir(root.join("migration/src"))
            .expect("la crate migration existe")
            .map(|input| {
                input
                    .expect("l'entrée se lit")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.starts_with('m') && name != "main.rs")
            .collect();

        assert_eq!(deposees.len(), 1, "{deposees:?}");
        deposees.into_iter().next().expect("un fichier déposé")
    }

    /// Le critère de la tâche : le fichier porte l'horodatage qu'attend SeaORM.
    #[test]
    fn the_fragment_migration_is_written_in_the_timestamped_format() {
        let (_parent, root) = project();
        let fragments = fragment_has_migration();

        run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

        let written = written_migration(&root);
        let timestamp = written
            .strip_prefix('m')
            .and_then(|reste| reste.strip_suffix("_create_users.rs"))
            .unwrap_or_else(|| panic!("« {written} » n'a pas la forme attendue"));

        assert_eq!(timestamp.len(), 15, "« {written} »");
        assert_eq!(&timestamp[8..9], "_", "« {written} »");
        assert!(
            timestamp
                .chars()
                .enumerate()
                .all(|(rang, c)| rang == 8 || c.is_ascii_digit()),
            "« {written} »"
        );
        assert_eq!(
            fs::read_to_string(root.join("migration/src").join(&written))
                .expect("la migration se lit"),
            "// la migration de demo_api\n"
        );
    }

    /// Le critère de la tâche : une migration déposée est une migration montée.
    #[test]
    fn the_migrations_anchor_is_completed_by_the_matching_call() {
        let (_parent, root) = project();
        let fragments = fragment_has_migration();

        run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

        let module = written_migration(&root).replace(".rs", "");
        assert_eq!(
            last_line_of(&root, &crate::anchors::MIGRATION_MODULES),
            format!("mod {module};")
        );
        assert_eq!(
            last_line_of(&root, &crate::anchors::MIGRATIONS),
            format!("Box::new({module}::Migration),")
        );
    }

    /// Une ancre que le squelette ne porte pas est une faute du manifeste.
    #[test]
    fn an_unknown_anchor_is_rejected_naming_the_existing_ones() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"middlewares\"\ncontent = \"peu importe\"\n",
            &[],
        );

        let error = plan_for(&fragment_options(&root, &fragments))
            .expect_err("`middlewares` n'est pas une ancre du squelette");

        assert!(error.to_string().contains("middlewares"), "{error}");
        assert!(
            error.to_string().contains("routes"),
            "le message n'oriente pas vers les ancres qui existent : {error}"
        );
    }

    /// Un fragment muet ne s'installe pas à vide : il le dit.
    #[test]
    fn a_fragment_without_a_manifest_is_rejected_naming_the_expected_file() {
        let (_parent, root) = project();
        let fragments = TempDir::new().expect("répertoire temporaire créable");
        fs::create_dir(fragments.path().join("muette")).expect("le fragment se crée");
        fs::write(
            fragments.path().join("muette/Note.md.jinja"),
            "rien de déclaré\n",
        )
        .expect("la template s'écrit");

        let mut options = options(&root, "muette");
        options.template_dir = Some(fragments.path().to_path_buf());

        let error = plan_for(&options).expect_err("le fragment ne déclare rien");

        assert!(matches!(error, Error::SansManifeste { .. }), "{error}");
        assert!(
            error.to_string().contains("muette/feature.toml"),
            "le message ne nomme pas le manifeste attendu : {error}"
        );
    }

    #[test]
    fn the_projected_compose_names_the_project_database_and_opens_the_host() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        // Le défaut de `config/default.toml` est 127.0.0.1 : sans cette variable, l'API
        // conteneurisée n'est joignable depuis nulle part.
        assert!(
            compose.contains("RBS_SERVER__HOST: 0.0.0.0"),
            "le compose n'ouvre pas l'hôte :\n{compose}"
        );

        // La base que `migrate` et `api` ouvrent est celle du projet : le compose la
        // nomme par la clé que Compose interpole, et le `.env` porte sa valeur.
        assert!(
            compose.contains("@db:5432/${POSTGRES_DB}"),
            "le compose ne nomme pas la base du projet :\n{compose}"
        );
        let env = projected(&planned, ".env");
        assert_eq!(
            crate::dotenv::value(&crate::dotenv::parse(env), "POSTGRES_DB"),
            Some("demo_api"),
            "{env}"
        );
    }

    /// Un projet déroulé avant que le squelette écrive ce profil ne le recevrait jamais,
    /// et le `RBS_ENV=production` que le compose pose désignerait un fichier absent : la
    /// documentation resterait publiée par le défaut.
    #[test]
    fn the_docker_fragment_writes_the_production_profile_a_project_lacks() {
        let (_parent, root) = project();
        fs::remove_file(root.join("config/production.toml")).expect("le squelette l'écrit");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let profil = projected(&planned, "config/production.toml");

        assert!(
            profil.contains("swagger_ui = false") && profil.contains("openapi_json = false"),
            "le profil déposé ne coupe pas la documentation :\n{profil}"
        );
    }

    /// Le compose est le seul déploiement que rbs livre : l'API qu'il monte n'a aucune
    /// raison de publier `/docs` et le document, que `config/default.toml` expose pour
    /// le développement.
    #[test]
    fn the_projected_compose_runs_the_api_on_the_production_profile() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(
            compose.contains("RBS_ENV: production"),
            "le compose laisse l'API sur le profil de développement :\n{compose}"
        );
    }

    /// Le même repli que `new.rs` sur une URL sans nom de base : sans lui, `POSTGRES_DB`
    /// reste vide, et l'image officielle refuse de s'initialiser sur une base sans nom.
    #[test]
    fn an_empty_database_name_in_the_project_env_falls_back_to_the_crate_name() {
        let (_parent, root) = project();
        avant_les_cles_du_compose(&root);
        let env = fs::read_to_string(root.join(".env")).expect("le .env doit exister");
        let sans_nom_de_base = env.replace(
            "RBS_DATABASE__URL=postgres://rbs:rbs@localhost:5432/demo_api",
            "RBS_DATABASE__URL=postgres://rbs:rbs@localhost:5432",
        );
        assert_ne!(
            env, sans_nom_de_base,
            "la ligne attendue n'a pas été trouvée"
        );
        fs::write(root.join(".env"), sans_nom_de_base).expect("le .env doit se réécrire");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let after = projected(&planned, ".env");

        assert_eq!(
            crate::dotenv::value(&crate::dotenv::parse(after), "POSTGRES_DB"),
            Some("demo_api"),
            "le repli du nom de base n'atteint pas la clé que le compose interpole :\n{after}"
        );
    }

    /// Un `.env` qui existe mais que rbs ne peut pas ouvrir porte peut-être d'autres
    /// identifiants que ceux du moteur : les remplacer en silence poserait un compose
    /// qui ne se connecte à rien.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_env_stops_the_installation_instead_of_inventing_credentials() {
        use std::os::unix::fs::PermissionsExt;

        let (_parent, root) = project();
        let env = root.join(".env");
        fs::set_permissions(&env, fs::Permissions::from_mode(0o000))
            .expect("les droits doivent se poser");

        let error = plan_for(&options(&root, "docker")).expect_err("le .env est illisible");

        assert!(
            error.to_string().contains(".env"),
            "le message ne nomme pas le fichier fautif : {error}"
        );
    }

    /// Un mot de passe dont le `/` n'est pas encodé met fin à l'autorité de l'URL, que
    /// rien ne décompose plus. Les identifiants tombaient alors à vide : le `.env`
    /// recevait un `POSTGRES_USER=` sans valeur, et le service `db` ne montait jamais.
    #[test]
    fn a_url_that_no_longer_decomposes_stops_the_installation() {
        const FAUTIVE: &str = "postgres://rbs:mot/de/passe@localhost:5432/demo_api";

        let (_parent, root) = project();
        avant_les_cles_du_compose(&root);
        viser(&root, FAUTIVE);

        let error = plan_for(&options(&root, "docker")).expect_err("l'URL ne se décompose pas");

        let message = error.to_string();
        assert!(
            message.contains("RBS_DATABASE__URL"),
            "le refus doit nommer la variable : {message}"
        );
        assert!(
            message.contains(FAUTIVE),
            "le refus doit montrer l'URL fautive : {message}"
        );
        assert!(
            message.contains("%2F"),
            "le refus doit donner l'encodage à poser : {message}"
        );
    }

    /// Un projet fraîchement créé qui n'a pas encore de `.env` s'installe : l'absence
    /// n'est pas une faute, et le fragment se pose sur les identifiants par défaut.
    #[test]
    fn a_missing_env_falls_back_to_the_default_credentials() {
        let (_parent, root) = project();
        fs::remove_file(root.join(".env")).expect("le squelette écrit un .env");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);

        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_USER"),
            Some("postgres"),
            "le .env reposé ne porte pas les identifiants par défaut :\n{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_PASSWORD"),
            Some("postgres"),
            "{env}"
        );
        assert_eq!(
            crate::dotenv::value(&paires, "POSTGRES_DB"),
            Some("demo_api"),
            "{env}"
        );
    }

    /// L'ancre disparue arrive ici par l'installation d'un fragment, et non par un plan
    /// direct : le code et le bloc doivent tout de même se lire, sans descendre dans
    /// `installation::Error` à la main.
    #[test]
    fn a_vanished_anchor_carries_its_code_and_block_through_the_installation() {
        let error = Error::Installation(installation::Error::Plan(plan::Error::Anchor(
            crate::anchors::Missing {
                anchor: crate::anchors::ROUTES,
            },
        )));

        assert_eq!(error.code(), "ancre_absente");
        assert_eq!(error.bloc(), Some(crate::anchors::ROUTES.block()));
    }

    #[test]
    fn a_dirty_working_tree_has_a_stable_code() {
        let error = Error::WorkingTreeSale(crate::errors::WorkingTreeSale {
            files: "src/main.rs".to_string(),
        });

        assert_eq!(error.code(), "arbre_sale");
    }

    /// Les trois pannes à bloc, portées directement par un plan ou par l'installation
    /// d'un fragment : un bloc sans son remède, ou l'inverse, laisserait un agent deviner
    /// où coller ce qu'on lui montre.
    #[test]
    fn remede_is_some_exactly_when_bloc_is_some_directly_and_through_the_installation() {
        let constructeurs: Vec<fn() -> plan::Error> = vec![
            || {
                plan::Error::Anchor(crate::anchors::Missing {
                    anchor: crate::anchors::ROUTES,
                })
            },
            || {
                plan::Error::MalPlacee(Box::new(crate::anchors::Misplaced {
                    anchor: crate::anchors::STATE_INIT,
                    before: "core: CoreState::new(".to_string(),
                    block: "// <rbs:state_init>\n// </rbs:state_init>".to_string(),
                }))
            },
            || plan::Error::ZoneAbsente {
                path: "AGENTS.md".to_string(),
                zone: crate::agents::MissingZone {
                    zone: "inventory".to_string(),
                },
            },
        ];

        for construire in constructeurs {
            let direct = Error::Plan(construire());
            assert_eq!(
                direct.remede().is_some(),
                direct.bloc().is_some(),
                "{direct:?}"
            );
            assert!(direct.remede().is_some(), "{direct:?}");

            let via_installation = Error::Installation(installation::Error::Plan(construire()));
            assert_eq!(
                via_installation.remede().is_some(),
                via_installation.bloc().is_some(),
                "{via_installation:?}"
            );
            assert!(via_installation.remede().is_some(), "{via_installation:?}");
        }
    }
}
