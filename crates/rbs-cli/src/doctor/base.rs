//! Joignabilité de la base et version du serveur.
//!
//! Deux constats de nature différente. Le premier tient à une connexion TCP : immédiate,
//! elle n'exige rien du projet. Le second demande une requête, donc un client SQL — que
//! rbs n'embarque pas : il le demande au binaire de la crate `migration`, comme
//! `rbs migrate` le fait déjà.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use crate::migrate;

use super::Check;

const TITRE: &str = "base";

/// Port de PostgreSQL quand l'URL n'en donne pas.
const PORT_PAR_DEFAUT: u16 = 5432;

/// Version minimale : `uuidv7()`, que les migrations générées posent en défaut de clé
/// primaire, n'existe qu'à partir de PostgreSQL 18.
const MINIMUM: u32 = 180_000;

/// Délai au-delà duquel l'hôte est tenu pour injoignable.
const DELAI: Duration = Duration::from_secs(3);

/// Vérifie que la base répond et qu'elle est assez récente.
pub(crate) fn check(root: &Path) -> Check {
    let variables = match migrate::project_variables(root) {
        Ok(variables) => variables,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    let url = match url(&variables) {
        Some(url) => url,
        None => {
            return Check::failed(
                TITRE,
                format!(
                    "{} n'est lisible ni dans le .env ni dans l'environnement",
                    migrate::URL
                ),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    let Some((hote, port)) = host_and_port(&url) else {
        return Check::failed(
            TITRE,
            format!("{} n'est pas une URL PostgreSQL", migrate::URL),
            "attendu : postgres://utilisateur:motdepasse@hote:port/base",
        );
    };

    if !reachable(&hote, port) {
        return Check::failed(
            TITRE,
            format!("rien ne répond sur {hote}:{port}"),
            "démarrez PostgreSQL, ou corrigez l'URL du .env",
        );
    }

    match version(root, &variables) {
        Ok(number) if recent_enough(number) => Check::ok(
            TITRE,
            format!("PostgreSQL {} répond sur {hote}:{port}", readable(number)),
        ),
        Ok(number) => Check::failed(
            TITRE,
            format!(
                "PostgreSQL {} sur {hote}:{port}, {} attendu au minimum",
                readable(number),
                readable(MINIMUM)
            ),
            format!(
                "les migrations générées posent uuidv7() en défaut de clé primaire, apparu en PostgreSQL {}",
                readable(MINIMUM)
            ),
        ),
        Err(detail) => Check::failed(
            TITRE,
            format!("{hote}:{port} répond, mais sa version reste inconnue : {detail}"),
            "vérifiez que `cargo run -p migration -- version` aboutit",
        ),
    }
}

/// L'URL visée : celle du `.env`, ou celle que l'appelant a exportée.
///
/// `variables` a déjà été amputé de ce que l'environnement porte : l'y chercher d'abord,
/// puis dans l'environnement, couvre les deux provenances sans en préférer une à tort.
fn url(variables: &[(String, String)]) -> Option<String> {
    crate::dotenv::value(variables, migrate::URL)
        .map(str::to_string)
        .or_else(|| std::env::var(migrate::URL).ok())
}

/// Vrai si une connexion TCP aboutit dans le délai imparti.
pub(crate) fn reachable(hote: &str, port: u16) -> bool {
    let Ok(adresses) = (hote, port).to_socket_addrs() else {
        return false;
    };

    adresses
        .into_iter()
        .any(|adresse| TcpStream::connect_timeout(&adresse, DELAI).is_ok())
}

/// Demande son `server_version_num` au binaire de la crate `migration`.
fn version(root: &Path, variables: &[(String, String)]) -> Result<u32, String> {
    let output = migrate::launch(root, "version", variables, true).map_err(|e| e.to_string())?;

    output
        .split_whitespace()
        .next_back()
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| format!("réponse incomprise : {}", output.trim()))
}

/// Découpe une URL PostgreSQL en hôte et port.
///
/// Le dernier `@` sépare : un mot de passe a le droit d'en contenir un.
pub(crate) fn host_and_port(url: &str) -> Option<(String, u16)> {
    let reste = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))?;

    let apres_identifiants = match reste.rsplit_once('@') {
        Some((_, after)) => after,
        None => reste,
    };

    let autorite = apres_identifiants
        .split(['/', '?'])
        .next()
        .filter(|autorite| !autorite.is_empty())?;

    match autorite.rsplit_once(':') {
        Some((hote, port)) => Some((hote.to_string(), port.parse().ok()?)),
        None => Some((autorite.to_string(), PORT_PAR_DEFAUT)),
    }
}

/// Vrai si le serveur sait poser `uuidv7()`.
fn recent_enough(version: u32) -> bool {
    version >= MINIMUM
}

/// Rend un `server_version_num` lisible : `180001` devient `18.1`.
fn readable(version: u32) -> String {
    format!("{}.{}", version / 10_000, version % 10_000)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet visant `url`, sans passer par le binaire ni par cargo.
    fn project(url: &str) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: url.to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    #[test]
    fn the_host_and_the_port_read_from_the_url() {
        assert_eq!(
            host_and_port("postgres://rbs:rbs@localhost:55433/demo"),
            Some(("localhost".to_string(), 55433))
        );
    }

    #[test]
    fn sans_port_celui_de_postgresql_est_supposé() {
        assert_eq!(
            host_and_port("postgres://rbs:rbs@db.interne/demo"),
            Some(("db.interne".to_string(), PORT_PAR_DEFAUT))
        );
    }

    #[test]
    fn an_at_sign_in_the_password_does_not_shift_the_host() {
        assert_eq!(
            host_and_port("postgres://rbs:p@ss@localhost:5432/demo"),
            Some(("localhost".to_string(), 5432))
        );
    }

    #[test]
    fn a_url_without_credentials_stays_readable() {
        assert_eq!(
            host_and_port("postgres://localhost/demo"),
            Some(("localhost".to_string(), PORT_PAR_DEFAUT))
        );
    }

    #[test]
    fn a_url_that_is_not_postgresql_is_rejected() {
        assert_eq!(host_and_port("mysql://localhost/demo"), None);
    }

    #[test]
    fn the_version_number_renders_as_major_minor() {
        assert_eq!(readable(180_001), "18.1");
        assert_eq!(readable(180_000), "18.0");
        assert_eq!(readable(170_004), "17.4");
    }

    #[test]
    fn postgresql_18_is_the_minimum_because_uuidv7_depends_on_it() {
        assert!(recent_enough(180_000), "18.0 convient");
        assert!(recent_enough(190_002), "une version ultérieure convient");
        assert!(!recent_enough(170_009), "17.9 reste en deçà du minimum");
    }

    #[test]
    fn an_unreachable_database_is_reported_with_its_host_and_port() {
        // Port 1 : réservé, rien n'y écoute — le refus est immédiat et déterministe.
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("127.0.0.1:1"));
        assert!(check.remedy.is_some());
    }

    #[test]
    fn a_url_missing_from_env_is_reported_without_attempting_a_connection() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");
        std::fs::write(root.join(".env"), "RBS_ENV=development\n").expect("écriture du .env");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains(migrate::URL));
    }
}
