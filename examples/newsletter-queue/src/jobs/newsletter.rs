use minijinja::context;
use rbs_core::HasCoreState;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

use super::Job;
use crate::state::AppState;
use crate::subscribers::repository;

// region: job
/// L'envoi d'une lettre à un abonné.
///
/// Le payload porte l'identifiant et non l'adresse : entre l'enfilage et l'exécution, un
/// abonné a pu corriger la sienne, et c'est celle de l'envoi qui compte.
#[derive(Debug, Serialize, Deserialize)]
pub struct SendNewsletter {
    pub subscriber: Uuid,
    pub subject: String,
    pub body: String,
}

#[async_trait::async_trait]
impl Job for SendNewsletter {
    const KIND: &'static str = "send_newsletter";

    /// Une erreur rendue ici vaut réessai, puis échec définitif après N tentatives.
    ///
    /// C'est tout ce qui sépare ce job de `Mailer::send_detached`, dont l'échec ne laisse
    /// qu'une ligne de journal : un SMTP indisponible une minute perd la lettre détachée
    /// et fait patienter celle-ci.
    async fn run(&self, state: &AppState) -> anyhow::Result<()> {
        let subscriber = repository::find(state.core().db(), self.subscriber)
            .await?
            .ok_or_else(|| anyhow::anyhow!("l'abonné {} n'existe plus", self.subscriber))?;

        state
            .mail
            .send_template(
                &subscriber.email,
                &self.subject,
                "newsletter.html",
                context! { name => subscriber.name, body => self.body },
            )
            .await?;

        Ok(())
    }
}
// endregion: job
