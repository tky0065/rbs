mod health;
mod openapi;
mod router;
mod state;
// <rbs:features>
mod articles;
// </rbs:features>

use anyhow::Context;

// region: demarrage
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rbs_core::logs::init()?;

    let config = rbs_core::Config::load()?;
    let adresse = format!("{}:{}", config.server.host, config.server.port);
    let db = rbs_core::db::connect(&config.database).await?;

    let app = router::router(state::AppState::new(db, config)?);

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter sur {adresse}"))?;

    tracing::info!(%adresse, "démarrage");
    axum::serve(listener, app).await?;

    Ok(())
}
// endregion: demarrage
