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

use std::io;
use std::path::{Path, PathBuf};

use minijinja::context;

use crate::dotenv;
use crate::git;
use crate::manifest;
use crate::metadata;
use crate::migrate;
use crate::plan;
use crate::templates::{self, Source};

/// Ce qu'il faut savoir pour installer une feature.
pub(crate) struct Options {
    /// Nom de la feature, tel que le sous-répertoire de `templates/features` la nomme.
    pub feature: String,
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
    /// Chemins des fichiers de la feature, relatifs à la racine du projet.
    pub files: Vec<String>,
    /// Ce que le fragment annonce installer, tel que son manifeste le décrit.
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
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Seule une ancre disparue a un remède tenant en un bloc de texte : les autres pannes
    /// se règlent par une décision — commiter, corriger le manifeste du fragment.
    pub(crate) fn remedy(&self) -> Option<String> {
        let plan::Error::Anchor(absente) = self.plan()? else {
            return None;
        };

        Some(format!(
            "dans {} :\n{}",
            absente.anchor.file,
            absente.anchor.block()
        ))
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
    if metadonnees
        .features
        .iter()
        .any(|installee| installee == &options.feature)
    {
        return Ok(Planned {
            plan: plan::Builder::new(root).finir(),
            files: Vec::new(),
            description: String::new(),
            entrainees: Vec::new(),
            deja_installee: true,
            zone_manquante: None,
        });
    }

    if !options.force {
        git::garde(&root)?;
    }

    // La feature demandée et celles qu'elle entraîne partagent un seul plan :
    // l'utilisateur voit ce qui s'écrira, y compris ce qu'il n'a pas nommé, avant que
    // quoi que ce soit ne s'écrive.
    let a_poser = resoudre(
        options.template_dir.as_deref(),
        &options.feature,
        &metadonnees.features,
    )?;

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
        // Ce que le projet porte déjà, tel que `[package.metadata.rbs]` l'inscrit : un
        // fragment qui sait qu'un autre est là s'appuie dessus — la limite de débit
        // compte dans Redis quand le cache existe, dans sa mémoire sinon.
        features => metadonnees.features.clone(),
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
    };

    let mut builder = plan::Builder::new(root.clone());
    let timestamp = crate::generate::migration::current_timestamp();
    let mut files = Vec::new();

    for fragment in &a_poser {
        files.extend(installation::actions(
            &installation::Fragment {
                name: &fragment.name,
                manifest: &fragment.manifest,
                templates: &fragment.templates,
                context: context.clone(),
                timestamp: &timestamp,
            },
            &mut builder,
        )?);

        builder.patch(plan::PatchToml::InscrireFeature(fragment.name.clone()))?;
    }

    let posees: Vec<String> = a_poser.iter().map(|f| f.name.clone()).collect();

    // L'inventaire décrit le projet tel que ce plan le laissera : les features viennent
    // d'y être inscrites, et le manifeste du disque les ignore encore.
    let zone_manquante = crate::agents::refresh(&mut builder, &root, &metadonnees, &posees)?;

    let description = a_poser
        .iter()
        .find(|fragment| fragment.name == options.feature)
        .map_or_else(String::new, |fragment| {
            fragment.manifest.feature.description.clone()
        });

