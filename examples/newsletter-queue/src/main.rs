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
    // </rbs:startup>

    let app = router::router(state);

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter sur {adresse}"))?;

    tracing::info!(%adresse, "démarrage");
    axum::serve(listener, app).await?;

    Ok(())
}
