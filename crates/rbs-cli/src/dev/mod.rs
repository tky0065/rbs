//! `rbs dev` : démarrage du projet en une commande.
//!
//! La séquence est planifiée avant d'être exécutée, comme `add` et `generate` le font :
//! ce qui va être lancé se lit sur une `Vec<Step>`, sans qu'aucun processus ait démarré.
//! C'est aussi ce qui rend l'orchestration vérifiable sans Docker ni base.

pub(crate) mod watch;

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::database::Database;
use crate::doctor::base;
use crate::{metadata, migrate};

/// Nom du compose que le squelette écrit à la racine du projet.
const COMPOSE: &str = "docker-compose.yml";

/// Délai accordé à une base que rbs vient de remonter.
///
/// Un PostgreSQL sorti de son conteneur met quelques secondes à accepter une connexion :
/// couper trop tôt ferait échouer le cas que `rbs dev` sert justement.
const ATTENTE_APRES_COMPOSE: Duration = Duration::from_secs(30);

/// Délai accordé à une base que rbs n'a pas démarrée.
///
/// Elle est censée déjà tourner : trente secondes de silence avant de dire à quelqu'un
/// qu'il a oublié son PostgreSQL sont trente secondes de perdues.
const ATTENTE: Duration = Duration::from_secs(3);

/// Intervalle entre deux tentatives de connexion.
const INTERVALLE: Duration = Duration::from_millis(250);

/// Une étape du démarrage, dans l'ordre où elle sera exécutée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    /// Remonter les services du compose désigné.
    Compose(PathBuf),
    /// Attendre que la base accepte une connexion.
    Database {
        /// Hôte tiré de l'URL du projet.
        host: String,
        /// Port tiré de l'URL du projet.
        port: u16,
    },
    /// Appliquer les migrations en attente.
    Migrations,
    /// Lancer le serveur, et le relancer à chaque changement.
    Server,
}

/// Ce qui peut empêcher de démarrer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,

    /// Le `.env` du projet est absent, illisible, ou muet sur la base.
    #[error("{0}")]
    Env(#[from] migrate::Error),

    /// L'URL de la base ne dit pas quel hôte joindre.
    #[error("{} n'est pas une URL PostgreSQL exploitable", migrate::URL)]
    UrlIllisible,

    /// Rien n'écoute là où la base est attendue.
    #[error("rien ne répond sur {host}:{port} : la base du projet n'est pas démarrée")]
    Injoignable {
        /// Hôte tiré de l'URL du projet.
        host: String,
        /// Port tiré de l'URL du projet.
        port: u16,
    },

    /// `docker` n'a pas pu être lancé.
    #[error("docker n'a pas pu être lancé : {0}")]
    Docker(#[source] io::Error),

    /// `docker compose up` a échoué.
    #[error("`docker compose up -d` a échoué (code {code})")]
    Compose {
        /// Code de sortie de docker.
        code: i32,
    },

    /// Le répertoire courant n'a pas pu être lu.
    #[error("le répertoire courant est illisible : {0}")]
    Cwd(#[source] io::Error),

    /// Le watch n'a pas pu être installé, ou s'est interrompu.
    #[error("le watch s'est interrompu : {0}")]
    Watch(String),

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Error {
    /// Ce qu'il y a à faire, quand il y a quelque chose à faire.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Self::Injoignable { .. } => Some(format!(
                "démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez \
                 {} dans le .env du projet",
                migrate::URL
            )),
            Self::UrlIllisible => Some(format!(
                "attendu : {}=postgres://utilisateur:motdepasse@hote:port/base",
                migrate::URL
            )),
            // Le compose du squelette est le chemin par défaut depuis cette branche :
            // un projet sans Docker installé n'a plus besoin d'`add docker` pour heurter
            // cette erreur, et doit repartir sans lui — ce que le contrôle de présence du
            // compose, au début de `plan()`, garantit déjà si le fichier disparaît.
            Self::Docker(_) => Some(format!(
                "installez Docker, ou supprimez {COMPOSE} pour démarrer la base vous-même \
                 — `rbs dev` repart alors sans lui"
            )),
            _ => None,
        }
    }
}

/// Démarre le projet qui contient `directory`.
pub(crate) fn run(directory: &Path) -> Result<(), Error> {
    let root = metadata::project_root(directory)?;
    let steps = plan(&root)?;

    crate::ui::info(&render(&steps));

    let attente = patience(&steps);
    start(&root, &steps, attente)
}

/// Ce que rbs accorde à la base, selon qu'il vient ou non de la démarrer.
fn patience(steps: &[Step]) -> Duration {
    if steps.iter().any(|step| matches!(step, Step::Compose(_))) {
        ATTENTE_APRES_COMPOSE
    } else {
        ATTENTE
    }
}