    Ok(Planned {
        plan: builder.finir(),
        files,
        description,
        entrainees: posees
            .into_iter()
            .filter(|name| name != &options.feature)
            .collect(),
        deja_installee: false,
        zone_manquante,
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

/// Les fragments à poser pour honorer `feature` : elle, et ceux qu'elle entraîne.
///
/// Ils sont rendus par ordre alphabétique, celui que `rbs new --with` suit déjà : deux
/// chemins d'installation équivalents doivent laisser le même projet, et rien dans un
/// fragment déclaratif ne dépend de l'ordre où ses voisins ont écrit. C'est aussi ce qui
/// garde les `pub mod` du squelette dans l'ordre où rustfmt les veut.
///
/// Un fragment que `[package.metadata.rbs]` inscrit déjà n'est pas reposé : l'entraînement
/// obéit à la même idempotence que l'installation directe.
fn resoudre(
    template_dir: Option<&Path>,
    feature: &str,
    installees: &[String],
) -> Result<Vec<Prevu>, Error> {
    let mut resolution = Resolution {
        template_dir,
        installees,
        poses: Vec::new(),
        en_cours: Vec::new(),
    };
    resolution.resoudre(feature)?;

    let mut poses = resolution.poses;
    poses.sort_by(|gauche, droite| gauche.name.cmp(&droite.name));

    Ok(poses)
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
        let manifest = read_manifest(&source, feature)?;
        let templates = source
            .files()
            .map_err(|source| crate::errors::Acces::new(Path::new(feature), source))?;

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

/// Lit le manifeste du fragment, qui dit ce que son installation fait au projet.
fn read_manifest(source: &Source, feature: &str) -> Result<manifest::Manifest, Error> {
    let text = source
        .manifest()
        .map_err(|source| crate::errors::Acces::new(Path::new(feature), source))?
        .ok_or_else(|| Error::SansManifeste {
            feature: feature.to_string(),
        })?;

    Ok(manifest::read(&text, &format!("{feature}/feature.toml"))?)
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

    fn options(root: &Path, feature: &str) -> Options {
        Options {
            feature: feature.to_string(),
            directory: root.to_path_buf(),
            force: false,
            template_dir: None,
        }
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

    /// L'inventaire est ce que l'agent lit pour savoir ce que le projet porte : une
    /// feature installée qui n'y figure pas le renvoie explorer le disque.
    #[test]
    fn installing_a_feature_names_it_in_the_agents_inventory() {
        let (_parent, root) = project();

        let planned = run(&Options {
            feature: "redis".to_string(),
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

    /// Un fichier de documentation supprimé ne doit pas empêcher d'installer une feature.
    #[test]
    fn a_missing_agents_file_does_not_stop_the_installation() {
        let (_parent, root) = project();
        std::fs::remove_file(root.join("AGENTS.md")).expect("le fichier existe");

        let planned = run(&Options {
            feature: "redis".to_string(),
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
            startup.contains("demo_api::jobs::worker::spawn(state.clone());"),
            "le worker n'est pas détaché au démarrage :\n{main}"
        );

        let configuration = projected(&planned, "config/default.toml");
        for cle in ["max_attempts", "retry_delay_secs", "poll_interval_secs"] {
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
                r#"rbs_core::health::Probe::new("storage", crate::storage::probe(&state.storage)),"#,
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
            ["src/cors/mod.rs", "src/cors/config.rs", "src/cors/tests.rs"]
        );
        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS).contains(".layer(crate::cors::layer())"),
            "{}",
            projected(&planned, "src/router.rs")
        );
        assert!(
            !anchor_body(&planned, &crate::anchors::ROUTES).contains("cors"),
            "la couche s'est montée parmi les routes"
        );

        // Le squelette déclare déjà `tower-http` pour la borne de durée : le fragment
        // ajoute sa feature à celle qui est là plutôt qu'une seconde déclaration.
        let manifeste = projected(&planned, "Cargo.toml");
        assert!(
            manifeste
                .contains("tower-http = { version = \"0.7\", features = [\"timeout\", \"cors\"] }"),
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
                "src/rate_limit/mod.rs",
                "src/rate_limit/config.rs",
                "src/rate_limit/counter.rs",
                "src/rate_limit/tests.rs",
            ]
        );

        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS)
                .contains("crate::rate_limit::middleware"),
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
                "src/observability/mod.rs",
                "src/observability/config.rs",
                "src/observability/metrics.rs",
                "src/observability/tests.rs",
            ]
        );

        let bibliotheque = projected(&planned, "src/lib.rs");
        assert!(
            bibliotheque.contains("pub mod observability;"),
            "{bibliotheque}"
        );
        assert!(
            anchor_body(&planned, &crate::anchors::LAYERS)
                .contains("crate::observability::metrics::middleware"),
            "{}",
            projected(&planned, "src/router.rs")
        );
        assert!(
            anchor_body(&planned, &crate::anchors::STARTUP)
                .contains("demo_api::observability::serve(&state).await?;"),
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
        let metriques = projected(&planned, "src/observability/metrics.rs");

        assert!(metriques.contains("MatchedPath"), "{metriques}");
        assert!(
            !metriques.contains("uri().path()"),
            "le chemin demandé sert d'étiquette :\n{metriques}"
        );

        let tests = projected(&planned, "src/observability/tests.rs");
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
        let counter = projected(&planned, "src/rate_limit/counter.rs");

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
        let counter = projected(&planned, "src/rate_limit/counter.rs");

        assert!(counter.contains("deadpool_redis"), "{counter}");
        assert!(
            counter.contains("crate::cache::Config::load()"),
            "{counter}"
        );
        assert!(!counter.contains("HashMap"), "{counter}");
    }

    /// Le critère de la tâche 12 : `rbs add auth` ne laisse pas `/auth/login` sans limite,
    /// et l'utilisateur le lit avant que quoi que ce soit ne s'écrive.
    #[test]
    fn adding_auth_announces_and_lays_down_the_rate_limit_fragment() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert_eq!(planned.entrainees, ["rate-limit"]);
        assert!(
            planned
                .files
                .iter()
                .any(|file| file == "src/rate_limit/mod.rs"),
            "{:?}",
            planned.files
        );

        let manifeste = projected(&planned, "Cargo.toml");
        assert!(
            manifeste.contains("features = [\"health\", \"auth\", \"rate-limit\"]"),
            "les deux features doivent être inscrites :\n{manifeste}"
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

        let planned = plan_for(&options(&root, "auth")).expect("le plan doit se calculer");

        assert!(planned.entrainees.is_empty(), "{:?}", planned.entrainees);
        assert!(
            !planned
                .files
                .iter()
                .any(|file| file.starts_with("src/rate_limit/")),
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
}
