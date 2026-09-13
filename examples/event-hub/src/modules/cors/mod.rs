use std::time::Duration;

use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

pub mod config;

#[cfg(test)]
mod tests;

pub use config::Config;

/// L'origine qui ouvre l'API à tout le monde, telle qu'elle s'écrit dans `[cors]`.
const JOKER: &str = "*";

/// La couche CORS du projet, telle que `src/router.rs` la pose.
///
/// Une section illisible ne rend pas une couche permissive : elle rend une couche qui
/// n'autorise aucune origine, et le journal dit pourquoi. Ouvrir l'API parce qu'un
/// fichier de configuration est fautif serait décider à la place du développeur.
pub fn layer() -> CorsLayer {
    match Config::load().map_err(Error::from).and_then(|c| build(&c)) {
        Ok(couche) => couche,
        Err(error) => {
            tracing::error!(%error, "aucune origine autorisée : la section [cors] est inexploitable");
            CorsLayer::new()
        }
    }
}

/// Ce qui rend une section `[cors]` inexploitable.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// La section ne se lit pas.
    #[error("{0}")]
    Config(#[from] rbs_core::config::ConfigError),

    /// Le navigateur refuse d'envoyer des identifiants à une origine joker : la
    /// combinaison n'a aucun effet utile, et la croire active est le vrai danger.
    #[error("cors.credentials = true ne se combine pas avec l'origine `*`")]
    JokerAvecIdentifiants,

    /// Une entrée de la section n'est pas une valeur d'en-tête HTTP.
    #[error("cors.{champ} : `{valeur}` n'est pas une valeur acceptable")]
    Invalide {
        /// Clé fautive de la section.
        champ: &'static str,
        /// Entrée refusée, telle qu'elle est écrite.
        valeur: String,
    },
}

/// Construit la couche que `config` décrit.
pub fn build(config: &Config) -> Result<CorsLayer, Error> {
    Ok(CorsLayer::new()
        .allow_origin(origins(config)?)
        .allow_methods(methods(config)?)
        .allow_headers(headers(config)?)
        .allow_credentials(config.credentials)
        .max_age(Duration::from_secs(config.max_age_secs)))
}

/// Les origines autorisées : la liste énumérée, ou toutes si elle porte le joker.
fn origins(config: &Config) -> Result<AllowOrigin, Error> {
    if config.origins.iter().any(|origine| origine == JOKER) {
        if config.credentials {
            return Err(Error::JokerAvecIdentifiants);
        }

        return Ok(AllowOrigin::any());
    }

    let origines = config
        .origins
        .iter()
        .map(|origine| {
            HeaderValue::from_str(origine).map_err(|_| Error::Invalide {
                champ: "origins",
                valeur: origine.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AllowOrigin::list(origines))
}

fn methods(config: &Config) -> Result<Vec<Method>, Error> {
    config
        .methods
        .iter()
        .map(|methode| {
            Method::from_bytes(methode.as_bytes()).map_err(|_| Error::Invalide {
                champ: "methods",
                valeur: methode.clone(),
            })
        })
        .collect()
}

fn headers(config: &Config) -> Result<Vec<HeaderName>, Error> {
    config
        .headers
        .iter()
        .map(|entete| {
            HeaderName::from_bytes(entete.as_bytes()).map_err(|_| Error::Invalide {
                champ: "headers",
                valeur: entete.clone(),
            })
        })
        .collect()
}
