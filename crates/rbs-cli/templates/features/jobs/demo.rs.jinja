use serde::{Deserialize, Serialize};

use super::Job;
use crate::state::AppState;

/// Le job d'exemple : il écrit son message dans les logs.
///
/// Il est là pour que la file ait quelque chose à exécuter le jour où elle est installée.
/// Écrivez les vôtres sur ce modèle, et retirez son inscription de `jobs::registry`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    pub message: String,
}

#[async_trait::async_trait]
impl Job for Log {
    const KIND: &'static str = "log";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        tracing::info!(message = %self.message, "job `log`");

        Ok(())
    }
}
