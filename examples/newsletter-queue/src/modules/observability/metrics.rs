use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Étiquette `path` d'une requête qui ne correspond à aucune route.
///
/// Une constante, jamais le chemin reçu : un scanner qui frappe mille adresses
/// inexistantes n'ouvre ainsi qu'une série, et non mille.
const HORS_ROUTE: &str = "<hors route>";

/// Bornes de l'histogramme de latence, en secondes.
///
/// Serrées sous la seconde, où se joue la latence d'une API, et espacées au-delà : les
/// quantiles ne se calculent qu'entre deux bornes, et un `0.5` isolé ne dirait plus rien
/// d'une réponse à 30 ms.
const BORNES: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Installe le registre du processus et rend de quoi le publier au format Prometheus.
///
/// Un seul registre par processus : le second appel échoue, la façade `metrics` ne
/// gardant qu'un enregistreur global.
pub fn install() -> anyhow::Result<PrometheusHandle> {
    Ok(PrometheusBuilder::new()
        .set_buckets(BORNES)?
        .install_recorder()?)
}

/// Compte les requêtes, mesure leur latence, et suit celles qui sont en vol.
///
/// Les trois séries portent le gabarit de route et non l'adresse reçue : voir [`gabarit`].
pub async fn middleware(request: Request, next: Next) -> Response {
    let method = request.method().as_str().to_owned();
    let path = gabarit(&request);

    metrics::gauge!("http_requests_in_flight").increment(1.0);
    let debut = Instant::now();

    let response = next.run(request).await;

    let latence = debut.elapsed().as_secs_f64();
    metrics::gauge!("http_requests_in_flight").decrement(1.0);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method.clone(),
        "path" => path.clone(),
    )
    .record(latence);

    metrics::counter!(
        "http_requests_total",
        "method" => method,
        "path" => path,
        "status" => response.status().as_str().to_owned(),
    )
    .increment(1);

    response
}

/// Le gabarit de route sous lequel la requête est comptée.
///
/// `MatchedPath` et jamais l'adresse demandée : `/articles/{id}` fait une série,
/// `/articles/0192f3…` en ferait une par article, et le collecteur tomberait au bout de
/// quelques heures. C'est la contrainte qui décide de tout ce module.
fn gabarit(request: &Request) -> String {
    request.extensions().get::<MatchedPath>().map_or_else(
        || HORS_ROUTE.to_owned(),
        |gabarit| gabarit.as_str().to_owned(),
    )
}
