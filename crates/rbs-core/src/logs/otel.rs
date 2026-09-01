//! Export OTLP des spans, greffé sur l'abonné que [`init`](super::init) pose.
//!
//! Les deux réglages viennent de l'environnement et non de `config/default.toml` :
//! `logs::init()` s'exécute avant `Config::load()` et n'a donc aucune configuration à
//! lire. Ce sont par ailleurs les noms que tout collecteur et tout opérateur connaissent.

use std::env;
use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::registry::LookupSpan;

use super::LogError;

/// Variable nommant le collecteur OTLP. Absente, aucun span n'est exporté.
pub const VARIABLE_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";

/// Variable nommant le service dans les traces exportées. À défaut, le nom du binaire.
pub const VARIABLE_SERVICE: &str = "OTEL_SERVICE_NAME";

/// Le nom de service d'un processus dont l'exécutable n'est pas nommable, cas qu'un
/// binaire supprimé sous ses propres pieds suffit à produire.
const SERVICE_INCONNU: &str = "unknown_service";

/// Le fournisseur monté par [`couche`], que [`shutdown`] vide.
///
/// Gardé ici plutôt que rendu à l'appelant : `init()` ne rend rien, et un projet qui
/// devrait faire voyager ce fournisseur jusqu'à la fin de son `main` porterait la moitié
/// de la mécanique que le noyau existe pour lui épargner.
static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// La couche d'export, ou `None` quand aucun collecteur n'est nommé.
///
/// Un développeur qui lance `cargo run` sur son poste ne doit pas payer le démarrage d'un
/// exportateur vers un endpoint injoignable : l'absence de variable est un mode de
/// fonctionnement, pas une faute.
pub(super) fn couche<S>() -> Result<Option<OpenTelemetryLayer<S, SdkTracer>>, LogError>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    if env::var_os(VARIABLE_ENDPOINT).is_none() {
        return Ok(None);
    }

    // L'endpoint n'est pas transmis au constructeur : `opentelemetry-otlp` lit la même
    // variable, et la lui repasser ferait diverger les deux lectures au premier nom
    // d'option qui change.
    let exportateur = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;

    let fournisseur = SdkTracerProvider::builder()
        .with_batch_exporter(exportateur)
        .with_resource(Resource::builder().with_service_name(nom_service()).build())
        .build();

    let tracer = fournisseur.tracer("rbs-core");

    // Un second `init()` dans le même processus ne remplace pas le premier fournisseur :
    // `shutdown` doit vider celui dont les lots sont réellement en attente.
    let _ = PROVIDER.set(fournisseur);

    Ok(Some(tracing_opentelemetry::layer().with_tracer(tracer)))
}

/// Le nom sous lequel le service paraît dans les traces exportées.
///
/// À défaut de [`VARIABLE_SERVICE`], le nom du binaire en cours. Et non
/// `CARGO_PKG_NAME` : cette macro s'évalue à la compilation de *cette* crate, et
/// nommerait donc le noyau pour toutes les applications qui l'embarquent.
fn nom_service() -> String {
    if let Ok(nomme) = env::var(VARIABLE_SERVICE) {
        return nomme;
    }

    env::current_exe()
        .ok()
        .and_then(|chemin| {
            chemin
                .file_stem()
                .map(|nom| nom.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| SERVICE_INCONNU.to_owned())
}

/// Vide les lots en attente et rend la main quand ils sont partis.
pub(super) fn shutdown() {
    let Some(fournisseur) = PROVIDER.get() else {
        return;
    };

    if let Err(faute) = fournisseur.shutdown() {
        // Un arrêt est un arrêt : refuser de rendre la main parce que le collecteur ne
        // répond plus ferait d'une perte de traces une panne d'extinction.
        tracing::warn!(%faute, "traces en attente non exportées à l'arrêt");
    }
}

/// Dit si un exportateur a été monté, ce que seul le test de l'absence de collecteur
/// peut constater de l'extérieur.
#[cfg(test)]
pub(super) fn exportateur_installe() -> bool {
    PROVIDER.get().is_some()
}
