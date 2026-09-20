//! `rbs add <feature>` : le fragment d'une feature déposé dans un projet existant.
//!
//! Rien de propre à `docker` ou à `ci` n'est écrit ici. Le catalogue d'une feature est son
//! répertoire sous `templates/features`, et son contexte de rendu se déduit du projet
//! visé : ajouter une feature qui n'apporte pas de code Rust, c'est ajouter un répertoire.
//!
//! La séquence est celle de `generate` — racine, garde Git, plan, application — pour la
//! même raison : ce qui modifie un projet existant se montre avant de s'écrire, et
//! s'écrit en entier ou pas du tout.

pub(crate) mod installation;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

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
    /// Ce que son manifeste dit rester à faire, rendu, et vide s'il ne dit rien.
    ///
    /// Porté par la pose et non par le `Planned` : un fragment entraîné a ses propres
    /// gestes — `auth` arrive avec `mail`, dont le SMTP reste à régler — et les fondre
    /// dans une liste unique perdrait à qui ils appartiennent.
    pub next_steps: Vec<String>,
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

impl From<crate::contexte::Erreur> for Error {
    fn from(faute: crate::contexte::Erreur) -> Self {
        match faute {
            crate::contexte::Erreur::Env(source) => Error::Env(source),
            crate::contexte::Erreur::UrlIndecomposable { url } => Error::UrlIndecomposable { url },
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
    // Le moteur vient du manifeste, seul endroit où le choix de `rbs new` a survécu : un
    // fragment posé six mois plus tard n'a plus les flags de la création.
    let database = metadonnees.database;

    let context = crate::contexte::projet(&root, &nom_projet, database, features)?;

    let mut builder = plan::Builder::new(root.clone());
    let timestamp = crate::generate::migration::current_timestamp();
    #[cfg(test)]
    let mut files = Vec::new();
    let mut poses = Vec::new();

    for fragment in &a_poser {
        // Un seul exemplaire pour les deux lectures du manifeste : le plan et ce qu'il
        // restera à faire décrivent le même fragment, rendu dans le même contexte.
        let vu = installation::Fragment {
            name: &fragment.name,
            manifest: &fragment.manifest,
            templates: &fragment.templates,
            context: context.clone(),
            timestamp: &timestamp,
        };
        let deposes = installation::actions(&vu, &mut builder)?;

        poses.push(Pose {
            name: fragment.name.clone(),
            files: deposes.len(),
            migration: fragment.manifest.migration.is_some(),
            next_steps: installation::next_steps(&vu)?,
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
        planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} absent du plan"))
            .after
            .as_deref()
            .unwrap_or_else(|| panic!("{path} est projeté absent"))
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
                !file
                    .after
                    .as_deref()
                    .unwrap_or_default()
                    .contains("a'b:c$(id)"),
                "{} porte le mot de passe du projet :\n{}",
                file.path,
                file.after.as_deref().unwrap_or_default()
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
                    !file.after.as_deref().unwrap_or_default().contains(cle),
                    "{} porte une clé `{cle}` sur un projet SQLite :\n{}",
                    file.path,
                    file.after.as_deref().unwrap_or_default()
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
                .as_deref()
                .unwrap_or_default()
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();

            assert!(
                !keys.contains("password"),
                "{} porte le secret en clé de configuration :\n{}",
                file.path,
                file.after.as_deref().unwrap_or_default()
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

    /// Le critère de la tâche : un projet fraîchement posé porte un compte capable
    /// d'entrer dans l'espace d'administration.
    ///
    /// `register` ne fixe aucun rôle et la colonne défaut à `"user"` : sans ce seed, la
    /// table des comptes dont l'écran de connexion parle reste vide de tout administrateur.
    #[test]
    fn adding_auth_lays_down_the_seed_of_an_administrator_account() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert!(
            planned.files.iter().any(|f| f == "src/seeds/admin.rs"),
            "le seed du compte d'administration n'est pas déposé : {:?}",
            planned.files
        );

        let binaire = projected(&planned, "src/seeds/main.rs");
        let declares = crate::anchors::body(binaire, crate::anchors::SEEDS)
            .expect("le binaire des seeds porte son ancre");
        assert!(
            declares.contains("admin,"),
            "le seed n'est pas déclaré dans le binaire :\n{binaire}"
        );

        let seed = projected(&planned, "src/seeds/admin.rs");
        assert!(
            seed.contains("Set(model::Role::Admin)"),
            "le compte semé ne porte pas le rôle qui ouvre l'administration :\n{seed}"
        );
        assert!(
            seed.contains("email_verified_at"),
            "l'adresse n'est pas datée : `login_requires_verification` refuserait le \
             compte :\n{seed}"
        );
        // La bibliothèque du projet, et non `crate::` : le binaire des seeds est une
        // racine de crate distincte de celle de l'application.
        assert!(
            seed.contains("use demo_api::auth::model;"),
            "le seed n'atteint pas l'entité par la bibliothèque du projet :\n{seed}"
        );
    }

    /// `rbs seed` refuse la production, mais `cargo run --bin seed` ne passe pas par lui :
    /// le compte que ce seed écrit a tous les droits, et sa garde vit donc dans le seed.
    #[test]
    fn the_administrator_seed_refuses_to_run_in_production() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");
        let seed = projected(&planned, "src/seeds/admin.rs");

        assert!(
            seed.contains("RBS_ENV") && seed.contains("production"),
            "le seed n'a pas de garde de production :\n{seed}"
        );

        // Le binaire des seeds n'en reçoit aucune : une garde posée là changerait le sort
        // des seeds déjà écrits, qui ne créent aucun compte. Il ne reçoit que la
        // déclaration, et rien d'autre.
        let binaire = projected(&planned, "src/seeds/main.rs");
        let avant = fs::read_to_string(root.join("src/seeds/main.rs"))
            .expect("le squelette porte le binaire des seeds");
        assert_eq!(
            binaire.replace("    admin,\n", ""),
            avant,
            "le binaire des seeds a reçu autre chose que la déclaration du seed"
        );
    }

    /// Les identifiants du compte semé atteignent le `.env` sans être publiés.
    ///
    /// L'adresse se déduit du projet plutôt que d'être tirée : un tirage rendrait
    /// soixante-quatre caractères hexadécimaux sans `@`, que l'écran de connexion refuse.
    #[test]
    fn the_administrator_credentials_reach_the_env_without_being_published() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        let env = projected(&planned, ".env");
        let paires = crate::dotenv::parse(env);

        assert_eq!(
            crate::dotenv::value(&paires, "ADMIN_EMAIL"),
            Some("admin@demo-api.test"),
            "{env}"
        );

        let tire = crate::dotenv::value(&paires, "ADMIN_PASSWORD")
            .expect("le .env doit porter le mot de passe du compte");
        assert_eq!(tire.len(), 64, "{env}");

        let exemple = projected(&planned, ".env.example");
        assert!(
            exemple.contains("ADMIN_EMAIL=") && exemple.contains("ADMIN_PASSWORD="),
            "les deux variables ne sont pas documentées :\n{exemple}"
        );
        assert!(
            !exemple.contains(tire),
            "le mot de passe tiré est publié dans l'exemple versionné :\n{exemple}"
        );
    }

    /// Les gestes du fragment nomment les deux variables, faute de quoi le développeur
    /// ignore avec quoi se connecter — et ils le disent dans la langue du projet.
    #[test]
    fn the_auth_fragment_names_the_credentials_of_the_account_it_seeds() {
        for (lang, traduit) in [
            (crate::lang::Lang::Fr, "compte"),
            (crate::lang::Lang::En, "account"),
        ] {
            let (_parent, root) = crate::fixtures::Project::new().lang(lang).create();

            let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");
            let pose = planned
                .poses
                .iter()
                .find(|pose| pose.name == "auth")
                .expect("le fragment est posé");
            let etapes = pose.next_steps.join("\n");

            for geste in [
                "rbs seed",
                "ADMIN_EMAIL",
                "ADMIN_PASSWORD",
                "admin@demo-api.test",
                traduit,
            ] {
                assert!(
                    etapes.contains(geste),
                    "`{geste}` n'est pas dit en {} :\n{etapes}",
                    lang.name()
                );
            }
        }
    }

    /// Le compte semé ne se connecterait pas sous le défaut versionné : le poste de
    /// travail lève la preuve d'adresse, la production la garde.
    #[test]
    fn the_development_profile_alone_lifts_the_verification_the_seeded_account_lacks() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        let defaut = projected(&planned, "config/default.toml");
        assert!(
            defaut.contains("login_requires_verification = true"),
            "le défaut versionné a perdu la preuve d'adresse :\n{defaut}"
        );

        let developpement = projected(&planned, "config/development.toml");
        assert!(
            developpement.contains("login_requires_verification = false"),
            "le profil de développement ne lève pas la preuve d'adresse :\n{developpement}"
        );

        // `cargo test` tourne sous ce profil-là : les tests engendrés poseraient sinon
        // leur 401 d'adresse non vérifiée sur une règle que le fichier vient de lever.
        let harnais = projected(&planned, "src/auth/tests/mod.rs");
        assert!(
            harnais.contains("state.flows.login_requires_verification = true;"),
            "le harnais des tests engendrés lit la règle au lieu de la poser :\n{harnais}"
        );
    }

    /// `app_url` est la racine des liens que les courriels portent : elle doit nommer un
    /// port où quelque chose écoute, et ce port n'est pas le même dans les deux profils.
    #[test]
    fn the_application_url_names_the_port_that_serves_each_profile() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        let defaut = projected(&planned, "config/default.toml");
        assert!(
            defaut.contains("app_url = \"http://localhost:8080\""),
            "le binaire sert le frontend construit sur 8080 :\n{defaut}"
        );

        let developpement = projected(&planned, "config/development.toml");
        assert!(
            developpement.contains("app_url = \"http://localhost:5173\""),
            "Vite sert sur 5173 :\n{developpement}"
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

    /// Le critère de la tâche : un fragment peut ajouter une ligne aux exclusions Git du
    /// projet, et rejouer la pose ne la double pas.
    ///
    /// Sans cette ancre, un fragment qui dépose un répertoire volumineux — les dépendances
    /// d'un frontend — n'avait que deux issues : livrer un `.gitignore` entier, qui entre
    /// en conflit avec celui du squelette, ou laisser le développeur le découvrir à son
    /// premier `git status`.
    #[test]
    fn a_fragment_can_add_a_line_to_the_project_exclusions() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"ignore\"\ncontent = \"/node_modules\"\n",
            &[],
        );

        run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

        let exclusions = fs::read_to_string(root.join(".gitignore")).expect(".gitignore lisible");
        assert!(exclusions.contains("# <rbs:ignore>"), "{exclusions}");
        assert_eq!(
            exclusions.matches("/node_modules").count(),
            1,
            "{exclusions}"
        );

        // Une seconde pose réelle, et non la relance du même fragment — que
        // `[package.metadata.rbs]` arrêterait avant toute insertion : deux fragments
        // peuvent vouloir exclure le même répertoire de build, et la ligne doit rester
        // unique. L'idempotence de l'insertion nue est éprouvée sous ce seam, par
        // `installation::tests::an_exclusion_already_in_place_is_a_no_op_the_second_time`.
        fs::create_dir(fragments.path().join("autre")).expect("le second fragment se crée");
        fs::write(
            fragments.path().join("autre/feature.toml"),
            "[feature]\ndescription = \"autre\"\n\n\
             [[anchors]]\nanchor = \"ignore\"\ncontent = \"/node_modules\"\n",
        )
        .expect("le manifeste s'écrit");

        let mut seconde = fragment_options(&root, &fragments);
        seconde.features = vec!["autre".to_string()];
        seconde.force = true;
        run(&seconde).expect("la seconde installation doit aboutir");

        let apres = fs::read_to_string(root.join(".gitignore")).expect(".gitignore lisible");
        assert_eq!(
            apres.matches("/node_modules").count(),
            1,
            "la ligne est doublée : {apres}"
        );
    }

