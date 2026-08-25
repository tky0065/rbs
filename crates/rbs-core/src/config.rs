//! Chargement de la configuration de l'application.
//!
//! Cinq couches fusionnées dans cet ordre, la dernière l'emportant :
//!
//! ```text
//! défauts → config/default.toml → config/{RBS_ENV}.toml → .env → variables d'environnement
//! ```
//!
//! Les deux fichiers TOML sont optionnels. Les variables portent le préfixe `RBS_`, et
//! `__` y sépare les niveaux : `RBS_DATABASE__URL` alimente `database.url`.

use std::path::Path;

use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use figment::value::Value;
use serde::Deserialize;

/// Préfixe des variables d'environnement lues, dans l'environnement comme dans `.env`.
const PREFIXE: &str = "RBS_";

/// Profil retenu quand `RBS_ENV` n'est défini nulle part.
const PROFIL_PAR_DEFAUT: &str = "development";

/// Configuration de l'application, validée au démarrage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Config {
    /// Profil actif, qui désigne le fichier `config/{env}.toml` chargé.
    pub env: String,
    /// Adresse d'écoute du serveur HTTP.
    pub server: ServerConfig,
    /// Accès à la base de données.
    pub database: DatabaseConfig,
}

/// Adresse d'écoute du serveur HTTP.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    /// Interface d'écoute.
    pub host: String,
    /// Port d'écoute.
    pub port: u16,
}

/// Accès à la base de données.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DatabaseConfig {
    /// URL de connexion PostgreSQL. Aucune valeur par défaut : son absence fait échouer
    /// le démarrage.
    pub url: String,
}

/// Échec du chargement de la configuration.
///
/// Distincte d'[`Error`](crate::Error) : une erreur survenue au démarrage ne devient
/// jamais une réponse HTTP.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Valeur manquante, mal formée, ou fichier TOML illisible.
    ///
    /// L'erreur est mise en boîte : `figment::Error` pèse 208 octets, qui alourdiraient
    /// chaque `Result` du module.
    #[error("configuration invalide : {0}")]
    Invalide(Box<figment::Error>),
    /// Fichier `.env` présent mais illisible.
    #[error("fichier `.env` illisible : {0}")]
    Dotenv(#[from] dotenvy::Error),
}

impl From<figment::Error> for ConfigError {
    fn from(erreur: figment::Error) -> Self {
        Self::Invalide(Box::new(erreur))
    }
}

impl Config {
    /// Charge la configuration depuis le répertoire courant.
    ///
    /// # Erreurs
    ///
    /// Échoue si un champ requis manque, si une valeur est mal typée, ou si un fichier
    /// lu est illisible. Le message nomme le champ fautif.
    pub fn load() -> Result<Self, ConfigError> {
        Ok(figment()?.extract()?)
    }
}

/// Assemble les cinq couches.
///
/// Le profil est résolu en deux temps : les couches indépendantes du profil sont
/// fusionnées une première fois pour en extraire `env`, qui désigne alors le fichier
/// `config/{env}.toml` de l'assemblage final.
fn figment() -> Result<Figment, ConfigError> {
    let base = Figment::new()
        .merge(Serialized::default("env", PROFIL_PAR_DEFAUT))
        .merge(Serialized::default("server.host", "127.0.0.1"))
        .merge(Serialized::default("server.port", 8080))
        .merge(Toml::file("config/default.toml"));

    let profil: String = surcharges(base.clone())?
        .extract_inner("env")
        .unwrap_or_else(|_| PROFIL_PAR_DEFAUT.to_owned());

    surcharges(base.merge(Toml::file(format!("config/{profil}.toml"))))
}

/// Empile la couche `.env` puis celle de l'environnement, qui la recouvre.
fn surcharges(figment: Figment) -> Result<Figment, ConfigError> {
    Ok(dotenv(figment, ".env")?.merge(Env::prefixed(PREFIXE).split("__")))
}

/// Lit `.env` **sans toucher à l'environnement du processus**.
///
/// `dotenvy::dotenv()` exporterait ces valeurs dans l'environnement global : la
/// précédence entre les deux couches deviendrait implicite, et les tests fuiteraient
/// les uns dans les autres. Un fichier absent est une couche vide, pas une erreur.
fn dotenv(mut figment: Figment, chemin: impl AsRef<Path>) -> Result<Figment, ConfigError> {
    let entrees = match dotenvy::from_path_iter(chemin.as_ref()) {
        Ok(entrees) => entrees,
        Err(dotenvy::Error::Io(erreur)) if erreur.kind() == std::io::ErrorKind::NotFound => {
            return Ok(figment);
        }
        Err(erreur) => return Err(erreur.into()),
    };

    for entree in entrees {
        let (cle, valeur) = entree?;
        let Some(cle) = cle.strip_prefix(PREFIXE) else {
            continue;
        };
        let cle = cle.to_lowercase().replace("__", ".");
        let valeur: Value = valeur.parse().expect("infaillible");
        figment = figment.merge(Serialized::default(&cle, valeur));
    }

    Ok(figment)
}

#[cfg(test)]
// `Jail::expect_with` impose une fermeture renvoyant `figment::Error`, dont la taille
// déclenche `result_large_err` — signature imposée par figment, pas un choix local.
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;
    use figment::Jail;

    const DEFAULT_TOML: &str = r#"
        [server]
        port = 8080

        [database]
        url = "postgres://localhost/app"
    "#;

    #[test]
    fn un_champ_requis_manquant_fait_echouer_le_chargement_en_nommant_le_champ() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                "[server]\nport = 8080\n\n[database]\n",
            )?;

            let erreur = Config::load().expect_err("`database.url` n'a pas de défaut");

            let message = erreur.to_string();
            assert!(
                message.contains("url"),
                "le message doit nommer le champ fautif, obtenu : {message}"
            );
            Ok(())
        });
    }

    #[test]
    fn une_variable_d_environnement_ecrase_la_valeur_du_fichier_toml() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;
            jail.set_env("RBS_SERVER__PORT", "9999");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.server.port, 9999);
            Ok(())
        });
    }

    #[test]
    fn le_fichier_dotenv_est_lu_mais_cede_devant_l_environnement() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;
            jail.create_file(".env", "RBS_SERVER__PORT=7777\nRBS_SERVER__HOST=0.0.0.0\n")?;
            jail.set_env("RBS_SERVER__PORT", "9999");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.server.host, "0.0.0.0", "valeur lue depuis `.env`");
            assert_eq!(config.server.port, 9999, "l'environnement l'emporte");
            Ok(())
        });
    }

    #[test]
    fn le_fichier_du_profil_ecrase_le_fichier_par_defaut() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;
            jail.create_file("config/production.toml", "[server]\nport = 80\n")?;
            jail.set_env("RBS_ENV", "production");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.env, "production");
            assert_eq!(config.server.port, 80);
            Ok(())
        });
    }

    #[test]
    fn le_profil_se_lit_aussi_depuis_le_fichier_dotenv() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;
            jail.create_file("config/staging.toml", "[server]\nport = 81\n")?;
            jail.create_file(".env", "RBS_ENV=staging\n")?;

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.server.port, 81);
            Ok(())
        });
    }

    #[test]
    fn les_valeurs_par_defaut_s_appliquent_sans_aucun_fichier() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("RBS_DATABASE__URL", "postgres://localhost/app");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.env, "development");
            assert_eq!(config.server.host, "127.0.0.1");
            assert_eq!(config.server.port, 8080);
            Ok(())
        });
    }
}