/// Établit la séquence de démarrage à partir de l'état du projet.
pub(crate) fn plan(root: &Path) -> Result<Vec<Step>, Error> {
    let mut steps = Vec::new();

    // Le compose n'est plus la marque d'une feature : le squelette l'écrit pour tout
    // projet dont la base a un serveur à monter. Sa présence est le seul critère.
    let compose = root.join(COMPOSE);
    if compose.is_file() {
        steps.push(Step::Compose(compose));
    }

    // SQLite n'a pas de serveur : son URL ne porte ni hôte ni port, et attendre qu'un
    // port réponde ferait échouer un projet parfaitement démarrable.
    if database_of(root).a_un_serveur() {
        let variables = migrate::project_variables(root)?;
        let url = url(&variables).ok_or(Error::UrlIllisible)?;
        let (host, port) = base::host_and_port(&url).ok_or(Error::UrlIllisible)?;

        steps.push(Step::Database { host, port });
    }

    steps.push(Step::Migrations);
    steps.push(Step::Server);

    Ok(steps)
}

/// Moteur que le manifeste déclare, PostgreSQL à défaut de manifeste lisible.
///
/// Un manifeste illisible n'est pas tranché ici : les étapes suivantes le rencontreront
/// avec un message qui nomme le fichier, là où un échec ici ne dirait que « moteur ».
fn database_of(root: &Path) -> Database {
    metadata::read(&root.join("Cargo.toml"))
        .map(|metadata| metadata.database)
        .unwrap_or_default()
}

/// L'URL visée : celle du `.env`, ou celle que l'appelant a exportée.
fn url(variables: &[(String, String)]) -> Option<String> {
    crate::dotenv::value(variables, migrate::URL)
        .map(str::to_string)
        .or_else(|| std::env::var(migrate::URL).ok())
}

/// Exécute les étapes du plan, dans l'ordre.
fn start(root: &Path, steps: &[Step], attente: Duration) -> Result<(), Error> {
    let variables = migrate::project_variables(root)?;

    for step in steps {
        match step {
            Step::Compose(file) => compose(root, file)?,
            Step::Database { host, port } => {
                wait_for(host, *port, attente, base::reachable, |etape| match etape {
                    Attente::Debut => {
                        crate::ui::waiting(&format!("en attente de la base ({host}:{port})"));
                    }
                    Attente::Seconde => crate::ui::tick(),
                    Attente::Fin => crate::ui::end_of_line(),
                })?;
            }
            Step::Migrations => {
                migrate::launch(root, "up", &variables, false)?;
            }
            Step::Server => watch::run(root, &variables)?,
        }
    }

    Ok(())
}

/// Remonte les services du compose, en arrière-plan.
fn compose(root: &Path, file: &Path) -> Result<(), Error> {
    let status = Command::new("docker")
        .current_dir(root)
        .args(["compose", "-f"])
        .arg(file)
        .args(["up", "-d"])
        .status()
        .map_err(Error::Docker)?;

    if !status.success() {
        return Err(Error::Compose {
            code: status.code().unwrap_or(1),
        });
    }

    Ok(())
}

/// Ce que l'attente d'une base donne à voir.
///
/// L'affichage est laissé à l'appelant : `wait_for` reste vérifiable sans détourner la
/// sortie standard, et le plan reste le seul endroit du module qui sache écrire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attente {
    /// La base n'a pas répondu du premier coup : l'attente commence.
    Debut,
    /// Une seconde de plus s'est écoulée.
    Seconde,
    /// La base a répondu, ou le délai est écoulé.
    Fin,
}

/// Sonde `host:port` jusqu'à ce qu'il réponde, ou que `attente` soit écoulée.
///
/// `montre` reçoit le déroulé de l'attente. Rien ne lui est remis quand la base répond
/// du premier coup : trente secondes de silence n'ont besoin d'être expliquées que
/// lorsqu'elles ont lieu.
fn wait_for(
    host: &str,
    port: u16,
    attente: Duration,
    reachable: impl Fn(&str, u16) -> bool,
    mut montre: impl FnMut(Attente),
) -> Result<(), Error> {
    let depart = Instant::now();
    let echeance = depart + attente;
    let mut annoncee = false;
    let mut secondes = 0;

    loop {
        if reachable(host, port) {
            if annoncee {
                montre(Attente::Fin);
            }

            return Ok(());
        }

        if Instant::now() >= echeance {
            if annoncee {
                montre(Attente::Fin);
            }

            return Err(Error::Injoignable {
                host: host.to_string(),
                port,
            });
        }

        if !annoncee {
            montre(Attente::Debut);
            annoncee = true;
        }

        std::thread::sleep(INTERVALLE.min(attente));

        // Le décompte suit l'horloge et non les tours de boucle : l'intervalle de sonde
        // peut changer sans que le rythme affiché suive.
        let ecoulees = depart.elapsed().as_secs();
        if ecoulees > secondes {
            secondes = ecoulees;
            montre(Attente::Seconde);
        }
    }
}

