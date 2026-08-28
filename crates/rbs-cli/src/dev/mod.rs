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

use crate::doctor::base;
use crate::{metadata, migrate};

/// Nom du fichier que la feature `docker` installe à la racine du projet.
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
    #[error(
        "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici"
    )]
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
}

impl Error {
    /// Ce qu'il y a à faire, quand il y a quelque chose à faire.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Self::Injoignable { .. } => Some(format!(
                "démarrez-la — `docker compose up -d` si la feature docker est installée — \
                 ou corrigez {} dans le .env du projet",
                migrate::URL
            )),
            Self::UrlIllisible => Some(format!(
                "attendu : {}=postgres://utilisateur:motdepasse@hote:port/base",
                migrate::URL
            )),
            _ => None,
        }
    }
}

/// Démarre le projet qui contient `directory`.
pub(crate) fn run(directory: &Path) -> Result<(), Error> {
    let root = metadata::project_root(directory).ok_or(Error::PasUnProjet)?;
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
    plan_with(root, |path| path.is_file())
}

/// Le plan, la présence des fichiers étant décidée par `exists`.
///
/// La sonde est un paramètre pour qu'un test constate non seulement le plan produit, mais
/// aussi qu'aucun compose n'a été *cherché* là où le projet n'en a pas demandé.
fn plan_with(root: &Path, exists: impl Fn(&Path) -> bool) -> Result<Vec<Step>, Error> {
    let mut steps = Vec::new();

    // L'ordre des deux conditions est le critère lui-même : un projet sans la feature ne
    // doit pas voir son disque interrogé, fût-ce pour un fichier absent.
    if declares_docker(root) {
        let compose = root.join(COMPOSE);
        if exists(&compose) {
            steps.push(Step::Compose(compose));
        }
    }

    let variables = migrate::project_variables(root)?;
    let url = url(&variables).ok_or(Error::UrlIllisible)?;
    let (host, port) = base::host_and_port(&url).ok_or(Error::UrlIllisible)?;

    steps.push(Step::Database { host, port });
    steps.push(Step::Migrations);
    steps.push(Step::Server);

    Ok(steps)
}

/// Vrai si `[package.metadata.rbs]` déclare la feature `docker`.
fn declares_docker(root: &Path) -> bool {
    metadata::read(&root.join("Cargo.toml"))
        .is_ok_and(|metadata| metadata.features.iter().any(|feature| feature == "docker"))
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
                wait_for(host, *port, attente, base::reachable)?;
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

/// Sonde `host:port` jusqu'à ce qu'il réponde, ou que `attente` soit écoulée.
fn wait_for(
    host: &str,
    port: u16,
    attente: Duration,
    reachable: impl Fn(&str, u16) -> bool,
) -> Result<(), Error> {
    let echeance = Instant::now() + attente;

    loop {
        if reachable(host, port) {
            return Ok(());
        }

        if Instant::now() >= echeance {
            return Err(Error::Injoignable {
                host: host.to_string(),
                port,
            });
        }

        std::thread::sleep(INTERVALLE.min(attente));
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
    use std::cell::RefCell;
    use std::net::TcpListener;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
    fn project(features: &[&str], url: &str) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: url.to_string(),
                features: features.iter().map(|f| (*f).to_string()).collect(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
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

    #[test]
    fn without_the_docker_feature_no_compose_is_looked_for() {
        let (_parent, root) = project(&[], "postgres://rbs:rbs@localhost:5432/demo_api");
        let cherches = RefCell::new(Vec::new());

        let steps = plan_with(&root, |path| {
            cherches.borrow_mut().push(path.to_path_buf());
            true
        })
        .expect("le plan se calcule");

        assert!(
            cherches.borrow().is_empty(),
            "un compose a été cherché sur un projet qui n'en a pas : {:?}",
            cherches.borrow()
        );
        assert!(
            !steps.iter().any(|step| matches!(step, Step::Compose(_))),
            "un compose est remonté sans la feature : {steps:?}"
        );
        assert!(
            steps.contains(&Step::Server),
            "le serveur ne démarre pas pour autant : {steps:?}"
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

    #[test]
    fn an_unreachable_database_is_named_rather_than_panicked_on() {
        let error = wait_for("localhost", 5432, Duration::from_millis(10), |_, _| false)
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
        wait_for("localhost", 5432, Duration::from_millis(10), |_, _| true)
            .expect("la sonde répond");
    }

    #[test]
    fn outside_an_rbs_project_nothing_is_started() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(error, Error::PasUnProjet));
    }
}
