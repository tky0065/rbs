//! Émet un événement par niveau pour juger à l'œil du rendu du formateur `pretty`.
//!
//! `cargo run -p rbs-core --example logs_pretty`

use rbs_core::logs::PrettyFormat;

fn main() {
    let abonne = tracing_subscriber::fmt()
        .fmt_fields(PrettyFormat::new())
        .event_format(PrettyFormat::new())
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(abonne).expect("aucun abonné global posé");

    tracing::trace!(requetes = 1_284, "statistiques du pool");
    tracing::debug!(chemin = "config/dev.toml", "fichier de configuration lu");
    tracing::info!(env = "dev", port = 3000, "serveur démarré");
    tracing::warn!(actives = 18, max = 20, "pool proche de la saturation");

    let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
    let _entree = span.enter();
    tracing::error!(status = 422, latency_ms = 12.4, "requête refusée");
}