/// Le plan, tel qu'il s'affiche avant que quoi que ce soit ne démarre.
fn render(steps: &[Step]) -> String {
    let lignes: Vec<String> = steps
        .iter()
        .map(|step| match step {
            Step::Compose(file) => format!(
                "  compose     {}",
                file.file_name()
                    .unwrap_or(file.as_os_str())
                    .to_string_lossy()
            ),
            Step::Database { host, port } => format!("  base        {host}:{port}"),
            Step::Migrations => "  migrations  rbs migrate up".to_string(),
            Step::Server => "  serveur     cargo run, relancé à chaque changement".to_string(),
        })
        .collect();

    lignes.join("\n")
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
    fn project(features: &[&str], url: &str) -> (TempDir, PathBuf) {
        project_on(Database::default(), features, url)
    }

    /// Le même, sur le moteur demandé.
    fn project_on(database: Database, features: &[&str], url: &str) -> (TempDir, PathBuf) {
        crate::fixtures::Project::new()
            .database(database)
            .features(features)
            .url(url)
            .create()
    }

    // SQLite n'a pas de serveur : attendre qu'un port réponde ferait échouer `rbs dev`
    // sur un projet parfaitement démarrable, et l'URL n'a de toute façon ni hôte ni port.
    #[test]
    fn a_sqlite_project_waits_for_no_database_and_starts_anyway() {
        let (_parent, root) = project_on(Database::Sqlite, &[], "sqlite://demo_api.db?mode=rwc");

        let steps = plan(&root).expect("le plan doit se calculer");

        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Step::Database { .. })),
            "le plan attend une base que SQLite n'a pas : {steps:?}"
        );
        assert!(
            steps.iter().any(|step| matches!(step, Step::Migrations)),
            "le plan n'applique plus les migrations : {steps:?}"
        );
        assert!(
            steps.iter().any(|step| matches!(step, Step::Server)),
            "le plan ne démarre plus le serveur : {steps:?}"
        );
    }

    /// Installe la feature `docker` par le vrai chemin de `rbs add`.
    ///
    /// À la main, le nom du compose serait recopié depuis le code testé : c'est le
    /// fragment qui décide comment le fichier s'appelle, et lui seul.
    fn install_docker(root: &Path) {
        let planned = crate::add::plan_for(&crate::add::Options {
            feature: "docker".to_string(),
            directory: root.to_path_buf(),
            force: true,
            template_dir: None,
        })
        .expect("le fragment docker est embarqué");

        crate::plan::application::apply(&planned.plan, true).expect("le fragment s'installe");
    }

    /// Un port que rien ne sert : lié puis relâché, personne ne l'a repris entre-temps.
    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .expect("un port éphémère est libre")
            .local_addr()
            .expect("le socket porte son adresse")
            .port()
    }

    /// Le compose n'est plus conditionné à `[package.metadata.rbs] features` : le
    /// squelette l'écrit, et un projet neuf doit démarrer sans `rbs add docker`.
    #[test]
    fn a_fresh_project_mounts_its_compose_without_the_docker_feature() {
        let (_parent, root) = project(&[], "postgres://rbs:rbs@localhost:5432/demo_api");
        assert!(
            !crate::metadata::read(&root.join("Cargo.toml"))
                .expect("manifeste lisible")
                .features
                .iter()
                .any(|f| f == "docker"),
            "le projet de ce test ne doit pas porter la feature"
        );

        let steps = plan(&root).expect("le plan doit se calculer");

        assert!(matches!(steps.first(), Some(Step::Compose(_))), "{steps:?}");
    }

    #[test]
    fn a_project_without_a_compose_starts_at_the_database() {
        let (_parent, root) = project(&[], "postgres://rbs:rbs@localhost:5432/demo_api");
        std::fs::remove_file(root.join(COMPOSE)).expect("le compose doit exister");

        let steps = plan(&root).expect("le plan doit se calculer");

        assert!(
            !steps.iter().any(|step| matches!(step, Step::Compose(_))),
            "{steps:?}"
        );
    }

    #[test]
    fn with_the_docker_feature_the_compose_is_brought_up_first() {
        let (_parent, root) = project(&[], "postgres://rbs:rbs@localhost:5432/demo_api");
        install_docker(&root);

        let steps = plan(&root).expect("le plan se calcule");

        assert_eq!(
            steps.first(),
            Some(&Step::Compose(root.join(COMPOSE))),
            "le compose n'ouvre pas le démarrage : {steps:?}"
        );
    }

    /// Avant cette branche, seul un projet ayant fait `rbs add docker` heurtait
    /// `Error::Docker` ; le compose du squelette en fait désormais le chemin par défaut,
    /// et `Error::Docker` tombait dans le `_ => None` de `remedy()`, sans aucun remède.
    #[test]
    fn docker_not_being_launchable_names_both_ways_out() {
        let error = Error::Docker(io::Error::from(io::ErrorKind::NotFound));

        let remedy = error
            .remedy()
            .expect("l'absence de docker doit dire quoi faire");

        assert!(
            remedy.to_lowercase().contains("docker"),
            "le remède ne parle pas d'installer Docker : {remedy}"
        );
        assert!(
            remedy.contains(COMPOSE),
            "le remède ne dit pas de retirer le compose pour démarrer sans lui : {remedy}"
        );
    }

    #[test]
    fn an_unreachable_database_is_named_rather_than_panicked_on() {
        let error = wait_for(
            "localhost",
            5432,
            Duration::from_millis(10),
            |_, _| false,
            |_| {},
        )
        .expect_err("rien ne répond");

        let message = error.to_string();
        assert!(
            message.contains("localhost") && message.contains("5432"),
            "le message ne nomme pas ce qui manque : {message}"
        );
        assert!(
            error
                .remedy()
                .is_some_and(|remedy| remedy.contains(migrate::URL)),
            "l'erreur ne dit pas quoi faire : {message}"
        );
    }

    #[test]
    fn a_startup_against_a_dead_port_stops_before_the_migrations() {
        let port = free_port();
        let (_parent, root) = project(&[], &format!("postgres://rbs:rbs@127.0.0.1:{port}/demo"));
        // Le compose du squelette est réel : le laisser dans le plan ferait `start`
        // lancer un vrai `docker compose up -d` avant même d'atteindre le port mort.
        std::fs::remove_file(root.join(COMPOSE)).expect("le compose doit exister");
        let steps = plan(&root).expect("le plan se calcule");

        let error = start(&root, &steps, Duration::from_millis(10)).expect_err("le port est mort");

        assert!(
            matches!(&error, Error::Injoignable { host, port: p } if host == "127.0.0.1" && *p == port),
            "le démarrage n'a pas buté sur la base : {error}"
        );
    }

    #[test]
    fn a_database_rbs_just_started_is_given_more_time_than_one_it_did_not() {
        let sans_compose = vec![Step::Database {
            host: "localhost".to_string(),
            port: 5432,
        }];
        let avec_compose = vec![
            Step::Compose(PathBuf::from(COMPOSE)),
            sans_compose[0].clone(),
        ];

        assert!(
            patience(&avec_compose) > patience(&sans_compose),
            "un conteneur qui démarre n'a pas plus de temps qu'une base censée tourner"
        );
    }

    #[test]
    fn a_reachable_database_lets_the_startup_go_on() {
        wait_for(
            "localhost",
            5432,
            Duration::from_millis(10),
            |_, _| true,
            |_| {},
        )
        .expect("la sonde répond");
    }

    /// Une base qui répond du premier coup n'a fait attendre personne : annoncer une
    /// attente qui n'a pas eu lieu ferait du bruit sur le chemin normal.
    #[test]
    fn a_database_answering_at_once_announces_nothing() {
        let mut vues = Vec::new();

        wait_for(
            "localhost",
            5432,
            Duration::from_millis(10),
            |_, _| true,
            |etape| vues.push(etape),
        )
        .expect("la sonde répond");

        assert!(
            vues.is_empty(),
            "l'attente s'annonce sans attendre : {vues:?}"
        );
    }

    /// Trente secondes de silence ne disent pas si `rbs dev` attend PostgreSQL ou s'il
    /// s'est planté : l'attente s'annonce dès qu'elle commence, puis se compte.
    #[test]
    fn a_wait_announces_itself_then_counts_the_seconds() {
        let mut vues = Vec::new();

        // Un peu plus d'une seconde : le délai réel est le seul moyen d'observer le
        // rythme, `wait_for` n'ayant pas d'horloge à lui substituer.
        wait_for(
            "localhost",
            5432,
            Duration::from_millis(1_050),
            |_, _| false,
            |etape| vues.push(etape),
        )
        .expect_err("rien ne répond");

        assert_eq!(
            vues.first(),
            Some(&Attente::Debut),
            "l'attente ne s'annonce pas : {vues:?}"
        );
        assert!(
            vues.contains(&Attente::Seconde),
            "la seconde écoulée ne se compte pas : {vues:?}"
        );
        assert_eq!(
            vues.last(),
            Some(&Attente::Fin),
            "la ligne d'attente reste ouverte : {vues:?}"
        );
    }

    #[test]
    fn outside_an_rbs_project_nothing_is_started() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(error, Error::PasUnProjet));
    }
}
