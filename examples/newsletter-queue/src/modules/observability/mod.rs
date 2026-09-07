pub mod config;
pub mod metrics;

#[cfg(test)]
mod tests;

use anyhow::Context;
use axum::Router;
use axum::routing::get;
use rbs_core::HasCoreState;

pub use config::Config;

use crate::state::AppState;

/// Installe le registre de métriques et sert `/metrics` sur son propre listener.
///
/// Le registre est posé avant que la fonction ne rende la main : le routeur du projet se
/// construit juste après, et un middleware qui compterait dans un registre pas encore
/// installé perdrait les premières requêtes.
///
/// `/metrics` n'est monté sur le routeur public à aucun moment. Les métriques publient la
/// topologie interne du service — routes, volumétrie, versions ; les exposer sur le port
/// de l'API demanderait à chaque déploiement une règle de reverse-proxy pour les cacher,
/// et un déploiement qui l'oublie fuit sans le savoir.
///
/// Les traces, elles, partent d'elles-mêmes vers `OTEL_EXPORTER_OTLP_ENDPOINT` : le noyau
/// les greffe sur l'abonné qu'il pose à la première ligne du `main`. Rien ici ne les
/// concerne, sinon qu'un `rbs_core::logs::shutdown()` avant la fin du processus est ce
/// qui pousse le dernier lot au lieu de le perdre.
///
/// # Erreurs
///
/// Échoue si la section `[observability]` est illisible ou si le port est déjà pris — le
/// cas le plus courant étant `metrics_port` égal à `server.port`, que `rbs doctor`
/// signale avant le démarrage.
pub async fn serve(state: &AppState) -> anyhow::Result<()> {
    let config = Config::load()?;
    let handle = metrics::install()?;

    // Le même hôte que l'API, un port distinct : une écoute sur une autre interface se
    // règle où l'API règle la sienne, et non dans une seconde clé qui la contredirait.
    let adresse = format!(
        "{}:{}",
        state.core().config().server.host,
        config.metrics_port
    );

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter les métriques sur {adresse}"))?;

    let app = Router::new().route(
        "/metrics",
        get(move || {
            let handle = handle.clone();
            async move { handle.render() }
        }),
    );

    tracing::info!(%adresse, "métriques");

    tokio::spawn(async move {
        if let Err(faute) = axum::serve(listener, app).await {
            tracing::error!(%faute, "le listener des métriques s'est arrêté");
        }
    });

    Ok(())
}