    /// Le critère de la tâche : l'ancre est optionnelle. Un projet dont le développeur a
    /// supprimé le `.gitignore` reçoit le fragment quand même, et le bloc lui est montré.
    #[test]
    fn a_project_without_exclusions_still_receives_the_fragment() {
        let (_parent, root) = project();
        fs::remove_file(root.join(".gitignore")).expect("le squelette pose un .gitignore");
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"ignore\"\ncontent = \"/node_modules\"\n",
            &[],
        );

        let planned =
            plan_for(&fragment_options(&root, &fragments)).expect("le plan doit se calculer");

        assert!(
            !planned.plan.files().iter().any(|f| f.path == ".gitignore"),
            "le plan ne doit pas inventer de .gitignore"
        );
        let sautees = planned.plan.sautees();
        assert_eq!(sautees.len(), 1, "{sautees:?}");
        assert_eq!(sautees[0].anchor, crate::anchors::IGNORE);

        let rendered = plan::render::plan(&planned.plan);
        assert!(
            rendered.contains(".gitignore absent"),
            "le rendu ne nomme pas le fichier absent :\n{rendered}"
        );
        assert!(
            rendered.contains("/node_modules"),
            "le rendu ne montre pas la ligne à coller :\n{rendered}"
        );
    }

    /// Le critère de la tâche : planifier n'écrit rien, l'ancre des exclusions comprise.
    #[test]
    fn planning_an_exclusion_leaves_the_project_untouched() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\n\
             [[anchors]]\nanchor = \"ignore\"\ncontent = \"/node_modules\"\n",
            &[],
        );
        let before = fingerprint(&root);

        plan_for(&fragment_options(&root, &fragments)).expect("le plan doit se calculer");

        assert_eq!(fingerprint(&root), before, "la planification a écrit");
    }

    /// Le critère de la tâche : les lignes de `next_steps` passent par le moteur de
    /// template comme le reste du manifeste, et arrivent sur la pose du fragment.
    #[test]
    fn a_fragment_can_say_what_is_left_to_do() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\
             next_steps = [\"cd frontend && npm install\", \"{@ project_name @} : npm run build\"]\n",
            &[],
        );

        let planned =
            plan_for(&fragment_options(&root, &fragments)).expect("le plan doit se calculer");

        let pose = planned
            .poses
            .iter()
            .find(|pose| pose.name == "essai")
            .expect("le fragment est posé");
        assert_eq!(
            pose.next_steps,
            [
                // Rendue sans être interprétée : ce qui ressemble à une commande reste du
                // texte, le mécanisme des fragments n'exécutant rien.
                "cd frontend && npm install".to_string(),
                "demo-api : npm run build".to_string(),
            ]
        );
    }

    /// Le critère de la tâche : ces lignes ne paraissent pas dans le plan. Le plan dit ce
    /// qui va s'écrire ; ce qu'il reste à faire ne se lit qu'une fois qu'il s'est écrit.
    #[test]
    fn what_is_left_to_do_is_not_part_of_the_plan() {
        let (_parent, root) = project();
        let fragments = fragment(
            "[feature]\ndescription = \"essai\"\n\
             next_steps = [\"cd frontend && npm install\"]\n",
            &[],
        );
        let before = fingerprint(&root);

        let planned =
            plan_for(&fragment_options(&root, &fragments)).expect("le plan doit se calculer");

        let rendered = plan::render::plan(&planned.plan);
        assert!(
            !rendered.contains("npm install"),
            "le plan annonce une étape qui n'est pas une écriture :\n{rendered}"
        );
        assert_eq!(fingerprint(&root), before, "la planification a écrit");
    }

    /// Un manifeste qui ne déclare rien se comporte exactement comme avant : une liste
    /// vide, et pas une ligne de plus à l'affichage.
    #[test]
    fn a_silent_manifest_leaves_the_pose_without_any_step() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "cors")).expect("le plan doit se calculer");

        let pose = planned
            .poses
            .iter()
            .find(|pose| pose.name == "cors")
            .expect("le fragment est posé");
        assert!(pose.next_steps.is_empty(), "{:?}", pose.next_steps);
    }

    /// Ce que le fragment du frontend dépose, et où : un module sous le point de montage,
    /// un repli sur le routeur, sa section dans la configuration du projet, et le client
    /// que ce repli servira.
    #[test]
    fn the_frontend_fragment_plans_a_module_a_fallback_and_its_configuration() {
        let (_parent, root) = project();
        let before = fingerprint(&root);

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        // L'ossature, nommée en entier ; les composants d'interface, eux, sont comptés
        // par le test voisin. Les lister ici en ferait une copie du manifeste, que tout
        // composant ajouté déplacerait sans rien apprendre de plus.
        let ossature: Vec<&String> = planned
            .files
            .iter()
            .filter(|chemin| !chemin.starts_with("frontend/src/components/ui/"))
            .collect();
        assert_eq!(
            ossature,
            [
                "src/modules/frontend/mod.rs",
                "src/modules/frontend/config.rs",
                "src/modules/frontend/amorcage.rs",
                "src/modules/frontend/feuille.rs",
                "src/modules/frontend/tests.rs",
                "frontend/README.md",
                "frontend/package.json",
                "frontend/tsconfig.json",
                "frontend/vite.config.ts",
                "frontend/index.html",
                "frontend/public/favicon.svg",
                "frontend/src/main.ts",
                "frontend/src/App.vue",
                "frontend/src/router/index.ts",
                "frontend/src/lib/utils.ts",
                "frontend/src/assets/main.css",
                "frontend/src/components/Bande.vue",
                "frontend/src/views/Accueil.vue",
                "frontend/src/views/accueil-textes.ts",
                "frontend/src/views/Galerie.vue",
                "frontend/src/views/galerie-textes.ts",
            ]
            .iter()
            .collect::<Vec<_>>()
        );

        // Les deux moitiés du fragment ne se rencontrent qu'ici : le binaire sert `dir`,
        // et Vite écrit sous son défaut, `dist` à la racine du client. Déplacer l'un des
        // deux — une clé changée, un `outDir` posé dans la configuration de build —
        // rendrait la page d'amorçage éternelle, sans que rien n'échoue.
        let config = projected(&planned, "config/default.toml");
        assert!(config.contains("dir = \"frontend/dist\""), "{config}");
        let vite = projected(&planned, "frontend/vite.config.ts");
        assert!(
            !vite.contains("outDir"),
            "le build sort ailleurs que là où le binaire regarde :\n{vite}"
        );

        // Les deux répertoires que `npm` engendre n'entrent pas au dépôt.
        let exclusions = projected(&planned, ".gitignore");
        assert!(
            exclusions.contains("/frontend/node_modules/"),
            "{exclusions}"
        );
        assert!(exclusions.contains("/frontend/dist/"), "{exclusions}");

        let montage = projected(&planned, "src/modules/mod.rs");
        assert!(montage.contains("pub mod frontend;"), "{montage}");

        let routeur = projected(&planned, "src/router.rs");
        assert!(
            routeur.contains(".merge(crate::modules::frontend::routes())"),
            "{routeur}"
        );

        // Le repli est intérieur à `routes`, et non à `layers` : une couche verrait passer
        // toutes les requêtes de l'API, là où un repli ne voit que ce que personne n'a
        // réclamé.
        let ancre = |balise: &str| {
            routeur
                .find(balise)
                .unwrap_or_else(|| panic!("{balise} absente :\n{routeur}"))
        };
        assert!(
            ancre("// <rbs:routes>") < ancre(".merge(crate::modules::frontend::routes())")
                && ancre(".merge(crate::modules::frontend::routes())") < ancre("// </rbs:routes>"),
            "le repli n'est pas dans l'ancre des routes :\n{routeur}"
        );

        assert!(config.contains("[frontend]"), "{config}");

        // `tower` ne vivait qu'en dépendance de développement, et `tower-http` sans sa
        // feature `fs` : le service de fichiers a besoin des deux à l'exécution.
        let cargo = projected(&planned, "Cargo.toml");
        assert!(cargo.contains("tower ="), "{cargo}");
        assert!(
            cargo.contains("\"fs\""),
            "la feature `fs` de tower-http manque :\n{cargo}"
        );

        assert_eq!(fingerprint(&root), before, "la planification a écrit");
    }

    /// Le fragment livre du code que le CLI ne sait pas construire : il dit donc les
    /// gestes qui restent, par le champ du manifeste et non par la table du CLI.
    ///
    /// Les trois lignes sont l'unique endroit où le geste se dit avant l'exécution du
    /// binaire — la page d'amorçage, elle, ne se lit qu'une fois le serveur lancé.
    #[test]
    fn the_frontend_fragment_says_where_to_read_what_is_left_to_do() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        let pose = planned
            .poses
            .iter()
            .find(|pose| pose.name == "frontend")
            .expect("le fragment est posé");
        let etapes = pose.next_steps.join("\n");
        for geste in ["cd frontend && npm install", "npm run build", "cargo run"] {
            assert!(
                etapes.contains(geste),
                "`{geste}` n'est pas dit :\n{etapes}"
            );
        }
    }

    /// Le seul filet contre un délimiteur de bloc égaré dans un composant, ou une variable
    /// hors contexte : le moteur rend chaque fichier *pendant* la planification, si bien
    /// qu'un plan calculé est déjà un rendu de tout ce que le fragment livre.
    ///
    /// Deux projets, et non un. Le socle porte deux jeux de textes et se branche sur la
    /// langue ; un projet neuf en français n'exerce qu'une moitié des fichiers, et c'est
    /// l'autre qui casserait chez l'utilisateur. Le second projet est aussi celui où tous
    /// les autres fragments sont posés : une branche qui se demande « tel fragment est-il
    /// là ? » y répond oui.
    #[test]
    fn the_frontend_fragment_renders_every_file_it_ships_on_a_bare_and_on_a_full_project() {
        let livres = |root: &std::path::Path| {
            let planned = plan_for(&options(root, "frontend")).expect("le plan doit se calculer");

            // Tout ce que le fragment dépose, le module Rust compris : `projected` rend le
            // contenu que le plan porte, et le plan ne le porte que si le moteur l'a rendu.
            for chemin in &planned.files {
                let rendu = projected(&planned, chemin);
                assert!(!rendu.is_empty(), "{chemin} est rendu vide");
            }

            // Un plan dont le client serait absent passerait la boucle à vide sur la
            // moitié qui nous occupe : ce sont les fichiers rendus qu'on compte.
            let client: Vec<String> = planned
                .files
                .iter()
                .filter(|chemin| chemin.starts_with("frontend/"))
                .cloned()
                .collect();
            assert_eq!(client.len(), 98, "{client:?}");

            client
        };

        let (_nu, nu) = project();
        let (_complet, complet) = crate::fixtures::Project::new()
            .lang(crate::lang::Lang::En)
            .features(&[
                "api-keys",
                "audit",
                "auth",
                "ci",
                "cors",
                "docker",
                "jobs",
                "mail",
                "observability",
                "rate-limit",
                "redis",
                "scheduler",
                "storage",
                "webhooks",
            ])
            .create();

        assert_eq!(
            livres(&nu),
            livres(&complet),
            "le socle ne dépend d'aucun autre fragment : il livre le même arbre des deux côtés"
        );

        // Et les textes, eux, suivent bien la langue du projet : sans ce contrôle, un
        // socle qui ne se brancherait sur rien passerait le test ci-dessus sans avoir
        // jamais exercé sa seconde moitié.
        let anglais = plan_for(&options(&complet, "frontend")).expect("le plan doit se calculer");
        let textes = projected(&anglais, "frontend/src/views/accueil-textes.ts");
        assert!(textes.contains("The API is up"), "{textes}");
        assert!(!textes.contains("L'API tourne"), "{textes}");
    }

    /// Les quatorze composants d'interface, et la route qui les montre.
    ///
    /// Le décompte compte autant que la liste : le composant de formulaire a été écarté
    /// parce qu'il entraîne deux dépendances de validation, et un quinzième répertoire
    /// signalerait qu'il est rentré par la bande.
    #[test]
    fn the_frontend_fragment_ships_fourteen_components_and_a_gallery_that_renders_them() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        for composant in COMPOSANTS {
            let prefixe = format!("frontend/src/components/ui/{composant}/");
            assert!(
                planned
                    .files
                    .iter()
                    .any(|chemin| chemin.starts_with(&prefixe)),
                "le composant `{composant}` n'est pas déposé"
            );
        }

        let repertoires: std::collections::BTreeSet<&str> = planned
            .files
            .iter()
            .filter_map(|chemin| chemin.strip_prefix("frontend/src/components/ui/"))
            .filter_map(|reste| reste.split('/').next())
            .collect();
        assert_eq!(repertoires.len(), COMPOSANTS.len(), "{repertoires:?}");

        // La galerie est le seul endroit du socle où ils se trouvent ensemble : en
        // retirer un la casse, ce qu'aucun autre test ne verrait.
        //
        // Le quatorzième fait exception, et c'est sa nature qui le veut : une pile de
        // notifications se monte une fois pour l'application entière, sinon les écrans
        // qui viendront s'y ajouter n'en auraient aucune. La galerie ne l'importe donc
        // pas — elle le déclenche.
        let galerie = projected(&planned, "frontend/src/views/Galerie.vue");
        for composant in COMPOSANTS
            .iter()
            .filter(|composant| **composant != "sonner")
        {
            assert!(
                galerie.contains(&format!("@/components/ui/{composant}")),
                "la galerie ne rend pas `{composant}`"
            );
        }
        assert!(galerie.contains("toast."), "la galerie ne notifie rien");
        let racine = projected(&planned, "frontend/src/App.vue");
        assert!(
            racine.contains("@/components/ui/sonner"),
            "les notifications ne sont montées nulle part :\n{racine}"
        );

        let routeur = projected(&planned, "frontend/src/router/index.ts");
        assert!(routeur.contains("/galerie"), "{routeur}");
    }

    /// Le thème est un bloc, et rien de ce que le fragment livre ne lui échappe.
    ///
    /// C'est la promesse entière de la tranche : réécrire ce bloc réécrit l'identité. Une
    /// seule couleur en clair dans un composant, et elle devient fausse sans que rien
    /// n'échoue — c'est pourquoi la fouille est mécanique et porte sur tout l'arbre du
    /// client, vues comprises.
    #[test]
    fn nothing_the_fragment_ships_escapes_its_single_theme_block() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        // La version actuelle du générateur n'inline plus son thème : la feuille qu'il
        // engendre importe un fichier de son propre paquet, qui atterrit alors dans les
        // dépendances du projet sans commande pour s'en défaire — ce que vendoriser
        // devait précisément éviter.
        let feuille = projected(&planned, "frontend/src/assets/main.css");
        assert_eq!(feuille.matches("@theme").count(), 1, "{feuille}");
        for importation in feuille.lines().filter(|ligne| ligne.starts_with("@import")) {
            assert!(
                !importation.contains("shadcn"),
                "la feuille tire son thème du paquet du générateur : {importation}"
            );
        }
        let npm = projected(&planned, "frontend/package.json");
        assert!(!npm.contains("shadcn"), "{npm}");

        for chemin in &planned.files {
            // La feuille est le seul fichier admis à écrire une couleur : c'est elle qui
            // les nomme.
            if !chemin.starts_with("frontend/src/") || chemin.ends_with("assets/main.css") {
                continue;
            }
            let rendu = projected(&planned, chemin);

            assert_eq!(
                couleur_en_clair(rendu),
                None,
                "{chemin} écrit une couleur en clair"
            );

            // Les échelles nommées de Tailwind sont des jetons, mais pas ceux de ce
            // projet : `bg-red-500` pour une erreur contourne le thème aussi sûrement
            // qu'un code hexadécimal.
            for echappee in [
                "text-white",
                "bg-white",
                "text-black",
                "bg-black",
                "rounded-full",
                "rounded-[",
                "-slate-",
                "-gray-",
                "-zinc-",
                "-neutral-",
                "-stone-",
                "-red-",
                "-amber-",
                "-green-",
                "-blue-",
            ] {
                assert!(
                    !rendu.contains(echappee),
                    "{chemin} échappe au thème par `{echappee}`"
                );
            }
        }
    }

    /// Aucun emoji nulle part.
    ///
    /// Ce qui sert d'icône vient du jeu vectoriel ; un emoji glissé dans un libellé
    /// traverserait la génération, le compilateur et l'empaqueteur sans rien faire
    /// échouer, et ne se verrait qu'à l'écran.
    #[test]
    fn nothing_the_fragment_ships_carries_an_emoji() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        for chemin in &planned.files {
            if !chemin.starts_with("frontend/") {
                continue;
            }
            let rendu = projected(&planned, chemin);
            assert!(
                !rendu.chars().any(emoji),
                "{chemin} porte un emoji : {:?}",
                rendu.chars().find(|caractere| emoji(*caractere))
            );
        }
    }

    /// L'accueil n'affirme rien qu'il n'ait demandé.
    ///
    /// Le nom du projet vient de la génération, l'état de la sonde et la table des routes
    /// d'un appel au service qui sert la page. Une table de routes écrite en dur mentirait
    /// dès la première entité engendrée ; un lien de documentation écrit en dur mentirait
    /// dès qu'un drapeau de `[docs]` serait coupé.
    #[test]
    fn the_home_page_states_only_what_it_reads_from_the_service() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");
        let accueil = projected(&planned, "frontend/src/views/Accueil.vue");

        assert!(
            accueil.contains("'demo-api'"),
            "le nom du projet n'est pas interpolé :\n{accueil}"
        );

        for route in ["'/health'", "'/api-docs/openapi.json'", "'/docs'"] {
            assert!(
                accueil.contains(route),
                "{route} n'est pas nommée :\n{accueil}"
            );
        }
        // Les trois sont nommées *et* demandées : la sonde directement, les deux routes de
        // documentation par le même interrogateur. Une route citée dans un libellé sans
        // être jamais appelée serait exactement l'affirmation non vérifiée qu'on interdit.
        assert!(
            accueil.contains("await fetch('/health')"),
            "la sonde n'est pas interrogée :\n{accueil}"
        );
        assert_eq!(
            accueil.matches("await interroge(").count(),
            2,
            "les deux routes de documentation ne sont pas interrogées :\n{accueil}"
        );

        // La table sort du document, et non d'une liste recopiée : c'est `paths` qu'elle
        // parcourt.
        assert!(
            accueil.contains(".paths ?? {}"),
            "la table des routes ne vient pas du document :\n{accueil}"
        );

        // Un 200 ne prouve rien : le repli rend l'application pour toute route inconnue.
        // Le document n'est déclaré servi que parce qu'il s'est nommé.
        assert!(
            accueil.contains("typeof publie.openapi !== 'string'"),
            "le document est cru sur son seul code de retour :\n{accueil}"
        );

        // Et un échec ne prouve rien non plus : un service muet ne dit rien du fichier de
        // configuration, et la page n'a pas le droit d'y nommer un drapeau qu'elle n'a pas
        // lu. C'est tout l'objet du quatrième état.
        assert!(
            accueil.contains("type Verdict = 'attente' | 'servi' | 'coupe' | 'injoignable'"),
            "« coupé » et « injoignable » sont confondus :\n{accueil}"
        );

        let textes = projected(&planned, "frontend/src/views/accueil-textes.ts");
        assert!(
            textes.contains("docs_muet:"),
            "aucun libellé ne dit l'ignorance :\n{textes}"
        );
    }

    /// La sonde s'imprime, elle ne se met pas à jour.
    ///
    /// Chaque interrogation ajoute une ligne sous la précédente : la page accumule un
    /// journal, comme le ferait une imprimante ligne. Un état muté sur place effacerait
    /// l'avant-dernière lecture, qui est précisément ce qu'on regarde quand on se demande
    /// si le service vient de tomber.
    #[test]
    fn the_home_page_prints_each_probe_under_the_previous_one() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");
        let accueil = projected(&planned, "frontend/src/views/Accueil.vue");

        assert!(
            accueil.contains("v-for=\"releve in releves\""),
            "le journal n'est pas parcouru :\n{accueil}"
        );
        assert!(
            accueil.contains("[...releves.value, releve]"),
            "la lecture remplace au lieu de s'ajouter :\n{accueil}"
        );
        assert!(
            accueil.contains("setInterval(sonder"),
            "la sonde n'est jamais réinterrogée :\n{accueil}"
        );
    }

    /// La commande d'essai vise l'adresse qui a servi la page, et se copie d'un geste.
    ///
    /// Un port écrit d'avance serait faux la moitié du temps : en développement la page
    /// vient de Vite, qui relaie l'API, et une fois construite du binaire lui-même.
    #[test]
    fn the_home_page_offers_a_curl_that_runs_where_it_is_served() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");
        let accueil = projected(&planned, "frontend/src/views/Accueil.vue");

        assert!(
            accueil.contains("window.location.origin"),
            "la commande ne vise pas l'origine servie :\n{accueil}"
        );
        assert!(
            accueil.contains("curl -s ${ORIGINE}/health"),
            "la commande d'essai n'est pas celle qu'on annonce :\n{accueil}"
        );
        assert!(
            accueil.contains("navigator.clipboard.writeText(COMMANDE)"),
            "la commande ne se copie pas :\n{accueil}"
        );
    }

    /// La police vient du registre, et la pile système reste derrière.
    ///
    /// Le mécanisme des fragments est UTF-8 : un `.woff2` n'en sort pas. La police est
    /// donc une dépendance npm, et tant que l'installation n'a pas eu lieu — ou si le
    /// paquet s'en va — la page se lit dans la chasse de la machine, celle-là même que
    /// la page d'amorçage emploie.
    #[test]
    fn the_font_comes_from_the_registry_and_keeps_a_system_fallback() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        let npm = projected(&planned, "frontend/package.json");
        assert!(
            npm.contains("\"@fontsource/ibm-plex-mono\""),
            "la police n'est pas au manifeste :\n{npm}"
        );

        let feuille = projected(&planned, "frontend/src/assets/main.css");
        for graisse in ["latin-400", "latin-700"] {
            assert!(
                feuille.contains(&format!(
                    "@import '@fontsource/ibm-plex-mono/{graisse}.css';"
                )),
                "`{graisse}` n'est pas importée :\n{feuille}"
            );
        }

        let pile = feuille
            .split("--font-sans:")
            .nth(1)
            .and_then(|reste| reste.split(';').next())
            .expect("le thème déclare une pile de polices");
        assert!(pile.contains("'IBM Plex Mono'"), "{pile}");
        for repli in ["ui-monospace", "monospace"] {
            assert!(
                pile.contains(repli),
                "la pile système ne prend pas le relais : {pile}"
            );
        }

        for chemin in &planned.files {
            for binaire in [".woff", ".woff2", ".ttf", ".otf"] {
                assert!(
                    !chemin.ends_with(binaire),
                    "{chemin} : le fragment ne peut livrer aucun binaire"
                );
            }
        }
    }

    /// L'icône d'onglet est un vecteur que le fragment dépose.
    ///
    /// Un `.ico` est un binaire, et le mécanisme n'en livre aucun : le SVG est du texte,
    /// et c'est la seule forme d'icône qu'un fragment sache poser.
    #[test]
    fn the_tab_icon_is_a_vector_the_fragment_ships() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        assert!(
            planned
                .files
                .iter()
                .any(|chemin| chemin == "frontend/public/favicon.svg"),
            "l'icône n'est pas déposée : {:?}",
            planned.files
        );

        let icone = projected(&planned, "frontend/public/favicon.svg");
        assert!(icone.starts_with("<svg"), "{icone}");
        assert!(icone.contains("viewBox"), "{icone}");

        let index = projected(&planned, "frontend/index.html");
        for morceau in ["rel=\"icon\"", "type=\"image/svg+xml\"", "/favicon.svg"] {
            assert!(
                index.contains(morceau),
                "`{morceau}` manque au document :\n{index}"
            );
        }
    }

    /// Qui a demandé moins de mouvement obtient la ligne posée, non imprimée.
    ///
    /// L'animation est accrochée à `motion-safe`, donc absente sous la préférence, plutôt
    /// que jouée puis neutralisée : rien à casser, puisque rien n'est posé.
    #[test]
    fn the_print_animation_is_dropped_under_reduced_motion() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");

        let accueil = projected(&planned, "frontend/src/views/Accueil.vue");
        assert!(
            accueil.contains("motion-safe:animate-impression"),
            "l'animation n'est pas conditionnée à la préférence :\n{accueil}"
        );

        let feuille = projected(&planned, "frontend/src/assets/main.css");
        assert!(feuille.contains("--animate-impression:"), "{feuille}");
        assert!(feuille.contains("@keyframes impression"), "{feuille}");

        // L'animation n'est jamais *posée* sous la préférence, plutôt que posée puis
        // neutralisée : c'est `motion-safe:` qui le fait, et la page ne doit donc porter
        // aucune classe d'impression inconditionnelle.
        assert!(
            !accueil.contains("\"animate-impression"),
            "l'animation est posée sans condition :\n{accueil}"
        );
    }

    /// L'accueil est une pile de bandes, chacune remplaçable seule.
    ///
    /// C'est ce qui en fait une vitrine et non une page : son propriétaire remplacera la
    /// bande d'essai par la sienne sans toucher aux autres, et ses libellés vivent dans un
    /// fichier à part, rendu dans la langue du projet.
    #[test]
    fn the_home_page_is_a_stack_of_bands_each_replaceable_alone() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "frontend")).expect("le plan doit se calculer");
        let accueil = projected(&planned, "frontend/src/views/Accueil.vue");

        assert!(
            accueil.matches("<Bande").count() >= 6,
            "l'accueil ne se décompose pas en bandes :\n{accueil}"
        );
        assert!(
            accueil.contains(":titre=\"TEXTES.sonde\""),
            "une bande porte son titre ailleurs qu'en prop :\n{accueil}"
        );
        assert!(
            planned
                .files
                .iter()
                .any(|chemin| chemin == "frontend/src/views/accueil-textes.ts"),
            "les libellés ne vivent pas à part : {:?}",
            planned.files
        );
    }

    /// Les quatorze composants d'interface que le socle dépose, par le nom de leur
    /// répertoire.
    const COMPOSANTS: [&str; 14] = [
        "badge",
        "button",
        "card",
        "checkbox",
        "dialog",
        "dropdown-menu",
        "input",
        "label",
        "select",
        "separator",
        "sheet",
        "skeleton",
        "sonner",
        "table",
    ];

    /// La première couleur écrite en clair dans `source`, s'il y en a une.
    ///
    /// Un `#` suivi de trois ou six chiffres hexadécimaux que rien d'alphanumérique ne
    /// prolonge : de quoi distinguer `#f2efe6` d'un `#app` ou d'un lien de documentation.
    fn couleur_en_clair(source: &str) -> Option<String> {
        source.match_indices('#').find_map(|(depart, _)| {
            let suite = &source[depart + 1..];
            [6, 3].into_iter().find_map(|longueur| {
                let chiffres = suite.get(..longueur)?;
                (chiffres
                    .chars()
                    .all(|caractere| caractere.is_ascii_hexdigit())
                    && !suite[longueur..]
                        .starts_with(|caractere: char| caractere.is_alphanumeric()))
                .then(|| format!("#{chiffres}"))
            })
        })
    }

    /// Les blocs Unicode dont le socle n'admet aucun caractère.
    ///
    /// Les pictogrammes et les dingbats, pas les flèches ni la ponctuation : une ellipse
    /// ou un tiret cadratin sont de la typographie, et le socle en emploie.
    fn emoji(caractere: char) -> bool {
        matches!(
            caractere as u32,
            0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F
        )
    }

    /// Ce que le shell d'administration dépose, et rien d'autre.
    ///
    /// Il pose ses fichiers *dans* l'arbre du socle : une seule application, deux régimes
    /// de route. Un fichier du socle redéposé ici ferait un conflit de plan, et non une
    /// installation.
    const SHELL: [&str; 19] = [
        "frontend/src/api/jetons.ts",
        "frontend/src/api/index.ts",
        "frontend/src/stores/authentification.ts",
        "frontend/src/stores/interface.ts",
        "frontend/src/admin/montage.ts",
        "frontend/src/admin/garde.ts",
        "frontend/src/admin/rail.ts",
        "frontend/src/admin/document.ts",
        "frontend/src/admin/textes.ts",
        "frontend/src/admin/lien.ts",
        "frontend/src/admin/Shell.vue",
        "frontend/src/admin/vues/Connexion.vue",
        "frontend/src/admin/vues/Inscription.vue",
        "frontend/src/admin/vues/Reinitialisation.vue",
        "frontend/src/admin/vues/Verification.vue",
        "frontend/src/admin/vues/TableauDeBord.vue",
        "frontend/src/admin/vues/Sessions.vue",
        "frontend/src/admin/vues/Profil.vue",
        "frontend/src/admin/vues/Demonstration.vue",
    ];

    /// Les trois écrans de compte et de santé, et le nom de la route de chacun.
    const ECRANS: [(&str, &str); 3] = [
        ("frontend/src/admin/vues/TableauDeBord.vue", "admin-tableau"),
        ("frontend/src/admin/vues/Sessions.vue", "admin-sessions"),
        ("frontend/src/admin/vues/Profil.vue", "admin-profil"),
    ];

    /// Les pages publiques du shell : le fichier, le nom de la route, et l'appel du client
    /// engendré qui n'avait, avant elles, aucun appelant dans le frontend.
    ///
    /// La connexion n'y est pas : elle est publique comme les trois autres, mais elle
    /// existait déjà, et c'est son propre test qui la regarde.
    const PUBLIQUES: [(&str, &str, &str); 3] = [
        (
            "frontend/src/admin/vues/Inscription.vue",
            "admin-inscription",
            "api.authRegister(",
        ),
        (
            "frontend/src/admin/vues/Reinitialisation.vue",
            "admin-reinitialisation",
            "api.authResetPassword(",
        ),
        (
            "frontend/src/admin/vues/Verification.vue",
            "admin-verification",
            "api.authVerifyEmail(",
        ),
    ];

    /// Le shell exige le socle et l'authentification, et le plan nomme ce qu'il entraîne.
    ///
    /// Un développeur qui demande une interface d'administration reçoit aussi une table
    /// de comptes, une limite de débit et un SMTP à régler : quatre fragments qu'il n'a
    /// pas nommés, et qu'il doit lire avant que le plan ne s'applique, non après.
    #[test]
    fn the_admin_shell_drags_in_the_base_the_authentication_and_what_it_carries() {
        let (_parent, root) = project();
        let before = fingerprint(&root);

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let mut entrainees = planned.entrainees.clone();
        entrainees.sort();
        assert_eq!(
            entrainees,
            ["auth", "frontend", "mail", "rate-limit"],
            "l'entraînement transitif ne paraît pas dans le plan : {:?}",
            planned.entrainees
        );

        // Et posés dans cet ordre : le shell écrit dans l'arbre du socle, et appelle les
        // routes d'`auth`. Le voir passer en premier voudrait dire qu'il se pose sur un
        // répertoire qui n'existe pas encore.
        let ordre = ordre_de_pose(&planned);
        let rang = |nom: &str| {
            ordre
                .iter()
                .position(|pose| *pose == nom)
                .unwrap_or_else(|| panic!("`{nom}` n'est pas posée : {ordre:?}"))
        };
        for amont in ["frontend", "auth"] {
            assert!(
                rang(amont) < rang("frontend-admin"),
                "`{amont}` se pose après le shell : {ordre:?}"
            );
        }

        assert_eq!(fingerprint(&root), before, "la planification a écrit");
    }

    /// Le shell dépose ses quinze fichiers dans l'arbre du socle, et n'en redépose aucun.
    #[test]
    fn the_admin_shell_lands_in_the_tree_the_base_laid_down() {
        let (_parent, root) = crate::fixtures::Project::new()
            .features(&["frontend", "auth"])
            .create();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        assert_eq!(
            planned.files,
            SHELL
                .iter()
                .map(|chemin| (*chemin).to_string())
                .collect::<Vec<_>>(),
            "le shell ne dépose pas exactement ce qu'il annonce"
        );
    }

    /// L'espace d'administration se monte sans ancre, et les deux moitiés se cherchent.
    ///
    /// Un fragment ne peut pas redéposer le routeur du socle, et aucune ancre ne monterait
    /// cet espace-là : le montage se fait donc par découverte de fichier. Rien ne tient
    /// ensemble le motif que le routeur cherche et le chemin où le shell dépose — les voir
    /// diverger ne casse aucune compilation, cela rend seulement l'administration
    /// inatteignable.
    ///
    /// Les deux ancres du shell font l'inverse, et le test voisin les regarde : elles
    /// montent un écran *dans* cet espace, une fois celui-ci découvert.
    #[test]
    fn the_admin_space_mounts_itself_without_an_anchor() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let manifeste = manifeste_du_shell();
        let visees: Vec<&str> = manifeste
            .anchors
            .iter()
            .map(|ancre| ancre.anchor.as_str())
            .collect();
        assert!(
            !visees.contains(&"routes") && !visees.contains(&"modules"),
            "le shell se monte dans le socle par une ancre : {visees:?}"
        );

        // Le motif que le routeur du socle parcourt, et le fichier que le shell y dépose.
        let routeur = projected(&planned, "frontend/src/router/index.ts");
        assert!(
            routeur.contains("import.meta.glob<Montage>('../**/montage.ts', { eager: true })"),
            "le routeur du socle ne cherche aucun montage :\n{routeur}"
        );
        assert!(
            planned
                .files
                .iter()
                .any(|chemin| chemin == "frontend/src/admin/montage.ts"),
            "le shell ne dépose rien que ce motif trouverait : {:?}",
            planned.files
        );

        // La garde voyage par le même contrat : une route protégée atteinte sans session
        // n'a personne pour la détourner si le routeur ne la pose pas.
        assert!(
            routeur.contains("router.beforeEach(montage.garde)"),
            "le routeur du socle ne pose aucune garde :\n{routeur}"
        );
        let montage = projected(&planned, "frontend/src/admin/montage.ts");
        assert!(
            montage.contains("export { garde }"),
            "le shell ne rend aucune garde au routeur :\n{montage}"
        );
        assert!(
            montage.contains("name: 'admin-connexion'"),
            "la route où la garde renvoie n'est pas déclarée :\n{montage}"
        );
        let garde = projected(&planned, "frontend/src/admin/garde.ts");
        assert!(
            garde.contains("await authentification.restaurer()"),
            "la garde tranche sans attendre la session : un écran s'afficherait à moitié \
             chargé avant d'être remplacé\n{garde}"
        );
    }

    /// Le manifeste du shell, lu sur le disque de la crate.
    fn manifeste_du_shell() -> crate::manifest::Manifest {
        crate::manifest::read(
            &std::fs::read_to_string(std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/features/frontend-admin/feature.toml"
            )))
            .expect("le manifeste du shell doit se lire"),
            "frontend-admin/feature.toml",
        )
        .expect("le manifeste du shell doit s'analyser")
    }

    /// L'écran de démonstration se monte par les deux ancres, et par elles seules.
    ///
    /// C'est le geste entier que `rbs generate crud` refera table par table : une route
    /// dans la table de routage de l'espace, une entrée dans le rail, et rien d'autre.
    /// Le voir passer ici est ce qui prouve que les deux ancres sont posées au bon endroit
    /// — une ancre écrite dans un fichier que le fragment ne dépose pas, ou sous une ligne
    /// que le rendu n'écrit pas, laisserait le plan sauter l'insertion en silence.
    #[test]
    fn the_demonstration_screen_mounts_itself_through_the_two_anchors() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let manifeste = manifeste_du_shell();
        let visees: Vec<&str> = manifeste
            .anchors
            .iter()
            .map(|ancre| ancre.anchor.as_str())
            .collect();
        assert_eq!(visees, ["admin_routes", "admin_rail"]);

        // Rien n'a été sauté : une ancre absente de son fichier, ou un fichier absent,
        // ferait consigner l'insertion au lieu de l'écrire — et le plan afficherait le
        // bloc à reporter à la main plutôt que de monter l'écran.
        assert!(
            planned.plan.sautees().is_empty(),
            "une insertion a été sautée : {:?}",
            planned
                .plan
                .sautees()
                .iter()
                .map(|sautee| sautee.anchor.name.as_ref())
                .collect::<Vec<_>>()
        );

        let montage = projected(&planned, "frontend/src/admin/montage.ts");
        let dans = |source: &str, ancre: &str, ligne: &str| {
            let situe = |motif: &str| {
                source
                    .find(motif)
                    .unwrap_or_else(|| panic!("`{motif}` absent :\n{source}"))
            };
            assert!(
                situe(&format!("// <rbs:{ancre}>")) < situe(ligne)
                    && situe(ligne) < situe(&format!("// </rbs:{ancre}>")),
                "`{ligne}` n'est pas dans l'ancre `{ancre}` :\n{source}"
            );
        };

        dans(montage, "admin_routes", "name: 'admin-demonstration',");
        assert!(
            montage.contains("component: () => import('./vues/Demonstration.vue'),"),
            "l'écran ne part pas dans son propre morceau :\n{montage}"
        );

        let rail = projected(&planned, "frontend/src/admin/rail.ts");
        dans(
            rail,
            "admin_rail",
            "{ route: 'admin-demonstration', libelle: 'Démonstration' },",
        );

        // Le rail est servi deux fois, et l'écran ne s'y inscrit qu'une : c'est la
        // coquille qui parcourt la liste, aux deux endroits.
        let coquille = projected(&planned, "frontend/src/admin/Shell.vue");
        assert_eq!(
            coquille.matches("v-for=\"entree in RAIL\"").count(),
            2,
            "le rail n'est pas servi deux fois depuis la même liste :\n{coquille}"
        );
    }

    /// Un projet neuf n'ouvre pas une administration vide et muette.
    ///
    /// L'écran de démonstration est la template dont sortiront les écrans engendrés, et
    /// non un second gabarit : ce que l'on voit à l'installation est ce que la commande
    /// donnera ensuite. Il montre donc pour de bon une table filtrée, triée et paginée.
    #[test]
    fn a_fresh_project_opens_on_a_screen_that_shows_what_generation_will_give() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let ecran = projected(&planned, "frontend/src/admin/vues/Demonstration.vue");
        for geste in [
            "function filtrer(",
            "function trier(",
            "const pages = computed(",
        ] {
            assert!(ecran.contains(geste), "`{geste}` manque :\n{ecran}");
        }

        // Onze lignes et cinq par page : la pagination a de quoi se montrer.
        assert_eq!(ecran.matches("reference: 'DEM-").count(), 11, "{ecran}");
        assert!(ecran.contains("const TAILLE = 5"), "{ecran}");

        // Les lignes vivent dans le fichier : la démonstration n'appelle aucune route que
        // ce projet-ci n'expose pas, et le client engendré ne lui manque pas.
        assert!(
            !ecran.contains("@/api"),
            "l'écran de démonstration appelle une API que le projet n'a pas :\n{ecran}"
        );
    }

    /// Le serveur de développement relaie les routes d'`auth` dès que le shell est là.
    ///
    /// La connexion part vers la même origine que l'application. En développement, celle-ci
    /// est le port de Vite : sans relais, `POST /auth/login` rend l'index du client, avec
    /// un 200 et un corps que le client engendré lit comme une paire de jetons. Rien
    /// n'échoue — l'opérateur entre, et ressort à la première requête.
    ///
    /// La condition porte sur le répertoire du shell, et non sur les features de la pose :
    /// le socle est peut-être posé des mois avant lui, et rendu une seule fois. Le chemin
    /// qu'elle vise et celui où le shell dépose sont donc à tenir ensemble, comme le motif
    /// du routeur et le montage.
    #[test]
    fn the_development_proxy_relays_the_auth_routes_the_shell_calls() {
        let (_parent, root) = project();
        let planned = plan_for(&options(&root, "frontend-admin")).expect("le plan se calcule");

        let vite = projected(&planned, "frontend/vite.config.ts");
        assert!(
            vite.contains("RELAYE.push('/auth')"),
            "les routes d'`auth` ne sont pas relayées :\n{vite}"
        );
        assert!(
            vite.contains("existsSync(fileURLToPath(new URL('./src/admin'"),
            "le relais ne se conditionne pas à la présence du shell :\n{vite}"
        );
        assert!(
            planned
                .files
                .iter()
                .any(|chemin| chemin.starts_with("frontend/src/admin/")),
            "le shell ne dépose rien sous le répertoire que le relais guette : {:?}",
            planned.files
        );
    }

    /// Les appels passent par le client engendré, et aucune couche HTTP n'est réécrite.
    ///
    /// C'est ce qui rend les appels vérifiés à la compilation : le client sort du document
    /// OpenAPI du projet, ses chemins et ses corps sont ceux du contrat. Un `fetch` écrit
    /// à la main les reprendrait en chaînes de caractères, que rien ne relit.
    #[test]
    fn every_call_of_the_shell_goes_through_the_generated_client() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        for chemin in SHELL {
            let rendu = projected(&planned, chemin);
            for reecriture in ["XMLHttpRequest", "axios", "ofetch"] {
                assert!(
                    !rendu.contains(reecriture),
                    "{chemin} parle HTTP par lui-même, avec `{reecriture}`"
                );
            }

            // Une seule exception, et elle ne vise aucune route du contrat : le document
            // OpenAPI n'est pas une opération, et le client qui en sort ne saurait s'y
            // décrire. Elle vit dans un module à elle, pour que cette liste reste close.
            if chemin == "frontend/src/admin/document.ts" {
                continue;
            }
            assert!(
                !rendu.contains("fetch("),
                "{chemin} parle HTTP par lui-même, avec `fetch(`"
            );
        }

        let client = projected(&planned, "frontend/src/api/index.ts");
        assert!(
            client.contains("from './client'") && client.contains("new ApiClient("),
            "le shell ne consomme pas le client engendré :\n{client}"
        );

        // Le client n'est pas livré : il sort du contrat de ce projet-ci, et un client
        // figé mentirait dès la première route ajoutée. La commande qui l'engendre est
        // donc dite, et avant celle qui construirait sans lui.
        assert!(
            !planned
                .files
                .iter()
                .any(|chemin| chemin == "frontend/src/api/client.ts"),
            "le shell livre un client figé"
        );
        let etapes = planned
            .poses
            .iter()
            .find(|pose| pose.name == "frontend-admin")
            .expect("le shell est posé")
            .next_steps
            .clone();
        assert_eq!(
            etapes.first().map(String::as_str),
            Some("rbs generate client --lang ts --out frontend/src/api"),
            "la génération du client n'ouvre pas les gestes qui restent : {etapes:?}"
        );
    }

    /// La réinitialisation du mot de passe s'atteint depuis la connexion.
    ///
    /// C'est le seul écran qu'un opérateur enfermé dehors peut encore ouvrir : la demande
    /// vit donc sur celui-là, et non derrière la garde qu'il ne franchit plus. Elle part
    /// sur la route réelle du fragment `auth`, qui répond la même chose que l'adresse soit
    /// inscrite ou non — l'écran ne doit donc pas prétendre savoir laquelle il a touchée.
    #[test]
    fn the_password_reset_is_reachable_from_the_sign_in_screen() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let connexion = projected(&planned, "frontend/src/admin/vues/Connexion.vue");
        assert!(
            connexion.contains("api.authForgotPassword("),
            "la connexion n'offre aucune demande de réinitialisation :\n{connexion}"
        );
        assert!(
            connexion.contains("TEXTES.oubli"),
            "rien n'y renvoie depuis l'écran :\n{connexion}"
        );

        // La demande reste un dialogue de l'écran de connexion : une page pour un champ
        // n'apporte rien, et c'est le seul écran qu'un opérateur enfermé dehors ouvre
        // encore. Aucune route ne lui est donc ouverte, quand les trois autres parcours
        // publics en ont une.
        assert!(
            connexion.contains("<Dialog v-model:open=\"oubliOuvert\">"),
            "la demande n'est plus un dialogue de l'écran :\n{connexion}"
        );
        let montage = projected(&planned, "frontend/src/admin/montage.ts");
        assert!(
            !montage.contains("Oubli"),
            "la demande de réinitialisation s'est donné un écran à elle :\n{montage}"
        );
    }

    /// Les trois pages publiques se montent hors du shell, hors du rail, et devant la
    /// garde.
    ///
    /// Hors du shell parce qu'il n'a ni rail ni compte à montrer à qui n'est pas entré ;
    /// hors du rail parce que le rail est la navigation d'un espace authentifié, et que la
    /// connexion n'y figure pas davantage ; devant la garde parce que ces parcours sont
    /// précisément ce par quoi on obtient le jeton qu'elle réclame.
    #[test]
    fn the_public_screens_mount_outside_the_shell_the_rail_and_the_guard() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let montage = projected(&planned, "frontend/src/admin/montage.ts");
        let rail = projected(&planned, "frontend/src/admin/rail.ts");
        let garde = projected(&planned, "frontend/src/admin/garde.ts");

        // Ce que le shell couvre, et rien d'autre : les quatre publiques et lui-même.
        assert_eq!(
            montage.matches("path: '/admin").count(),
            5,
            "l'espace ne déclare pas les quatre pages publiques et le shell :\n{montage}"
        );

        let enfants = montage
            .split("children: [")
            .nth(1)
            .expect("le shell porte des enfants");

        for (chemin, route, appel) in PUBLIQUES {
            let fichier = chemin
                .rsplit('/')
                .next()
                .expect("le chemin porte un nom de fichier");

            assert!(
                planned.files.iter().any(|pose| pose == chemin),
                "{chemin} n'est pas déposé : {:?}",
                planned.files
            );
            assert!(
                montage.contains(&format!("name: '{route}'")),
                "la route `{route}` n'est pas déclarée :\n{montage}"
            );
            assert!(
                montage.contains(&format!("() => import('./vues/{fichier}')")),
                "`{fichier}` n'est pas chargé paresseusement :\n{montage}"
            );
            assert!(
                !enfants.contains(route),
                "`{route}` est montée sous le shell, derrière la garde :\n{montage}"
            );
            assert!(
                !rail.contains(route),
                "`{route}` est entrée dans le rail d'un espace authentifié :\n{rail}"
            );
            assert!(
                garde.contains(&format!("'{route}'")),
                "la garde ne laisse pas passer `{route}` :\n{garde}"
            );
            assert!(
                projected(&planned, chemin).contains(appel),
                "{chemin} n'appelle pas `{appel}`"
            );
        }

        // Et la connexion reste publique : la retirer de l'ensemble enfermerait dehors
        // tout le monde, y compris ceux qui ont déjà un compte.
        assert!(
            garde.contains("'admin-connexion'"),
            "la garde ne laisse plus passer la connexion :\n{garde}"
        );
    }

    /// Les liens que le fragment `auth` met dans ses courriels tombent sur un écran.
    ///
    /// `auth` les compose depuis `app_url`, sans rien savoir du shell posé à côté : c'est
    /// donc au shell de servir ces chemins-là. Un alias les sert sans redirection — le
    /// jeton vit dans le fragment de l'URL, qu'une redirection perdrait.
    #[test]
    fn the_paths_the_auth_emails_carry_land_on_a_screen() {
        let (_parent, root) = project();

        let shell = plan_for(&options(&root, "frontend-admin")).expect("le plan se calcule");
        let montage = projected(&shell, "frontend/src/admin/montage.ts");

        for chemin in ["/forgot-password", "/reset-password", "/verify-email"] {
            assert!(
                montage.contains(&format!("alias: '{chemin}'")),
                "`{chemin}` ne tombe sur aucun écran :\n{montage}"
            );
        }

        // Et ce sont bien les chemins qu'`auth` compose : le fragment les écrit dans ses
        // services, et les deux moitiés n'ont rien d'autre qui les tienne ensemble.
        let (_autre, projet) = project();
        let auth = plan_for(&options(&projet, "auth")).expect("le plan doit se calculer");
        for (fichier, chemin) in [
            ("src/auth/service/mod.rs", "\"forgot-password\""),
            ("src/auth/service/password.rs", "\"reset-password\""),
            ("src/auth/service/verification.rs", "\"verify-email\""),
        ] {
            let rendu = projected(&auth, fichier);
            assert!(
                rendu.contains(chemin),
                "{fichier} ne compose plus {chemin} :\n{rendu}"
            );
        }

        // Le jeton voyage dans le fragment, et c'est le fragment que les écrans lisent en
        // premier : les deux écrans passent par le même module pour ne pas diverger.
        let lien = projected(&shell, "frontend/src/admin/lien.ts");
        assert!(
            lien.contains("route.hash"),
            "le jeton n'est pas cherché dans le fragment de l'URL :\n{lien}"
        );
        for chemin in [
            "frontend/src/admin/vues/Reinitialisation.vue",
            "frontend/src/admin/vues/Verification.vue",
        ] {
            let rendu = projected(&shell, chemin);
            assert!(
                rendu.contains("jetonDuLien(route)"),
                "{chemin} relit le lien pour son compte :\n{rendu}"
            );
        }
    }

    /// `registration_enabled` ferme la route, et pas seulement l'écran.
    ///
    /// Un interrupteur qui n'aurait masqué qu'un bouton aurait laissé `POST /auth/register`
    /// ouvert à tout ce qui n'est pas un navigateur. Le refus part donc du service, avant
    /// même le hachage, et porte un code que l'écran sait lire.
    #[test]
    fn the_registration_switch_closes_the_route_and_not_only_the_screen() {
        let (_parent, root) = project();

        let auth = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        let config = projected(&auth, "config/default.toml");
        assert!(
            config.contains("registration_enabled = true"),
            "le réglage n'arrive pas avec le fragment :\n{config}"
        );

        let service = projected(&auth, "src/auth/service/session.rs");
        let ferme = service
            .find("if !flows.registration_enabled {")
            .expect("le service ne referme pas l'inscription");
        assert!(
            ferme
                < service
                    .find("hash::hash_password(&input.password)")
                    .expect("l'inscription hache le mot de passe"),
            "le refus paie un Argon2 qu'un service fermé n'a aucune raison de payer :\n{service}"
        );
        assert!(
            service.contains("code: \"registration_closed\","),
            "le refus ne se distingue pas d'un autre 403 :\n{service}"
        );

        // Et l'écran ne devine pas : il demande, et se range sur ce que le service dit.
        let (_second, projet) = project();
        let shell = plan_for(&options(&projet, "frontend-admin")).expect("le plan se calcule");
        let inscription = projected(&shell, "frontend/src/admin/vues/Inscription.vue");
        assert!(
            inscription.contains("api.authRegistrationStatus()"),
            "l'écran d'inscription devine l'état de l'interrupteur :\n{inscription}"
        );
        assert!(
            inscription.contains("'registration_closed'"),
            "l'écran ne lit pas le refus que la route rend :\n{inscription}"
        );
    }

    /// La gestion de profil devient une écriture : l'adresse s'y change.
    ///
    /// Sans `PATCH /auth/me` et l'écran qui l'appelle, « gestion de profil » ne désignait
    /// qu'une page en lecture seule.
    #[test]
    fn the_profile_screen_can_write_the_address_it_shows() {
        let (_parent, root) = project();

        let shell = plan_for(&options(&root, "frontend-admin")).expect("le plan se calcule");
        let profil = projected(&shell, "frontend/src/admin/vues/Profil.vue");

        assert!(
            profil.contains("api.authUpdateMe({ email:"),
            "le profil ne sait rien écrire :\n{profil}"
        );

        let (_second, projet) = project();
        let auth = plan_for(&options(&projet, "auth")).expect("le plan doit se calculer");
        let module = projected(&auth, "src/auth/mod.rs");
        assert!(
            module.contains("get(controller::me).patch(controller::account::update_me)"),
            "les deux méthodes de `/auth/me` ne se déclarent pas en une fois :\n{module}"
        );

        // L'adresse et rien d'autre : le DTO est celui des routes qui ne prennent qu'elle,
        // et un champ de plus y serait aussitôt visible ailleurs.
        let controleur = projected(&auth, "src/auth/controller/account.rs");
        assert!(
            controleur.contains("ValidatedJson<EmailRequest>"),
            "l'écriture du profil accepte autre chose que l'adresse :\n{controleur}"
        );
    }

    /// Les trois écrans se montent dans le routage et dans le rail, par le même nom.
    ///
    /// Rien ne tient ensemble ces deux moitiés : une entrée de rail qui nommerait une
    /// route absente rendrait un lien mort, que ni la vérification des types ni la
    /// construction ne verraient. Chacun part en outre dans son propre morceau — le
    /// visiteur de l'accueil ne télécharge pas l'administration.
    #[test]
    fn the_three_account_screens_mount_in_the_routing_and_in_the_rail() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let montage = projected(&planned, "frontend/src/admin/montage.ts");
        let rail = projected(&planned, "frontend/src/admin/rail.ts");

        for (chemin, route) in ECRANS {
            let fichier = chemin
                .rsplit('/')
                .next()
                .expect("le chemin porte un nom de fichier");

            assert!(
                planned.files.iter().any(|pose| pose == chemin),
                "{chemin} n'est pas déposé : {:?}",
                planned.files
            );
            assert!(
                montage.contains(&format!("name: '{route}'")),
                "la route `{route}` n'est pas déclarée :\n{montage}"
            );
            assert!(
                montage.contains(&format!("() => import('./vues/{fichier}')")),
                "`{fichier}` n'est pas chargé paresseusement :\n{montage}"
            );
            assert!(
                rail.contains(&format!("route: '{route}'")),
                "`{route}` n'est pas dans le rail :\n{rail}"
            );
        }

        // Sous le shell, et non à côté : un écran de compte monté en dehors n'aurait ni
        // rail, ni garde, et s'afficherait nu à qui n'est pas connecté. Les pages
        // publiques font l'inverse, et c'est leur propre test qui les regarde.
        let enfants_du_shell = montage
            .split("children: [")
            .nth(1)
            .expect("le shell porte des enfants");
        for (_, route) in ECRANS {
            assert!(
                enfants_du_shell.contains(route),
                "`{route}` s'est montée hors de l'espace :\n{montage}"
            );
        }

        // Le commentaire que la commande de génération remplacera reste en dernier :
        // ce que le fragment monte et ce qu'elle montera ne se disputent pas la ligne.
        let enfants = montage
            .split("children: [")
            .nth(1)
            .expect("le shell porte des enfants");
        let derniere = enfants
            .lines()
            .map(str::trim)
            .filter(|ligne| !ligne.is_empty())
            .take_while(|ligne| *ligne != "],")
            .last()
            .expect("les enfants ne sont pas vides");
        assert!(
            derniere.starts_with("// "),
            "le point d'insertion des écrans engendrés n'est plus en dernier : {derniere}"
        );
    }

    /// Le rail est servi deux fois et n'est écrit qu'une.
    ///
    /// Le shell le rend à demeure et dans un panneau latéral. Les entrées portées à la
    /// main dans les deux `<nav>` divergeaient à la première retouche ; un écran ajouté
    /// dans un seul des deux restait invisible sur téléphone, sans que rien n'échoue.
    #[test]
    fn the_shell_reads_its_rail_from_a_single_list() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let shell = projected(&planned, "frontend/src/admin/Shell.vue");
        assert_eq!(
            shell.matches("v-for=\"entree in RAIL\"").count(),
            2,
            "les deux rails ne parcourent pas la même liste :\n{shell}"
        );

        // Un lien de navigation, et non un gestionnaire de clic : c'est ce qui en fait
        // une ancre atteignable au clavier, et ce qui la rend ouvrable dans un onglet.
        assert_eq!(
            shell.matches("<RouterLink").count(),
            2,
            "le rail ne rend pas des liens :\n{shell}"
        );

        // Et les libellés ne sont écrits qu'à un endroit : le rail les tient des textes.
        let rail = projected(&planned, "frontend/src/admin/rail.ts");
        for (_, route) in ECRANS {
            assert!(
                !shell.contains(route),
                "`{route}` est recopiée dans le shell :\n{shell}"
            );
        }
        assert_eq!(
            rail.matches("libelle: TEXTES.").count(),
            ECRANS.len(),
            "une entrée du rail porte son libellé en dur :\n{rail}"
        );
    }

    /// Aucun écran ne s'affiche vide, ne confond « rien » et « cassé », ni ne rend un code.
    ///
    /// Les trois états se voient à l'écran et nulle part ailleurs : ce qui est éprouvable
    /// ici est qu'ils sont branchés. `phrase()` lit le `detail` RFC 9457 que le noyau rend
    /// — c'est lui qui distingue une phrase d'un `HTTP 422`.
    #[test]
    fn every_account_screen_says_what_it_is_doing_and_what_it_has_found() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        for (chemin, _) in ECRANS {
            let rendu = projected(&planned, chemin);

            assert!(
                rendu.contains("TEXTES.chargement"),
                "{chemin} n'annonce pas son chargement :\n{rendu}"
            );
            assert!(
                rendu.contains("phrase("),
                "{chemin} montre ses pannes sans les lire :\n{rendu}"
            );
            // Un code de statut recopié dans l'écran est exactement ce que `phrase()`
            // existe pour éviter.
            assert!(
                !rendu.contains("HTTP "),
                "{chemin} affiche un code brut :\n{rendu}"
            );
        }

        // Les deux écrans qui portent une liste la disent vide plutôt que muette, et le
        // disent autrement qu'ils ne diraient une panne.
        for (chemin, cle) in [
            ("frontend/src/admin/vues/TableauDeBord.vue", "aucune_sonde"),
            ("frontend/src/admin/vues/Sessions.vue", "aucune_session"),
        ] {
            let rendu = projected(&planned, chemin);
            assert!(
                rendu.contains(&format!("TEXTES.{cle}")),
                "{chemin} laisse une liste vide sans rien dire :\n{rendu}"
            );

            let textes = projected(&planned, "frontend/src/admin/textes.ts");
            assert!(
                textes.contains(&format!("{cle}:")),
                "`{cle}` n'a pas de libellé :\n{textes}"
            );
        }
    }

    /// Le tableau de bord ne montre que ce que le service publie vraiment.
    ///
    /// Les sondes viennent de `GET /health`, qui est une route du contrat. La version et
    /// le nombre de routes ne sont exposés par aucune opération : ils sont des propriétés
    /// du document OpenAPI, que le client engendré ne peut pas porter puisqu'il en sort.
    /// C'est la seule requête du shell qui ne passe pas par lui, et elle est bornée à ce
    /// module-là.
    #[test]
    fn the_dashboard_reads_the_probes_from_the_contract_and_the_rest_from_the_document() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let tableau = projected(&planned, "frontend/src/admin/vues/TableauDeBord.vue");
        assert!(
            tableau.contains("api.health()"),
            "le tableau de bord n'interroge pas la sonde :\n{tableau}"
        );
        assert!(
            tableau.contains("lireLeContrat()"),
            "le tableau de bord n'ouvre pas le document :\n{tableau}"
        );

        let document = projected(&planned, "frontend/src/admin/document.ts");
        assert!(
            document.contains("const DOCUMENT = '/api-docs/openapi.json'")
                && document.contains("fetch(DOCUMENT"),
            "le document n'est pas lu là où il est publié :\n{document}"
        );

        // Ce module est la seule exception à « tout passe par le client engendré », et
        // elle ne tient qu'à ce que le document ne soit pas une opération du contrat. Un
        // second appel ici viserait forcément une route, et l'exception deviendrait une
        // porte : le compte est donc tenu, et non la seule présence.
        assert_eq!(
            document.matches("fetch(").count(),
            1,
            "l'exception au client engendré porte plus d'un appel :\n{document}"
        );

        // Un 503 porte le même corps qu'un 200, et c'est le cas où l'opérateur a le plus
        // besoin de le lire : le rendre illisible reviendrait à n'afficher les sondes que
        // lorsqu'elles vont toutes bien.
        assert!(
            tableau.contains("ApiError"),
            "un service dégradé n'affiche aucune sonde :\n{tableau}"
        );
    }

    /// Les sessions se révoquent une par une et toutes d'un coup, par les routes réelles.
    ///
    /// La révocation globale emporte la session de l'appelant — le service le dit. La
    /// laisser ouverte ici rendrait un écran qui marche encore et cessera de marcher à la
    /// première requête, sans que rien ne l'ait annoncé.
    #[test]
    fn the_sessions_screen_revokes_one_and_all_and_sees_itself_out() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let sessions = projected(&planned, "frontend/src/admin/vues/Sessions.vue");
        for appel in [
            "api.authListSessions()",
            "api.authRevokeSession(",
            "api.authRevokeSessions()",
        ] {
            assert!(
                sessions.contains(appel),
                "`{appel}` manque à l'écran des sessions :\n{sessions}"
            );
        }
        assert!(
            sessions.contains("authentification.deconnexion()"),
            "la révocation globale laisse la session de l'appelant ouverte :\n{sessions}"
        );
    }

    /// Le profil change le mot de passe, et la paire neuve remplace celle qui vient de
    /// tomber.
    ///
    /// Le service ferme toutes les sessions du compte, celle de l'appelant comprise, puis
    /// réémet. Jeter cette paire déconnecterait au renouvellement suivant quelqu'un qui
    /// vient de faire exactement la bonne chose ; c'est le store qui la retient, parce que
    /// c'est lui qui tient la session.
    #[test]
    fn changing_the_password_keeps_the_session_the_service_just_reissued() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let profil = projected(&planned, "frontend/src/admin/vues/Profil.vue");
        assert!(
            profil.contains("api.authMe()"),
            "le profil n'interroge pas le compte courant :\n{profil}"
        );
        assert!(
            profil.contains("authentification.changerMotDePasse("),
            "le changement de mot de passe ne passe pas par le store :\n{profil}"
        );

        let store = projected(&planned, "frontend/src/stores/authentification.ts");
        let changement = store
            .split("async function changerMotDePasse(")
            .nth(1)
            .unwrap_or_else(|| panic!("le store ne sait pas changer de mot de passe :\n{store}"));
        assert!(
            changement.contains("api.authChangePassword("),
            "le store ne change rien par le contrat :\n{changement}"
        );
        assert!(
            changement.contains("retenir("),
            "la paire neuve est jetée :\n{changement}"
        );
    }

    /// Deux stores, et deux seulement.
    ///
    /// Un store par entité est le réflexe dont Pinia s'est précisément affranchi : il
    /// doublerait le code à recopier pour chaque écran engendré. Ce que le shell garde est
    /// ce qui ne se rattache à aucun écran — la session, et l'état de l'interface.
    #[test]
    fn the_shell_keeps_two_stores_and_no_more() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let stores: Vec<&String> = planned
            .files
            .iter()
            .filter(|chemin| chemin.starts_with("frontend/src/stores/"))
            .collect();
        assert_eq!(
            stores,
            [
                "frontend/src/stores/authentification.ts",
                "frontend/src/stores/interface.ts"
            ]
            .iter()
            .collect::<Vec<_>>()
        );

        // Le décompte compte autant que la liste : un `defineStore` glissé dans un écran
        // passerait le contrôle ci-dessus sans rien déplacer.
        let definitions: usize = planned
            .files
            .iter()
            .map(|chemin| projected(&planned, chemin).matches("defineStore(").count())
            .sum();
        assert_eq!(definitions, 2, "un troisième store est apparu");
    }

    /// Le jeton d'accès ne touche jamais le stockage, et le compromis est écrit sur place.
    ///
    /// Le stockage local survit à l'onglet, et c'est tout l'intérêt pour le jeton long ;
    /// c'est aussi ce qui le rend lisible par un script injecté. Y laisser passer le jeton
    /// d'accès ne changerait rien de visible et rendrait le partage gratuit.
    #[test]
    fn only_the_refresh_token_reaches_the_local_storage() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        let jetons = projected(&planned, "frontend/src/api/jetons.ts");
        assert!(
            jetons.contains("const CLE = 'demo-api.rafraichissement'"),
            "la clé du stockage n'est pas celle du jeton long :\n{jetons}"
        );
        for ligne in jetons
            .lines()
            .filter(|ligne| ligne.contains("localStorage"))
        {
            assert!(
                ligne.contains("(CLE"),
                "le stockage est touché ailleurs que sur la clé du jeton long : {ligne}"
            );
        }

        // Et nulle part ailleurs : le thème est le seul autre état qui survit à l'onglet.
        for chemin in SHELL {
            if chemin.ends_with("api/jetons.ts") || chemin.ends_with("stores/interface.ts") {
                continue;
            }
            assert!(
                !projected(&planned, chemin).contains("localStorage"),
                "{chemin} écrit dans le stockage du navigateur"
            );
        }

        // Le compromis se lit là où il se prend, et non seulement dans la documentation :
        // c'est ce fichier que relira celui qui voudra le reprendre.
        for terme in ["stockage local", "HttpOnly"] {
            assert!(
                jetons.contains(terme),
                "le compromis du transport n'est pas commenté sur place : `{terme}` absent"
            );
        }
    }

    /// Le seul filet contre un délimiteur de bloc égaré ou une variable hors contexte :
    /// planifier, c'est rendre.
    ///
    /// Deux projets, et non un. Le shell porte deux jeux de textes et se branche sur la
    /// langue du projet ; un projet neuf en français n'en exercerait qu'une moitié, et
    /// c'est l'autre qui casserait chez l'utilisateur.
    #[test]
    fn the_admin_shell_renders_every_file_it_ships_in_both_languages() {
        let (_nu, nu) = project();
        let francais = plan_for(&options(&nu, "frontend-admin")).expect("le plan se calcule");

        for chemin in SHELL {
            assert!(
                !projected(&francais, chemin).is_empty(),
                "{chemin} est rendu vide"
            );
        }

        let textes = projected(&francais, "frontend/src/admin/textes.ts");
        assert!(
            textes.contains("Adresse ou mot de passe refusé."),
            "{textes}"
        );
        assert!(!textes.contains("Email or password refused."), "{textes}");

        let (_complet, complet) = crate::fixtures::Project::new()
            .lang(crate::lang::Lang::En)
            .features(&["auth", "frontend"])
            .create();
        let anglais = plan_for(&options(&complet, "frontend-admin")).expect("le plan se calcule");

        assert_eq!(
            anglais.files,
            SHELL
                .iter()
                .map(|chemin| (*chemin).to_string())
                .collect::<Vec<_>>(),
            "le shell ne livre pas le même arbre des deux côtés"
        );
        let traduits = projected(&anglais, "frontend/src/admin/textes.ts");
        assert!(
            traduits.contains("Email or password refused."),
            "{traduits}"
        );
        assert!(
            !traduits.contains("Adresse ou mot de passe refusé."),
            "{traduits}"
        );
    }

    /// Le shell ne nomme aucune couleur, et ne porte aucun emoji.
    ///
    /// Le thème est un bloc : réécrire ce bloc réécrit l'identité, et une seule couleur en
    /// clair dans un écran la rend fausse sans que rien n'échoue. Un emoji, lui, traverse
    /// la génération, le compilateur et l'empaqueteur, et ne se voit qu'à l'écran.
    #[test]
    fn nothing_the_admin_shell_ships_escapes_the_theme_nor_carries_an_emoji() {
        let (_parent, root) = project();

        let planned =
            plan_for(&options(&root, "frontend-admin")).expect("le plan doit se calculer");

        for chemin in SHELL {
            let rendu = projected(&planned, chemin);

            assert_eq!(
                couleur_en_clair(rendu),
                None,
                "{chemin} écrit une couleur en clair"
            );
            for echappee in [
                "text-white",
                "bg-white",
                "text-black",
                "bg-black",
                "rounded-full",
                "rounded-[",
                "-slate-",
                "-gray-",
                "-zinc-",
                "-neutral-",
                "-stone-",
                "-red-",
                "-amber-",
                "-green-",
                "-blue-",
            ] {
                assert!(
                    !rendu.contains(echappee),
                    "{chemin} échappe au thème par `{echappee}`"
                );
            }
            assert!(
                !rendu.chars().any(emoji),
                "{chemin} porte un emoji : {:?}",
                rendu.chars().find(|caractere| emoji(*caractere))
            );
        }
    }
}
