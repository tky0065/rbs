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
    let reponse = next.run(request).instrument(span.clone()).await;
    let latence = debut.elapsed();

    // Émis dans le span, pour que l'événement porte lui aussi le `request_id` : statut et
    // latence ne sont connus qu'ici, quand le span est déjà ouvert.
    let _entree = span.enter();
    tracing::info!(
        status = reponse.status().as_u16(),
        latency_ms = latence.as_secs_f64() * 1_000.0,
        "request"
    );

    reponse
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

    /// Appelle `/` sous un abonné jetable et rend `(identifiant renvoyé, lignes JSON)`.
    ///
    /// Le test est synchrone : `capture` pose l'abonné pour le thread courant le temps
    /// d'une closure, et le futur y est mené à terme par un runtime local.
    fn appeler(routeur: Router) -> (String, Vec<Value>) {
        let mut identifiant = String::new();

        let sortie = capture(JsonFormat::new(), JsonFormat::new(), || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime de test");

            identifiant = runtime.block_on(async {
                let reponse = routeur
                    .oneshot(
                        Request::builder()
                            .uri("/")
                            .body(Body::empty())
                            .expect("requête valide"),
                    )
                    .await
                    .expect("le routeur doit répondre");

                reponse
                    .headers()
                    .get(request_id::X_REQUEST_ID)
                    .expect("la réponse doit porter l'en-tête")
                    .to_str()
                    .expect("en-tête ASCII")
                    .to_owned()
            });
        });

        let lignes = sortie
            .lines()
            .map(|ligne| serde_json::from_str(ligne).unwrap_or_else(|e| panic!("({e}) {ligne}")))
            .collect();

        (identifiant, lignes)
    }

    /// Monte `handler` derrière les deux middlewares, dans l'ordre que le module impose.
    fn routeur<H, T>(handler: H) -> Router
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
    fn un_log_emis_dans_un_handler_porte_le_request_id_de_sa_requete() {
        async fn handler() -> &'static str {
            tracing::info!("depuis le handler");
            "ok"
        }

        let (identifiant, lignes) = appeler(routeur(handler));

        let ligne = lignes
            .iter()
            .find(|ligne| ligne["msg"] == "depuis le handler")
            .expect("le log du handler doit être journalisé");
        assert_eq!(
            ligne["request_id"],
            Value::String(identifiant),
            "le log du handler doit porter le request_id de sa requête : {ligne}"
        );
    }

    #[test]
    fn l_evenement_final_porte_la_methode_le_chemin_le_statut_et_la_latence() {
        async fn handler() -> &'static str {
            "ok"
        }

        let (identifiant, lignes) = appeler(routeur(handler));

        let ligne = lignes
            .iter()
            .find(|ligne| ligne["msg"] == "request")
            .expect("l'événement de fin de requête doit être journalisé");
        assert_eq!(ligne["method"], "GET");
        assert_eq!(ligne["path"], "/");
        assert_eq!(ligne["status"], 200);
        assert_eq!(ligne["request_id"], Value::String(identifiant));
        assert!(
            ligne["latency_ms"].is_number(),
            "latence absente ou non numérique : {ligne}"
        );
    }

    #[test]
    fn une_reponse_d_erreur_est_tracee_avec_son_statut() {
        async fn handler() -> StatusCode {
            StatusCode::INTERNAL_SERVER_ERROR
        }

        let (_, lignes) = appeler(routeur(handler));

        let ligne = lignes
            .iter()
            .find(|ligne| ligne["msg"] == "request")
            .expect("l'événement de fin de requête doit être journalisé");
        assert_eq!(ligne["status"], 500);
    }
}
