// Le fragment livre une brique, pas une route : tant qu'aucun handler n'appelle le cache,
// le compilateur le tient pour mort. La ligne se retire au premier appel.
#![allow(dead_code)]

pub mod config;

#[cfg(test)]
mod tests;

use std::time::Duration;

use anyhow::Context;
use deadpool_redis::{Connection, Pool, Runtime};
use rbs_core::Result;
use redis::AsyncCommands;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub use config::Config;

use crate::state::AppState;

/// Le cache du projet, partagé par tous les handlers.
#[derive(Debug, Clone)]
pub struct Cache {
    pool: Pool,
    ttl: Duration,
}

impl Cache {
    /// Construit le cache depuis la section `[cache]` de la configuration.
    pub fn depuis_config() -> anyhow::Result<Self> {
        Self::nouveau(&Config::charger()?)
    }

    /// Construit le cache sur une configuration déjà chargée.
    ///
    /// Aucune connexion n'est ouverte ici : le pool est paresseux, et joint le serveur au
    /// premier appel. C'est ce qui permet à `AppState::new` de rester synchrone.
    pub fn nouveau(config: &Config) -> anyhow::Result<Self> {
        let pool = deadpool_redis::Config::from_url(&config.url)
            .create_pool(Some(Runtime::Tokio1))
            .with_context(|| format!("pool Redis inconstructible pour `{}`", config.url))?;

        Ok(Self {
            pool,
            ttl: Duration::from_secs(config.ttl_secs),
        })
    }

    /// Lit une valeur. Une clé absente ou expirée rend `None`.
    pub async fn get<T: DeserializeOwned>(&self, cle: &str) -> Result<Option<T>> {
        let mut connexion = self.connexion().await?;
        let brut: Option<Vec<u8>> = connexion
            .get(cle)
            .await
            .with_context(|| format!("lecture de `{cle}` impossible"))?;

        Ok(decoder(brut)?)
    }

    /// Écrit une valeur, pour la durée de vie que porte la configuration.
    pub async fn set<T: Serialize + ?Sized>(&self, cle: &str, valeur: &T) -> Result<()> {
        self.set_ttl(cle, valeur, self.ttl).await
    }

    /// Écrit une valeur pour une durée de vie donnée. Une durée nulle : aucune expiration.
    pub async fn set_ttl<T: Serialize + ?Sized>(
        &self,
        cle: &str,
        valeur: &T,
        ttl: Duration,
    ) -> Result<()> {
        let encode = encoder(valeur)?;
        let mut connexion = self.connexion().await?;
        let echec = || format!("écriture de `{cle}` impossible");

        if ttl.is_zero() {
            connexion
                .set::<_, _, ()>(cle, encode)
                .await
                .with_context(echec)?;
        } else {
            connexion
                .set_ex::<_, _, ()>(cle, encode, ttl.as_secs())
                .await
                .with_context(echec)?;
        }

        Ok(())
    }

    /// Retire une clé. Une clé absente n'est pas une erreur.
    pub async fn invalider(&self, cle: &str) -> Result<()> {
        let mut connexion = self.connexion().await?;
        connexion
            .del::<_, ()>(cle)
            .await
            .with_context(|| format!("suppression de `{cle}` impossible"))?;

        Ok(())
    }

    /// Retire toutes les clés d'un préfixe, et rend leur nombre.
    ///
    /// `SCAN` plutôt que `KEYS` : le second bloque le serveur le temps de parcourir tout
    /// l'espace de clés.
    pub async fn invalider_prefixe(&self, prefixe: &str) -> Result<usize> {
        let mut connexion = self.connexion().await?;
        let echec = || format!("balayage du préfixe `{prefixe}` impossible");

        let mut rendues = Vec::new();
        {
            let mut cles = connexion
                .scan_match::<_, String>(motif(prefixe))
                .await
                .with_context(echec)?;

            while let Some(cle) = cles.next_item().await {
                rendues.push(cle.with_context(echec)?);
            }
        }

        let cles = a_supprimer(prefixe, rendues);
        if cles.is_empty() {
            return Ok(0);
        }

        connexion
            .del::<_, ()>(&cles)
            .await
            .with_context(|| format!("suppression du préfixe `{prefixe}` impossible"))?;

        Ok(cles.len())
    }

    async fn connexion(&self) -> Result<Connection> {
        Ok(self
            .pool
            .get()
            .await
            .context("aucune connexion Redis disponible")?)
    }
}

fn encoder<T: Serialize + ?Sized>(valeur: &T) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(valeur).context("valeur non sérialisable")
}

/// `nil` — clé absente ou expirée — rend `None` : le cas courant d'un cache n'est pas
/// une panne, et l'appelant enchaîne sur sa source de vérité.
fn decoder<T: DeserializeOwned>(brut: Option<Vec<u8>>) -> anyhow::Result<Option<T>> {
    match brut {
        None => Ok(None),
        Some(octets) => serde_json::from_slice(&octets)
            .map(Some)
            .context("valeur en cache illisible"),
    }
}

/// Le motif `SCAN MATCH` d'un préfixe, métacaractères de glob échappés.
fn motif(prefixe: &str) -> String {
    let mut motif = String::with_capacity(prefixe.len() + 1);

    for caractere in prefixe.chars() {
        if matches!(caractere, '*' | '?' | '[' | ']' | '\\') {
            motif.push('\\');
        }
        motif.push(caractere);
    }

    motif.push('*');
    motif
}

/// Parmi les clés que le serveur a rendues, celles que le préfixe emporte réellement.
///
/// Le motif est un glob interprété à l'autre bout, et une suppression ne se défait pas :
/// le préfixe se revérifie ici, où il est sûr.
fn a_supprimer(prefixe: &str, mut cles: Vec<String>) -> Vec<String> {
    cles.retain(|cle| cle.starts_with(prefixe));
    cles
}

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart
// avec elle.
impl AppState {
    /// Le cache partagé, tel qu'un handler le lit depuis l'état.
    pub fn cache(&self) -> &Cache {
        &self.cache
    }
}
