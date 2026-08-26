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

use super::Controle;

const TITRE: &str = "base";

/// Port de PostgreSQL quand l'URL n'en donne pas.
const PORT_PAR_DEFAUT: u16 = 5432;

/// Version minimale : `uuidv7()`, que les migrations générées posent en défaut de clé
/// primaire, n'existe qu'à partir de PostgreSQL 18.
const MINIMUM: u32 = 180_000;

/// Délai au-delà duquel l'hôte est tenu pour injoignable.
const DELAI: Duration = Duration::from_secs(3);

/// Vérifie que la base répond et qu'elle est assez récente.
pub(crate) fn controler(racine: &Path) -> Controle {
    let variables = match migrate::variables_du_projet(racine) {
        Ok(variables) => variables,
        Err(erreur) => {
            return Controle::echec(
                TITRE,
                erreur.to_string(),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    let url = match url(&variables) {
        Some(url) => url,
        None => {
            return Controle::echec(
                TITRE,
                format!(
                    "{} n'est lisible ni dans le .env ni dans l'environnement",
                    migrate::URL
                ),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    let Some((hote, port)) = hote_et_port(&url) else {
        return Controle::echec(
            TITRE,
            format!("{} n'est pas une URL PostgreSQL", migrate::URL),
            "attendu : postgres://utilisateur:motdepasse@hote:port/base",
        );
    };

    if !joignable(&hote, port) {
        return Controle::echec(
            TITRE,
            format!("rien ne répond sur {hote}:{port}"),
            "démarrez PostgreSQL, ou corrigez l'URL du .env",
        );
    }

    match version(racine, &variables) {
        Ok(numero) if assez_recente(numero) => Controle::bon(
            TITRE,
            format!("PostgreSQL {} répond sur {hote}:{port}", lisible(numero)),
        ),
        Ok(numero) => Controle::echec(
            TITRE,
            format!(
                "PostgreSQL {} sur {hote}:{port}, {} attendu au minimum",
                lisible(numero),
                lisible(MINIMUM)
            ),
            format!(
                "les migrations générées posent uuidv7() en défaut de clé primaire, apparu en PostgreSQL {}",
                lisible(MINIMUM)
            ),
        ),
        Err(detail) => Controle::echec(
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
    crate::dotenv::valeur(variables, migrate::URL)
        .map(str::to_string)
        .or_else(|| std::env::var(migrate::URL).ok())
}

/// Vrai si une connexion TCP aboutit dans le délai imparti.
fn joignable(hote: &str, port: u16) -> bool {
    let Ok(adresses) = (hote, port).to_socket_addrs() else {
        return false;
    };

    adresses
        .into_iter()
        .any(|adresse| TcpStream::connect_timeout(&adresse, DELAI).is_ok())
}

/// Demande son `server_version_num` au binaire de la crate `migration`.
fn version(racine: &Path, variables: &[(String, String)]) -> Result<u32, String> {
    let sortie = migrate::lancer(racine, "version", variables, true).map_err(|e| e.to_string())?;

    sortie
        .split_whitespace()
        .next_back()
        .and_then(|numero| numero.parse().ok())
        .ok_or_else(|| format!("réponse incomprise : {}", sortie.trim()))
}

/// Découpe une URL PostgreSQL en hôte et port.
///
/// Le dernier `@` sépare : un mot de passe a le droit d'en contenir un.
fn hote_et_port(url: &str) -> Option<(String, u16)> {
    let reste = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))?;

    let apres_identifiants = match reste.rsplit_once('@') {
        Some((_, apres)) => apres,
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
fn assez_recente(version: u32) -> bool {
    version >= MINIMUM
}

/// Rend un `server_version_num` lisible : `180001` devient `18.1`.
fn lisible(version: u32) -> String {
    format!("{}.{}", version / 10_000, version % 10_000)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::Etat;
    use super::*;

    /// Un projet visant `url`, sans passer par le binaire ni par cargo.
    fn projet(url: &str) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let projet = crate::new::creer(
            &crate::new::Options {
                nom: "demo-api".to_string(),
                database_url: url.to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, projet.racine)
    }

    #[test]
    fn l_hote_et_le_port_se_lisent_dans_l_url() {
        assert_eq!(
            hote_et_port("postgres://rbs:rbs@localhost:55433/demo"),
            Some(("localhost".to_string(), 55433))
        );
    }

    #[test]
    fn sans_port_celui_de_postgresql_est_supposé() {
        assert_eq!(
            hote_et_port("postgres://rbs:rbs@db.interne/demo"),
            Some(("db.interne".to_string(), PORT_PAR_DEFAUT))
        );
    }

    #[test]
    fn un_arobase_dans_le_mot_de_passe_ne_deplace_pas_l_hote() {
        assert_eq!(
            hote_et_port("postgres://rbs:p@ss@localhost:5432/demo"),
            Some(("localhost".to_string(), 5432))
        );
    }

    #[test]
    fn une_url_sans_identifiants_reste_lisible() {
        assert_eq!(
            hote_et_port("postgres://localhost/demo"),
            Some(("localhost".to_string(), PORT_PAR_DEFAUT))
        );
    }

    #[test]
    fn une_url_qui_n_est_pas_du_postgresql_est_refusee() {
        assert_eq!(hote_et_port("mysql://localhost/demo"), None);
    }

    #[test]
    fn le_numero_de_version_se_rend_en_majeur_mineur() {
        assert_eq!(lisible(180_001), "18.1");
        assert_eq!(lisible(180_000), "18.0");
        assert_eq!(lisible(170_004), "17.4");
    }

    #[test]
    fn postgresql_18_est_le_minimum_car_uuidv7_en_depend() {
        assert!(assez_recente(180_000), "18.0 convient");
        assert!(assez_recente(190_002), "une version ultérieure convient");
        assert!(!assez_recente(170_009), "17.9 reste en deçà du minimum");
    }

    #[test]
    fn une_base_injoignable_est_signalee_avec_son_hote_et_son_port() {
        // Port 1 : réservé, rien n'y écoute — le refus est immédiat et déterministe.
        let (_parent, racine) = projet("postgres://rbs:rbs@127.0.0.1:1/demo");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("127.0.0.1:1"));
        assert!(controle.remede.is_some());
    }

    #[test]
    fn une_url_absente_du_env_est_signalee_sans_tentative_de_connexion() {
        let (_parent, racine) = projet("postgres://rbs:rbs@127.0.0.1:1/demo");
        std::fs::write(racine.join(".env"), "RBS_ENV=development\n").expect("écriture du .env");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains(migrate::URL));
    }
}
