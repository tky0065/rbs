//! Ouverture du pool de connexions à la base.
//!
//! Le pool s'ouvre au démarrage, jamais paresseusement : une base injoignable doit
//! arrêter le processus tout de suite, pas surgir au premier appel HTTP.

use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::DatabaseConfig;

/// Ce qui remplace le mot de passe dans un texte journalisé.
const MASQUE: &str = "***";

/// Échec de l'ouverture du pool au démarrage.
///
/// Distincte d'[`Error`](crate::Error), comme [`ConfigError`](crate::config::ConfigError)
/// et pour la même raison : une panne au démarrage ne devient jamais une réponse HTTP.
///
/// La cause est conservée sous forme de texte expurgé, non de `DbErr` : SeaORM inclut
/// l'URL de connexion complète dans ses messages, mot de passe compris.
#[derive(Debug, thiserror::Error)]
#[error(
    "connexion à la base impossible : {cause}\n\
     base visée : {url}\n\
     vérifiez `database.url` (RBS_DATABASE__URL) et {conseil}"
)]
pub struct ConnectError {
    /// URL visée, mot de passe masqué.
    url: String,
    /// Cause remontée par SeaORM, mot de passe masqué.
    cause: String,
    /// Ce qu'il reste à vérifier, propre au moteur que l'URL désigne.
    conseil: String,
}

/// Ce que l'URL laisse vérifier en plus du champ, selon le moteur qu'elle désigne.
///
/// Le moteur se lit dans le schéma et non dans les métadonnées du projet : le noyau est
/// une bibliothèque, et le `Cargo.toml` qui l'emploie ne lui appartient pas. SQLite n'a
/// pas de serveur à démarrer, mais un fichier à rendre accessible.
fn conseil(url: &str) -> String {
    let moteur = match url.split_once("://").map(|(scheme, _)| scheme) {
        Some("postgres" | "postgresql") => "PostgreSQL",
        Some("mysql") => "MySQL",
        Some("sqlite") => {
            return "que le fichier de base est accessible en écriture, \
                    son répertoire compris"
                .to_owned();
        }
        _ => return "que le serveur de base est démarré".to_owned(),
    };

    format!("que le serveur {moteur} est démarré")
}

/// Ouvre le pool de connexions décrit par `config`.
///
/// # Erreurs
///
/// Échoue si l'URL est inexploitable ou si le serveur ne répond pas dans le délai de
/// connexion configuré. Le message nomme le champ à corriger.
pub async fn connect(config: &DatabaseConfig) -> Result<DatabaseConnection, ConnectError> {
    let mut options = ConnectOptions::new(&config.url);
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
        // SeaORM journaliserait chaque requête via `log`, en doublon du middleware de
        // trace qui porte déjà le `request_id`.
        .sqlx_logging(false);

    Database::connect(options).await.map_err(|source| {
        let secret = password(&config.url);
        ConnectError {
            url: strip(&config.url, secret),
            cause: strip(&source.to_string(), secret),
            conseil: conseil(&config.url),
        }
    })
}

/// Isole le mot de passe d'une URL de connexion, s'il en porte un.
///
/// Le découpage est textuel plutôt que par parsing d'URL, pour traiter aussi les chaînes
/// que le parseur rejette — ce sont précisément celles qui finissent dans un message
/// d'erreur.
fn password(url: &str) -> Option<&str> {
    let autorite = &url[url.find("://")? + 3..];
    let fin = autorite.find('/').unwrap_or(autorite.len());
    let arobase = autorite[..fin].rfind('@')?;
    let deux_points = autorite[..arobase].find(':')?;

    Some(&autorite[deux_points + 1..arobase])
}

/// Remplace `secret` par [`MASQUE`] partout dans `text`.
///
/// Le remplacement est global et non ancré : un mot de passe qui serait aussi le nom de
/// la base masquerait les deux. Masquer de trop est le bon sens de l'erreur pour un texte
/// qui part dans les logs.
fn strip(text: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() => text.replace(secret, MASQUE),
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(url: &str) -> DatabaseConfig {
        DatabaseConfig {
            url: url.to_owned(),
            max_connections: 10,
            min_connections: 0,
            connect_timeout_secs: 5,
            acquire_timeout_secs: 5,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        }
    }

    /// Masque le mot de passe que `url` porte elle-même.
    fn mask(url: &str) -> String {
        strip(url, password(url))
    }

    #[tokio::test]
    async fn an_invalid_url_fails_with_a_message_naming_the_field() {
        let error = connect(&config("pas-une-url"))
            .await
            .expect_err("`pas-une-url` n'est pas une URL de connexion");

        let message = error.to_string();
        assert!(
            message.contains("database.url"),
            "le message doit nommer le field à corriger, obtenu : {message}"
        );
    }

    // Le message est celui que lit un développeur dont l'application refuse de démarrer :
    // lui parler de PostgreSQL quand il a configuré MySQL l'envoie chercher au mauvais
    // endroit. SQLite n'a pas de serveur du tout — sa phrase change de nature.
    #[tokio::test]
    async fn the_message_names_the_engine_actually_configured() {
        for (url, attendu) in [
            ("postgres://alice@localhost:99999/app", "PostgreSQL"),
            ("postgresql://alice@localhost:99999/app", "PostgreSQL"),
            ("mysql://alice@localhost:99999/app", "MySQL"),
        ] {
            let error = connect(&config(url))
                .await
                .expect_err("le port 99999 est hors bornes");

            let message = error.to_string();
            assert!(
                message.contains(attendu),
                "le message ne nomme pas {attendu} : {message}"
            );
            assert!(
                message.contains("est démarré"),
                "le message ne parle pas d'un serveur : {message}"
            );
        }
    }

    #[tokio::test]
    async fn sqlite_is_told_about_a_file_rather_than_a_server() {
        let error = connect(&config("sqlite:///introuvable/app.db"))
            .await
            .expect_err("le répertoire n'existe pas");

        let message = error.to_string();
        assert!(
            message.contains("fichier"),
            "SQLite n'a pas de serveur : le message doit parler du fichier : {message}"
        );
        assert!(
            !message.contains("démarré"),
            "SQLite n'a rien à démarrer : {message}"
        );
    }

    #[tokio::test]
    async fn the_password_does_not_appear_in_the_error_message() {
        let error = connect(&config("postgres://alice:s3cr3t@localhost:99999/app"))
            .await
            .expect_err("le port 99999 est hors bornes");

        let message = format!("{error} {error:?}");
        assert!(
            !message.contains("s3cr3t"),
            "mot de passe divulgué dans l'error : {message}"
        );
        assert!(
            message.contains("localhost"),
            "l'hôte visé reste utile au diagnostic, obtenu : {message}"
        );
    }

    #[test]
    fn masking_replaces_the_password_and_preserves_the_rest() {
        assert_eq!(
            mask("postgres://alice:s3cr3t@localhost:5432/app"),
            "postgres://alice:***@localhost:5432/app"
        );
    }

    #[test]
    fn masking_leaves_urls_without_a_password_intact() {
        for url in [
            "postgres://alice@localhost/app",
            "postgres://localhost/app",
            "pas-une-url",
            "",
        ] {
            assert_eq!(mask(url), url, "URL modifiée à tort : {url}");
        }
    }

    #[test]
    fn masking_ignores_a_colon_placed_after_the_authority() {
        assert_eq!(
            mask("postgres://localhost:5432/app"),
            "postgres://localhost:5432/app"
        );
    }
}
