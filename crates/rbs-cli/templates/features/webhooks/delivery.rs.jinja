use std::time::Duration;

use rbs_core::HasCoreState;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

use super::target::{Policy, Refusal, Resolver};
use super::{Config, repository, signature};
use crate::modules::jobs::Job;
use crate::state::AppState;

/// Ce que le receveur lit.
///
/// `id` est tiré à l'émission et voyage dans la charge utile du job : il est donc le même à
/// chaque réessai, ce qui est toute sa raison d'être — c'est la clé de déduplication du
/// receveur.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event: String,
    pub created_at: DateTimeWithTimeZone,
    pub data: serde_json::Value,
}

/// Une livraison à un abonné, telle qu'elle attend dans la file.
///
/// L'abonnement est désigné par son identifiant et non recopié : l'URL et le secret sont
/// relus au dépilage, si bien qu'un secret tourné s'applique aux livraisons déjà en file et
/// qu'une révocation les arrête.
#[derive(Debug, Serialize, Deserialize)]
pub struct Delivery {
    pub subscription: Uuid,
    pub event: Event,
}

#[async_trait::async_trait]
impl Job for Delivery {
    const KIND: &'static str = "webhooks::deliver";

    async fn run(&self, state: &AppState) -> anyhow::Result<()> {
        let db = state.core().db();

        let Some(abonnement) = repository::find(db, self.subscription).await? else {
            // L'abonnement a disparu de la table : il n'y a rien à livrer et rien à
            // réessayer. Un `Err` ferait cinq tentatives sur une ligne qui n'existe plus.
            tracing::info!(
                subscription = %self.subscription,
                event = %self.event.event,
                "livraison sans abonnement : abandonnée"
            );
            return Ok(());
        };

        // Relue au dépilage et non à l'émission : un abonnement révoqué entre les deux ne
        // reçoit rien, et le job se termine en succès — il n'y a rien à réessayer.
        if abonnement.revoked_at.is_some() {
            tracing::info!(
                subscription = %abonnement.id,
                event = %self.event.event,
                "abonnement révoqué : livraison abandonnée"
            );
            return Ok(());
        }

        // Sérialisé une fois, signé et envoyé : ces octets-là et pas d'autres. Sérialiser
        // deux fois exposerait à ce que l'ordre des clés change entre la signature et
        // l'envoi, et le receveur rejetterait une signature pourtant honnête.
        let corps = serde_json::to_vec(&self.event)?;
        let horodatage = chrono::Utc::now().timestamp();

        match state
            .webhooks()
            .post(
                &abonnement.url,
                &signature::header(&abonnement.secret, horodatage, &corps),
                &self.event.event,
                self.event.id,
                corps,
            )
            .await
        {
            Ok(()) => Ok(()),
            // Une cible interdite le restera : réessayer ne ferait que cinq lignes de
            // journal de plus. Le job se termine, et l'abonnement reste à révoquer.
            Err(PostError::Blocked(refus)) => {
                tracing::warn!(
                    subscription = %abonnement.id,
                    event = %self.event.event,
                    %refus,
                    "livraison vers une cible interdite : abandonnée"
                );
                Ok(())
            }
            Err(PostError::Transport(source)) => Err(source),
        }
    }
}

/// Le client HTTP des livraisons, partagé par le processus.
///
/// Un `reqwest::Client` porte son pool de connexions : le construire à chaque livraison
/// rouvrirait une session TLS par tentative. Il vit donc dans l'`AppState`, où le job le
/// retrouve.
#[derive(Clone, Debug)]
pub struct Sender {
    client: reqwest::Client,
    policy: Policy,
}

impl Sender {
    /// Construit le client d'après la section `[webhooks]` et le profil actif.
    ///
    /// L'échec remonte au démarrage plutôt qu'à la première livraison : un délai
    /// d'expiration illisible est une faute de configuration, et la découvrir six heures
    /// plus tard dans un journal de worker ne sert personne.
    pub fn from_config(config: &rbs_core::Config) -> anyhow::Result<Self> {
        let section = Config::load()?;
        let policy = Policy::for_env(&config.env);

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(section.timeout_secs))
                // Un 3xx est une réponse hors 2xx comme une autre : suivre une
                // redirection livrerait le corps signé là où le receveur — ou qui a pris
                // sa place — l'envoie, hors de toute politique.
                .redirect(reqwest::redirect::Policy::none())
                .dns_resolver(std::sync::Arc::new(Resolver::new(policy)))
                .build()?,
            policy,
        })
    }

    pub fn policy(&self) -> Policy {
        self.policy
    }

    /// POSTe un corps signé, en refusant la cible avant tout envoi plutôt qu'en laissant le
    /// transport échouer sur elle.
    ///
    /// Toute réponse hors 2xx vaut échec de transport, 4xx comprises : un receveur qui
    /// répond 400 à une livraison bien formée est en panne, et le distinguer d'un 503
    /// demanderait de deviner laquelle des deux parties a tort.
    pub(super) async fn post(
        &self,
        url: &str,
        signature: &str,
        event: &str,
        delivery: Uuid,
        body: Vec<u8>,
    ) -> Result<(), PostError> {
        let cible = self.policy.check(url).map_err(PostError::Blocked)?;

        // Résolue avant l'envoi pour que le refus soit nommé — le résolveur du client
        // refiltre à la connexion, mais son erreur arrive noyée dans celle du transport.
        let adresses = self
            .policy
            .resolve(&cible)
            .await
            .map_err(|source| PostError::Transport(source.into()))?;
        if adresses.is_empty() {
            return Err(PostError::Blocked(Refusal::PrivateHost));
        }

        let reponse = self
            .client
            .post(cible)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(signature::HEADER, signature)
            .header(signature::HEADER_EVENT, event)
            .header(signature::HEADER_DELIVERY, delivery.to_string())
            .body(body)
            .send()
            .await
            .map_err(|source| PostError::Transport(source.into()))?;

        let statut = reponse.status();

        if statut.is_success() {
            tracing::debug!(%url, %event, %delivery, status = statut.as_u16(), "livraison acceptée");
            return Ok(());
        }

        Err(PostError::Transport(anyhow::anyhow!(
            "{url} a répondu {statut}"
        )))
    }
}

/// Ce qui empêche une livraison, et ce que la file doit en faire.
#[derive(Debug)]
pub enum PostError {
    /// La cible est interdite : rien ne changera au prochain essai.
    Blocked(Refusal),
    /// Le transport ou le receveur a échoué : la file réessaie.
    Transport(anyhow::Error),
}
