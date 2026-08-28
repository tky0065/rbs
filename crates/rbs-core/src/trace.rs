//! Trace d'une requête HTTP.
//!
//! Ce middleware s'applique **à l'intérieur** de celui de [`request_id`](crate::request_id) :
//! il lit l'identifiant que ce dernier a posé, et n'en trouverait aucun s'il s'exécutait
//! avant lui.
//!
//! ```text
//! Router::new()
//!     .layer(axum::middleware::from_fn(rbs_core::trace::middleware))
//!     .layer(axum::middleware::from_fn(rbs_core::request_id::middleware))
//! ```

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

use crate::request_id;

/// Ouvre un span par requête et journalise son issue.
///
/// Le span porte le `request_id`, la méthode et le chemin : les deux formateurs
/// remontant les champs des spans parents, tout log émis pendant la requête en hérite
/// sans avoir à les répéter.
pub async fn middleware(request: Request, next: Next) -> Response {
    // Lus avant que `next.run` ne consomme la requête.
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let span = tracing::info_span!(
        "requete",
        request_id = request_id::current().unwrap_or_default(),
        method = %method,
        path = %path,
    );

    let debut = Instant::now();
    let response = next.run(request).instrument(span.clone()).await;
    let latence = debut.elapsed();

    // Émis dans le span, pour que l'événement porte lui aussi le `request_id` : statut et
    // latence ne sont connus qu'ici, quand le span est déjà ouvert.
    let _input = span.enter();
    tracing::info!(
        status = response.status().as_u16(),
        latency_ms = latence.as_secs_f64() * 1_000.0,
        "request"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::{JsonFormat, aide::capture};
    use crate::request_id;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use serde_json::Value;
    use tower::ServiceExt;

    /// Appelle `/` sous un abonné jetable et rend `(identifiant renvoyé, lines JSON)`.
    ///
    /// Le test est synchrone : `capture` pose l'abonné pour le thread courant le temps
    /// d'une closure, et le futur y est mené à terme par un runtime local.
    fn call(router: Router) -> (String, Vec<Value>) {
        let mut identifiant = String::new();

        let output = capture(JsonFormat::new(), JsonFormat::new(), || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime de test");

            identifiant = runtime.block_on(async {
                let response = router
                    .oneshot(
                        Request::builder()
                            .uri("/")
                            .body(Body::empty())
                            .expect("requête valide"),
                    )
                    .await
                    .expect("le router doit répondre");

                response
                    .headers()
                    .get(request_id::X_REQUEST_ID)
                    .expect("la réponse doit porter l'en-tête")
                    .to_str()
                    .expect("en-tête ASCII")
                    .to_owned()
            });
        });

        let lines = output
            .lines()
            .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("({e}) {line}")))
            .collect();

        (identifiant, lines)
    }

    /// Monte `handler` derrière les deux middlewares, dans l'ordre que le module impose.
    fn router<H, T>(handler: H) -> Router
    where
        H: axum::handler::Handler<T, ()>,
        T: 'static,
    {
        Router::new()
            .route("/", get(handler))
            .layer(axum::middleware::from_fn(middleware))
            .layer(axum::middleware::from_fn(request_id::middleware))
    }

    #[test]
    fn a_log_emitted_in_a_handler_carries_the_request_id_of_its_request() {
        async fn handler() -> &'static str {
            tracing::info!("depuis le handler");
            "ok"
        }

        let (identifiant, lines) = call(router(handler));

        let line = lines
            .iter()
            .find(|line| line["msg"] == "depuis le handler")
            .expect("le log du handler doit être journalisé");
        assert_eq!(
            line["request_id"],
            Value::String(identifiant),
            "le log du handler doit porter le request_id de sa requête : {line}"
        );
    }

    #[test]
    fn the_final_event_carries_the_method_the_path_the_status_and_the_latency() {
        async fn handler() -> &'static str {
            "ok"
        }

        let (identifiant, lines) = call(router(handler));

        let line = lines
            .iter()
            .find(|line| line["msg"] == "request")
            .expect("l'événement de fin de requête doit être journalisé");
        assert_eq!(line["method"], "GET");
        assert_eq!(line["path"], "/");
        assert_eq!(line["status"], 200);
        assert_eq!(line["request_id"], Value::String(identifiant));
        assert!(
            line["latency_ms"].is_number(),
            "latence absente ou non numérique : {line}"
        );
    }

    #[test]
    fn an_error_response_is_traced_with_its_status() {
        async fn handler() -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        let (_, lines) = call(router(handler));

        let line = lines
            .iter()
            .find(|line| line["msg"] == "request")
            .expect("l'événement de fin de requête doit être journalisé");
        assert_eq!(line["status"], 500);
    }
}
