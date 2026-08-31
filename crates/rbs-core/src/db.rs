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
#[non_exhaustive]
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
    let reste = &url[url.find("://")? + 3..];
    let arobase = fin_du_userinfo(reste)?;
    let deux_points = reste[..arobase].find(':')?;

    Some(&reste[deux_points + 1..arobase])
}

/// Position de l'arobase qui sépare le userinfo de l'hôte, s'il y en a une.
///
/// L'autorité ne s'arrête pas au premier `/` de la chaîne : un mot de passe non encodé
/// peut en porter un — un secret encodé en base64 en porte un caractère sur soixante-quatre
/// — et couper là laisse l'arobase de côté, donc le secret hors du masque. Elle ne va pas
/// non plus jusqu'à la dernière arobase de la chaîne : une arobase dans le nom de la base
/// ferait alors passer l'hôte et son port pour un mot de passe.
///
/// Le segment qui précède le premier `/` tranche entre les deux lectures : il porte
/// l'arobase quand l'autorité s'y termine ; sinon, son `:` introduit un port — un nombre —
/// si l'autorité est close, un mot de passe coupé en deux si elle continue au-delà du `/`.
fn fin_du_userinfo(reste: &str) -> Option<usize> {
    let premier_segment = &reste[..reste.find('/').unwrap_or(reste.len())];
    if let Some(arobase) = premier_segment.rfind('@') {
        return Some(arobase);
    }

    let mot_de_passe_coupe = premier_segment
        .split_once(':')
        .is_some_and(|(_, apres)| !apres.is_empty() && !apres.bytes().all(|b| b.is_ascii_digit()));

    if mot_de_passe_coupe {
        reste.rfind('@')
    } else {
        None
    }
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
            "le message doit nommer le champ à corriger, obtenu : {message}"
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
            "mot de passe divulgué dans l'erreur : {message}"
        );
        assert!(
            message.contains("localhost"),
            "l'hôte visé reste utile au diagnostic, obtenu : {message}"
        );
    }

    #[tokio::test]
    async fn a_password_carrying_a_slash_does_not_appear_either() {
        let error = connect(&config("postgres://alice:s3c/r3t@localhost:99999/app"))
            .await
            .expect_err("le port 99999 est hors bornes");

        let message = format!("{error} {error:?}");
        assert!(
            !message.contains("s3c/r3t"),
            "mot de passe divulgué dans l'erreur : {message}"
        );
    }

    #[test]
    fn masking_replaces_the_password_and_preserves_the_rest() {
        assert_eq!(
            mask("postgres://alice:s3cr3t@localhost:5432/app"),
            "postgres://alice:***@localhost:5432/app"
        );
    }

    // Un mot de passe non encodé porte souvent un `/` : un secret tiré au hasard puis
    // encodé en base64 en porte un caractère sur soixante-quatre. Arrêter l'autorité au
    // premier `/` de la chaîne coupait alors avant l'arobase, et le secret partait en
    // clair dans le message et dans les journaux.
    #[test]
    fn masking_replaces_a_password_carrying_a_slash() {
        for (url, attendu) in [
            (
                "postgres://alice:s3c/r3t@host/app",
                "postgres://alice:***@host/app",
            ),
            ("postgres://alice:s3c/r3t@host", "postgres://alice:***@host"),
            (
                "postgres://alice:s3c/r3t@host:5432/app?sslmode=require",
                "postgres://alice:***@host:5432/app?sslmode=require",
            ),
        ] {
            assert_eq!(mask(url), attendu, "mot de passe divulgué : {url}");
        }
    }

    // L'arobase que l'encodage aurait dû rendre en `%40` ne doit pas couper l'autorité
    // trop tôt : c'est la dernière qui sépare le userinfo de l'hôte.
    #[test]
    fn masking_replaces_a_password_carrying_an_at_sign() {
        for (url, attendu) in [
            (
                "postgres://alice:p@ss@host/app",
                "postgres://alice:***@host/app",
            ),
            (
                "postgres://alice:p%40ss@host/app",
                "postgres://alice:***@host/app",
            ),
        ] {
            assert_eq!(mask(url), attendu, "mot de passe divulgué : {url}");
        }
    }

    #[test]
    fn masking_leaves_urls_without_a_password_intact() {
        for url in [
            "postgres://alice@localhost/app",
            "postgres://localhost/app",
            // Une arobase dans le chemin ne fait pas de l'hôte un mot de passe.
            "postgres://localhost:5432/app@v2",
            "sqlite://app.db",
            "sqlite:///introuvable/app.db",
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
