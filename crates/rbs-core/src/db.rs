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
     vérifiez `database.url` (RBS_DATABASE__URL) et que le serveur PostgreSQL est démarré"
)]
pub struct ConnectError {
    /// URL visée, mot de passe masqué.
    url: String,
    /// Cause remontée par SeaORM, mot de passe masqué.
    cause: String,
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
        let secret = mot_de_passe(&config.url);
        ConnectError {
            url: expurger(&config.url, secret),
            cause: expurger(&source.to_string(), secret),
        }
    })
}

/// Isole le mot de passe d'une URL de connexion, s'il en porte un.
///
/// Le découpage est textuel plutôt que par parsing d'URL, pour traiter aussi les chaînes
/// que le parseur rejette — ce sont précisément celles qui finissent dans un message
/// d'erreur.
fn mot_de_passe(url: &str) -> Option<&str> {
    let autorite = &url[url.find("://")? + 3..];
    let fin = autorite.find('/').unwrap_or(autorite.len());
    let arobase = autorite[..fin].rfind('@')?;
    let deux_points = autorite[..arobase].find(':')?;

    Some(&autorite[deux_points + 1..arobase])
}

/// Remplace `secret` par [`MASQUE`] partout dans `texte`.
///
/// Le remplacement est global et non ancré : un mot de passe qui serait aussi le nom de
/// la base masquerait les deux. Masquer de trop est le bon sens de l'erreur pour un texte
/// qui part dans les logs.
fn expurger(texte: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() => texte.replace(secret, MASQUE),
        _ => texte.to_owned(),
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
    fn masquer(url: &str) -> String {
        expurger(url, mot_de_passe(url))
    }

    #[tokio::test]
    async fn une_url_invalide_echoue_avec_un_message_nommant_le_champ() {
        let erreur = connect(&config("pas-une-url"))
            .await
            .expect_err("`pas-une-url` n'est pas une URL de connexion");

        let message = erreur.to_string();
        assert!(
            message.contains("database.url"),
            "le message doit nommer le champ à corriger, obtenu : {message}"
        );
    }

    #[tokio::test]
    async fn le_mot_de_passe_n_apparait_pas_dans_le_message_d_erreur() {
        let erreur = connect(&config("postgres://alice:s3cr3t@localhost:99999/app"))
            .await
            .expect_err("le port 99999 est hors bornes");

        let message = format!("{erreur} {erreur:?}");
        assert!(
            !message.contains("s3cr3t"),
            "mot de passe divulgué dans l'erreur : {message}"
        );
        assert!(
            message.contains("localhost"),
            "l'hôte visé reste utile au diagnostic, obtenu : {message}"
        );
    }

    #[test]
    fn le_masquage_remplace_le_mot_de_passe_et_preserve_le_reste() {
        assert_eq!(
            masquer("postgres://alice:s3cr3t@localhost:5432/app"),
            "postgres://alice:***@localhost:5432/app"
        );
    }

    #[test]
    fn le_masquage_laisse_intactes_les_url_sans_mot_de_passe() {
        for url in [
            "postgres://alice@localhost/app",
            "postgres://localhost/app",
            "pas-une-url",
            "",
        ] {
            assert_eq!(masquer(url), url, "URL modifiée à tort : {url}");
        }
    }

    #[test]
    fn le_masquage_ignore_un_deux_points_situe_apres_l_autorite() {
        assert_eq!(
            masquer("postgres://localhost:5432/app"),
            "postgres://localhost:5432/app"
        );
    }
}
