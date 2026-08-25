//! Montre la bascule `RBS_LOG_FORMAT` sur les mêmes événements.
//!
//! `RBS_LOG_FORMAT=json cargo run -p rbs-core --example logs_format`

fn main() {
    if let Err(erreur) = rbs_core::logs::init() {
        eprintln!("{erreur}");
        std::process::exit(1);
    }

    tracing::info!(env = "dev", port = 3000, "serveur démarré");
    tracing::warn!(actives = 18, max = 20, "pool proche de la saturation");

    let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
    let _entree = span.enter();
    tracing::error!(status = 422, latency_ms = 12.4, "requête refusée");
}
