use anyhow::Context;
use newsletter_queue::{router, state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rbs_core::logs::init()?;

    let config = rbs_core::Config::load()?;
    let adresse = format!("{}:{}", config.server.host, config.server.port);
    let db = rbs_core::db::connect(&config.database).await?;

    let state = state::AppState::new(db, config)?;

    // <rbs:startup>
    newsletter_queue::jobs::worker::spawn(state.clone());
    newsletter_queue::observability::serve(&state).await?;
    // </rbs:startup>

    let app = router::router(state);

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter sur {adresse}"))?;

    tracing::info!(%adresse, "démarrage");

    // Le routeur est servi avec l'adresse du pair, et non nu : c'est le seul endroit d'où
    // elle peut entrer dans la requête, et une couche qui distingue ses clients — une
    // limite de débit — n'a pas d'autre source.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    // region: arret
    // Les spans partent par lots, et un processus qui meurt entre deux emporte le
    // dernier. Rien dans le squelette n'appelle ceci : le coût d'un oubli est ce lot, non
    // une panne, et l'appel ne fait rien tant que `rbs-core` n'a pas sa feature.
    rbs_core::logs::shutdown();
    // endregion: arret

    Ok(())
}
