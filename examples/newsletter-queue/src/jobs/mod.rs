pub mod config;
pub mod model;
pub mod newsletter;
pub mod queue;
pub mod worker;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub use config::Config;
// Réexportée pour que le projet écrive `jobs::enqueue(&transaction, &job)`, ce que
// `subscribers::service::broadcast` fait.
pub use queue::enqueue;
// L'envoi daté n'a pas d'appelant ici : la porte reste ouverte à qui voudra programmer
// une lettre plutôt que la diffuser sur-le-champ.
#[allow(unused_imports)]
pub use queue::enqueue_at;

use crate::state::AppState;

// region: trait
/// Un travail que le worker exécute hors du cycle d'une requête.
///
/// `KIND` est écrit dans la ligne et sert de clé au registre : le renommer sans migration
/// laisse en file des jobs que plus rien ne sait exécuter.
#[async_trait::async_trait]
pub trait Job: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Le nom sous lequel le job est enfilé, puis retrouvé.
    const KIND: &'static str;

    /// Ce que le job fait. Toute erreur rendue ici vaut réessai.
    async fn run(&self, state: &AppState) -> anyhow::Result<()>;
}
// endregion: trait

/// Un job dont le type a été oublié, tel que le registre le garde.
type Handler = Box<
    dyn Fn(AppState, serde_json::Value) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Ce que le worker sait exécuter.
#[derive(Default)]
pub struct Registry {
    handlers: HashMap<&'static str, Handler>,
}

impl Registry {
    /// Un registre vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit un job, et rend le registre pour enchaîner les inscriptions.
    pub fn register<J: Job>(mut self) -> Self {
        self.handlers.insert(
            J::KIND,
            Box::new(|state, payload| {
                Box::pin(async move {
                    let job: J = serde_json::from_value(payload)?;
                    job.run(&state).await
                })
            }),
        );

        self
    }

    /// Exécute le job décrit par une ligne de la file.
    ///
    /// Un `kind` inconnu est traité comme un échec du job et non du worker : la ligne part
    /// en réessai puis en échec définitif, et la file continue d'avancer.
    pub async fn run(
        &self,
        state: &AppState,
        kind: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<()> {
        let handler = self
            .handlers
            .get(kind)
            .ok_or_else(|| anyhow::anyhow!("aucun job n'est inscrit sous `{kind}`"))?;

        handler(state.clone(), payload).await
    }
}

// region: registry
/// Les jobs de ce projet. Inscrivez les vôtres ici.
///
/// Un `kind` absent d'ici part en réessai puis en échec : le registre est le seul endroit
/// où l'oubli se voie, et il ne se voit qu'à l'exécution.
pub fn registry() -> Registry {
    Registry::new().register::<newsletter::SendNewsletter>()
}
// endregion: registry
