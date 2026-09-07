use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderName, HeaderValue, RETRY_AFTER};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub mod config;
mod counter;

#[cfg(test)]
mod tests;

pub use config::{Config, Route, Rule};
pub use counter::Counter;

use crate::state::AppState;

/// En-tête par lequel un reverse proxy transmet l'adresse du client.
const FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

/// La limite de débit du projet, partagée par tous les handlers.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: Config,
    counter: Counter,
}

impl RateLimiter {
    /// Construit la limite depuis la section `[rate_limit]` de la configuration.
    pub fn from_config() -> anyhow::Result<Self> {
        Self::new(Config::load()?)
    }

    /// Construit la limite sur une configuration déjà chargée.
    pub fn new(config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            counter: Counter::new()?,
            config,
        })
    }

    /// La configuration que cette limite applique.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Compte une requête et dit si elle dépasse la limite qui la vise.
    ///
    /// La clé associe l'adresse à la portée de la règle, et non au chemin : sans quoi la
    /// limite globale se dédoublerait à chaque route nouvelle, et n'en serait plus une.
    pub async fn depasse(&self, client: IpAddr, path: &str) -> anyhow::Result<Option<Duration>> {
        let rule = self.config.rule(path);
        let count = self
            .counter
            .hit(&format!("rl|{client}|{}", rule.scope), rule.window)
            .await?;

        Ok((count > rule.limit).then_some(rule.window))
    }
}

/// Refuse les requêtes d'un client qui dépasse sa limite.
///
/// Une requête sans adresse cliente traverse sans être comptée : un compteur unique pour
/// tout le monde ferait payer à chacun ce qu'un seul consomme, et le garde-fou
/// deviendrait lui-même le déni de service. `axum::serve` fournit cette adresse ; un test
/// qui appelle le routeur en direct, non.
pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let limiter = state.rate_limit();

    let Some(client) = client(limiter.config(), request.headers(), request.extensions()) else {
        return next.run(request).await;
    };

    match limiter.depasse(client, request.uri().path()).await {
        Ok(Some(window)) => refus(window),
        Ok(None) => next.run(request).await,
        Err(error) => {
            // Un compteur injoignable ne ferme pas l'API : refuser tout le trafic parce
            // que Redis redémarre coûterait plus cher que ce que la limite protège.
            tracing::warn!(%error, "limite de débit non appliquée");
            next.run(request).await
        }
    }
}

/// L'adresse à laquelle la requête est imputée, si elle est connue.
fn client(
    config: &Config,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
) -> Option<IpAddr> {
    if config.trust_forwarded_for
        && let Some(transmise) = forwarded_for(headers)
    {
        return Some(transmise);
    }

    extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(pair)| pair.ip())
}

/// La première adresse de `X-Forwarded-For` : celle du client, les proxys suivant.
fn forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(&FORWARDED_FOR)?
        .to_str()
        .ok()?
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

/// La réponse 429, au format d'erreur du projet.
fn refus(window: Duration) -> Response {
    let mut response = rbs_core::Error::Domain {
        status: StatusCode::TOO_MANY_REQUESTS,
        code: "too_many_requests",
        message: "trop de requêtes : réessayez plus tard".to_string(),
    }
    .into_response();

    // La fenêtre entière plutôt que ce qu'il en reste : le compteur ne connaît le second
    // qu'au prix d'un aller-retour de plus, et majorer l'attente ne dessert que celui qui
    // a dépassé. Sans cet en-tête, un client qui réessaie le fait à l'aveugle.
    response
        .headers_mut()
        .insert(RETRY_AFTER, HeaderValue::from(window.as_secs().max(1)));

    response
}

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart
// avec elle.
impl AppState {
    /// La limite de débit partagée, telle que le middleware la lit depuis l'état.
    pub fn rate_limit(&self) -> &RateLimiter {
        &self.rate_limit
    }
}
