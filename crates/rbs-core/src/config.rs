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
    /// Exposition de la documentation OpenAPI.
    pub docs: DocsConfig,
    /// Signature des jetons et durées de vie des sessions.
    #[cfg(feature = "auth")]
    pub auth: AuthConfig,
}

/// Exposition de la documentation OpenAPI.
///
/// Les deux réglages sont séparés parce que les deux besoins ne sont pas symétriques :
/// couper l'interface tout en gardant le document sert à générer des clients ou à
/// vérifier un contrat d'API, alors que l'inverse n'a pas d'usage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DocsConfig {
    /// Montage de Swagger UI sur `/docs`.
    pub swagger_ui: bool,
    /// Exposition du document sur `/api-docs/openapi.json`.
    pub openapi_json: bool,
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
///
/// Seule `url` est requise. Les réglages du pool portent des défauts tenables en
/// production, qu'un projet sous charge ajuste sans forker le runtime.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DatabaseConfig {
    /// URL de connexion PostgreSQL. Aucune valeur par défaut : son absence fait échouer
    /// le démarrage.
    pub url: String,
    /// Nombre maximum de connexions ouvertes simultanément.
    pub max_connections: u32,
    /// Nombre de connexions maintenues ouvertes au repos.
    pub min_connections: u32,
    /// Délai d'établissement d'une connexion, en secondes.
    pub connect_timeout_secs: u64,
    /// Délai d'obtention d'une connexion du pool, en secondes.
    pub acquire_timeout_secs: u64,
    /// Durée d'inactivité au terme de laquelle une connexion est fermée, en secondes.
    pub idle_timeout_secs: u64,
    /// Durée de vie maximale d'une connexion, en secondes.
    pub max_lifetime_secs: u64,
}

/// Secret de signature et durées de vie des jetons.
///
/// Les deux durées suivent la dissymétrie habituelle : l'accès est court parce qu'il
/// n'est pas révocable, le rafraîchissement long parce qu'il l'est, ligne par ligne.
#[cfg(feature = "auth")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthConfig {
    /// Secret de signature HS256. Aucune valeur par défaut : son absence fait échouer
    /// le démarrage.
    pub secret: String,
    /// Durée de vie du jeton d'accès, en secondes.
    pub access_ttl_secs: u64,
    /// Durée de vie du jeton de rafraîchissement, en secondes.
    pub refresh_ttl_secs: u64,
}

/// Longueur minimale du secret de signature, en octets.
///
/// HS256 travaille sur une clé de 256 bits : un secret plus court n'ajoute rien à ce
/// que la force brute doit parcourir.
#[cfg(feature = "auth")]
const SECRET_MINIMUM: usize = 32;

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
    /// Section demandée absente de toutes les couches de la cascade.
    #[error("configuration invalide : la section `{0}` est absente")]
    SectionAbsente(String),
    /// Secret de signature trop court pour HS256.
    #[cfg(feature = "auth")]
    #[error("configuration invalide : `auth.secret` doit porter au moins 32 octets, {0} fournis")]
    SecretTropCourt(usize),
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
        let config: Self = figment()?.extract()?;

        // Un secret court n'est pas rattrapable au runtime : il se refuse au boot, où le
        // développeur le lit, et non à la première requête protégée.
        #[cfg(feature = "auth")]
        if config.auth.secret.len() < SECRET_MINIMUM {
            return Err(ConfigError::SecretTropCourt(config.auth.secret.len()));
        }

        Ok(config)
    }
}

