use std::time::Duration;

use anyhow::Context;
use rbs_core::HasCoreState;

use help_desk::{router, state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rbs_core::logs::init()?;

    let config = rbs_core::Config::load()?;
    let adresse = format!("{}:{}", config.server.host, config.server.port);
    let arret = Duration::from_secs(config.server.shutdown_timeout_secs);
    let db = rbs_core::db::connect(&config.database).await?;

    let state = state::AppState::new(db, config)?;

    // <rbs:startup>
    // </rbs:startup>

    let shutdown = state.core().shutdown().clone();
    let app = router::router(state);

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter sur {adresse}"))?;

    tracing::info!(%adresse, "démarrage");

    // Le routeur est servi avec l'adresse du pair, et non nu : c'est le seul endroit d'où
    // elle peut entrer dans la requête, et une couche qui distingue ses clients — une
    // limite de débit — n'a pas d'autre source.
    //
    // Ctrl-C ou SIGTERM cesse d'accepter et laisse finir les requêtes en vol ; les tâches
    // de fond détachées par l'état — un worker, un ticker — apprennent l'arrêt au même
    // instant et sont attendues ensuite, au plus `server.shutdown_timeout_secs`.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown.on_signal())
    .await?;

    shutdown.wait(arret).await;

    // En dernier : les spans partent par lots, et celui des derniers jobs partirait sans
    // cet appel.
    rbs_core::logs::shutdown();

    Ok(())
}
