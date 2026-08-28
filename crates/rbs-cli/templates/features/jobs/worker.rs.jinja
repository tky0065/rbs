use std::time::Duration;

use rbs_core::HasCoreState;

use super::Config;
use super::model::{Model, Status};
use super::{Registry, queue, registry};
use crate::state::AppState;

/// Détache le worker dans le runtime, et rend la main aussitôt.
///
/// Une configuration illisible retire le worker en le disant, plutôt que d'emporter le
/// serveur avec lui : l'API répond encore, et la file se remplit sans se vider.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        if let Err(error) = run(state).await {
            tracing::error!(%error, "le worker de la file ne démarre pas");
        }
    });
}

/// Dépile jusqu'à l'arrêt du processus.
pub async fn run(state: AppState) -> anyhow::Result<()> {
    let config = Config::load()?;
    let registry = registry();
    let attente = Duration::from_secs(config.poll_interval_secs);

    tracing::info!(
        poll_interval_secs = config.poll_interval_secs,
        "worker prêt"
    );

    loop {
        match queue::reserver_prochain_job(state.core().db()).await {
            Ok(Some(job)) => execute(&state, &registry, &config, job).await,
            Ok(None) => tokio::time::sleep(attente).await,
            // Une base momentanément injoignable ne condamne pas la file : le worker
            // retente au tour suivant plutôt que de rendre la main pour de bon.
            Err(error) => {
                tracing::error!(%error, "dépilage impossible");
                tokio::time::sleep(attente).await;
            }
        }
    }
}

/// Exécute un job réservé, puis inscrit son sort dans la ligne.
///
/// Visible du module : les tests du fragment jouent le réessai tour par tour, ce que la
/// boucle infinie de `run` ne permet pas.
pub(super) async fn execute(state: &AppState, registry: &Registry, config: &Config, job: Model) {
    let db = state.core().db();
    let result = registry.run(state, &job.kind, job.payload.clone()).await;

    let inscription = match result {
        Ok(()) => {
            tracing::debug!(job = %job.id, kind = %job.kind, "job exécuté");
            queue::mark_done(db, &job).await.map(|()| Status::Done)
        }
        Err(error) => {
            tracing::warn!(job = %job.id, kind = %job.kind, attempts = job.attempts, %error, "job en échec");
            queue::retry_or_fail(db, &job, config, &error).await
        }
    };

    // Le sort du job n'a pas pu être inscrit : la ligne reste en `running` et n'est plus
    // dépilée. Le dire est tout ce que le worker peut faire — la base ne répond pas.
    if let Err(error) = inscription {
        tracing::error!(job = %job.id, %error, "sort du job non inscrit");
    }
}
