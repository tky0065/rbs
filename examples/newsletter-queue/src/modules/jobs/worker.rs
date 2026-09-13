use std::time::Duration;

use rbs_core::HasCoreState;
use rbs_core::shutdown::Shutdown;
use sea_orm::DatabaseConnection;

use super::Config;
use super::model::{Model, Status};
use super::{Registry, queue, registry};
use crate::state::AppState;

/// Détache le worker dans le runtime, et rend la main aussitôt.
///
/// Une configuration illisible retire le worker en le disant, plutôt que d'emporter le
/// serveur avec lui : l'API répond encore, et la file se remplit sans se vider.
///
/// Détaché par le signal d'arrêt de l'état, et non par `tokio::spawn` : c'est ce qui fait
/// que `main` l'attend, job en cours compris, avant de sortir.
pub fn spawn(state: AppState) {
    let shutdown = state.core().shutdown().clone();

    shutdown.spawn(async move {
        if let Err(error) = run(state).await {
            tracing::error!(%error, "le worker de la file ne démarre pas");
        }
    });
}

/// Dépile jusqu'à l'arrêt du processus.
pub async fn run(state: AppState) -> anyhow::Result<()> {
    let config = Config::load()?;

    run_with(state, registry(), config).await;

    Ok(())
}

/// La boucle, sur un registre et une configuration reçus.
///
/// Séparée de `run` pour que les tests du fragment la jouent sur un registre à eux :
/// celui du projet ne connaît pas leurs jobs.
///
/// Un job réservé est toujours exécuté jusqu'au bout et son sort inscrit, arrêt demandé ou
/// non : c'est ce qui rend le bail de `lease_secs` inutile en temps normal.
pub(super) async fn run_with(state: AppState, registry: Registry, config: Config) {
    let shutdown = state.core().shutdown().clone();
    let attente = Duration::from_secs(config.poll_interval_secs);

    tracing::info!(
        poll_interval_secs = config.poll_interval_secs,
        lease_secs = config.lease_secs,
        "worker prêt"
    );

    while !shutdown.is_requested() {
        reprendre_les_abandonnes(state.core().db(), &config).await;

        match queue::reserver_prochain_job(state.core().db()).await {
            Ok(Some(job)) => execute(&state, &registry, &config, job).await,
            Ok(None) => pause(&shutdown, attente).await,
            // Une base momentanément injoignable ne condamne pas la file : le worker
            // retente au tour suivant plutôt que de rendre la main pour de bon.
            Err(error) => {
                tracing::error!(%error, "dépilage impossible");
                pause(&shutdown, attente).await;
            }
        }
    }

    tracing::info!("worker arrêté");
}

/// Dort `duree`, ou moins si l'arrêt est demandé entre-temps.
async fn pause(shutdown: &Shutdown, duree: Duration) {
    tokio::select! {
        _ = tokio::time::sleep(duree) => {}
        _ = shutdown.requested() => {}
    }
}

/// Reprend ce qu'un worker mort a laissé en `running` — au démarrage, puis à chaque
/// tour : deux `UPDATE` indexés qui ne touchent rien le plus souvent, contre une
/// livraison perdue sans bruit à chaque redémarrage.
///
/// Un échec ne retire pas le worker : la base injoignable sera dite par le dépilage qui
/// suit, et la reprise retentera au tour d'après.
async fn reprendre_les_abandonnes(db: &DatabaseConnection, config: &Config) {
    match queue::requeue_stale(db, config).await {
        Ok(reprise) => {
            if reprise.rendus > 0 {
                tracing::warn!(rendus = reprise.rendus, "jobs abandonnés rendus à la file");
            }
            if reprise.condamnes > 0 {
                tracing::warn!(
                    condamnes = reprise.condamnes,
                    "jobs abandonnés condamnés, sans tentative restante"
                );
            }
        }
        Err(error) => tracing::error!(%error, "reprise des jobs abandonnés impossible"),
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

    // Le sort du job n'a pas pu être inscrit : la ligne reste en `running` jusqu'à la fin
    // du bail, où elle sera rejouée. Le dire est tout ce que le worker peut faire — la
    // base ne répond pas.
    if let Err(error) = inscription {
        tracing::error!(job = %job.id, %error, "sort du job non inscrit");
    }
}