/// Charge une section que le noyau ne connaît pas, par la cascade de [`Config::load`].
///
/// Le noyau n'oppose aucun défaut à `T` : les valeurs par défaut sont celles que la
/// struct appelante porte par `#[serde(default)]`, là où son auteur les lit et les change.
///
/// # Erreurs
///
/// Échoue si la section est absente de toutes les couches, si un champ requis manque, ou
/// si une valeur est mal typée.
pub fn section<T: serde::de::DeserializeOwned>(nom: &str) -> Result<T, ConfigError> {
    let figment = figment()?;

    // Une section absente se distingue d'une section mal remplie : le message qui nomme la
    // table manquante mène à `config/default.toml`, celui de figment à un champ isolé.
    if !figment.contains(nom) {
        return Err(ConfigError::SectionAbsente(nom.to_owned()));
    }

    Ok(figment.extract_inner(nom)?)
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
        .merge(Serialized::default("database.max_connections", 10))
        .merge(Serialized::default("database.min_connections", 0))
        .merge(Serialized::default("database.connect_timeout_secs", 5))
        .merge(Serialized::default("database.acquire_timeout_secs", 5))
        .merge(Serialized::default("database.idle_timeout_secs", 600))
        .merge(Serialized::default("database.max_lifetime_secs", 1800))
        // Exposées par défaut : la documentation doit exister dès la génération du
        // projet. La couper est un geste de mise en production, pas l'état initial.
        .merge(Serialized::default("docs.swagger_ui", true))
        .merge(Serialized::default("docs.openapi_json", true));

    // Quinze minutes et trente jours. Aucun défaut pour le secret : poser la table
    // `auth` sans lui fait nommer `secret` par le message d'erreur du chargement.
    #[cfg(feature = "auth")]
    let base = base
        .merge(Serialized::default("auth.access_ttl_secs", 900))
        .merge(Serialized::default("auth.refresh_ttl_secs", 2_592_000));

    let base = base.merge(Toml::file("config/default.toml"));

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

    /// Secret satisfaisant la longueur minimale, pour les cas qui ne portent pas sur lui.
    #[cfg(feature = "auth")]
    const SECRET_DE_TEST: &str = "un secret de test qui porte au moins trente-deux octets";

    /// Le flag `auth` rend `auth.secret` requis. Les cas qui portent sur autre chose le
    /// fournissent par l'environnement plutôt que d'alourdir chaque fixture TOML.
    #[cfg(feature = "auth")]
    fn secret_de_test(jail: &mut Jail) {
        jail.set_env("RBS_AUTH__SECRET", SECRET_DE_TEST);
    }

    #[cfg(not(feature = "auth"))]
    fn secret_de_test(_jail: &mut Jail) {}

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
            secret_de_test(jail);
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
            secret_de_test(jail);
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
            secret_de_test(jail);
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
            secret_de_test(jail);
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
            secret_de_test(jail);
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
    fn les_reglages_du_pool_ont_des_defauts_sans_configuration() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.set_env("RBS_DATABASE__URL", "postgres://localhost/app");

            let database = Config::load()
                .expect("la configuration doit se charger")
                .database;

            assert_eq!(database.max_connections, 10);
            assert_eq!(database.min_connections, 0);
            assert_eq!(database.connect_timeout_secs, 5);
            assert_eq!(database.acquire_timeout_secs, 5);
            assert_eq!(database.idle_timeout_secs, 600);
            assert_eq!(database.max_lifetime_secs, 1800);
            Ok(())
        });
    }

    #[test]
    fn un_reglage_du_pool_se_surcharge_par_l_environnement() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.set_env("RBS_DATABASE__URL", "postgres://localhost/app");
            jail.set_env("RBS_DATABASE__MAX_CONNECTIONS", "42");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.database.max_connections, 42);
            Ok(())
        });
    }

    #[test]
    fn les_valeurs_par_defaut_s_appliquent_sans_aucun_fichier() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.set_env("RBS_DATABASE__URL", "postgres://localhost/app");

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.env, "development");
            assert_eq!(config.server.host, "127.0.0.1");
            assert_eq!(config.server.port, 8080);
            Ok(())
        });
    }

    #[test]
    fn sans_section_docs_swagger_et_le_document_json_sont_exposes() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;

            let config = Config::load().expect("la configuration doit se charger");

            assert!(config.docs.swagger_ui);
            assert!(config.docs.openapi_json);
            Ok(())
        });
    }

    #[test]
    fn couper_swagger_laisse_le_document_json_expose() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!("{DEFAULT_TOML}\n[docs]\nswagger_ui = false\n"),
            )?;

            let config = Config::load().expect("la configuration doit se charger");

            assert!(!config.docs.swagger_ui);
            assert!(
                config.docs.openapi_json,
                "les deux réglages doivent rester indépendants"
            );
            Ok(())
        });
    }

    #[test]
    fn une_variable_d_environnement_coupe_swagger() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            secret_de_test(jail);
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;
            jail.set_env("RBS_DOCS__SWAGGER_UI", "false");

            let config = Config::load().expect("la configuration doit se charger");

            assert!(!config.docs.swagger_ui);
            Ok(())
        });
    }

    #[cfg(feature = "auth")]
    #[test]
    fn un_secret_absent_fait_echouer_le_chargement_en_nommant_le_champ() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;

            let erreur = Config::load().expect_err("`auth.secret` n'a pas de défaut");

            let message = erreur.to_string();
            assert!(
                message.contains("`secret`") && message.contains("auth"),
                "le message doit nommer le champ fautif, obtenu : {message}"
            );
            Ok(())
        });
    }

    #[cfg(feature = "auth")]
    #[test]
    fn un_secret_de_moins_de_32_octets_est_refuse_au_chargement() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!("{DEFAULT_TOML}\n[auth]\nsecret = \"trop court\"\n"),
            )?;

            let erreur = Config::load().expect_err("un secret de 10 octets doit être refusé");

            assert!(
                matches!(erreur, ConfigError::SecretTropCourt(10)),
                "obtenu : {erreur:?}"
            );
            Ok(())
        });
    }

    #[cfg(feature = "auth")]
    #[test]
    fn sans_duree_de_vie_configuree_l_acces_dure_quinze_minutes_et_le_refresh_trente_jours() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!(
                    "{DEFAULT_TOML}\n[auth]\nsecret = \"un secret de test qui porte trente-deux octets\"\n"
                ),
            )?;

            let auth = Config::load()
                .expect("la configuration doit se charger")
                .auth;

            assert_eq!(auth.access_ttl_secs, 900);
            assert_eq!(auth.refresh_ttl_secs, 2_592_000);
            Ok(())
        });
    }

    /// Section dont le noyau ignore tout : ni champ dans [`Config`], ni défaut opposé.
    #[derive(Debug, Deserialize)]
    struct SectionEtrangere {
        url: String,
        #[serde(default = "ttl_par_defaut")]
        ttl_secs: u64,
    }

    /// Défaut porté par l'appelant, et par lui seul.
    fn ttl_par_defaut() -> u64 {
        300
    }

    #[test]
    fn une_section_absente_rend_une_erreur_nommant_la_section() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;

            let erreur = section::<SectionEtrangere>("externe")
                .expect_err("aucune section `externe` n'est déclarée");

            let message = erreur.to_string();
            assert!(
                message.contains("externe"),
                "le message doit nommer la section demandée, obtenu : {message}"
            );
            Ok(())
        });
    }

    #[test]
    fn pour_une_section_etrangere_le_profil_puis_l_environnement_l_emportent() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!("{DEFAULT_TOML}\n[externe]\nurl = \"depuis-default\"\nttl_secs = 1\n"),
            )?;
            jail.create_file(
                "config/production.toml",
                "[externe]\nurl = \"depuis-le-profil\"\nttl_secs = 2\n",
            )?;
            jail.set_env("RBS_ENV", "production");
            jail.set_env("RBS_EXTERNE__TTL_SECS", "3");

            let externe =
                section::<SectionEtrangere>("externe").expect("la section doit se charger");

            assert_eq!(
                externe.url, "depuis-le-profil",
                "le fichier du profil doit écraser le fichier par défaut"
            );
            assert_eq!(
                externe.ttl_secs, 3,
                "la variable d'environnement doit écraser les deux fichiers"
            );
            Ok(())
        });
    }

    #[test]
    fn les_defauts_serde_de_l_appelant_sont_respectes() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!("{DEFAULT_TOML}\n[externe]\nurl = \"depuis-default\"\n"),
            )?;

            let externe =
                section::<SectionEtrangere>("externe").expect("la section doit se charger");

            assert_eq!(
                externe.ttl_secs, 300,
                "le noyau n'oppose aucun défaut : celui de l'appelant doit s'appliquer"
            );
            Ok(())
        });
    }
}
