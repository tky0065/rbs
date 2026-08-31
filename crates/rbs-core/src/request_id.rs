//! Identifiant de corrélation de la requête courante.
//!
//! Ce module ne porte que le stockage : le middleware qui génère l'ULID, le reprend de
//! l'en-tête `x-request-id` et ouvre le scope vit ailleurs. Tout ce qui a besoin de
//! l'identifiant sans l'avoir reçu en paramètre — la réponse d'erreur, un log — le lit
//! par [`current`].

use std::future::Future;

use axum::extract::Request;
use axum::http::HeaderName;
use axum::http::header::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use ulid::Ulid;

/// En-tête portant l'identifiant, à l'entrée comme à la sortie.
pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Longueur maximale d'un identifiant repris de l'amont.
const LONGUEUR_MAX: usize = 128;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Attribue un identifiant à la requête et le renvoie dans la réponse.
///
/// L'identifiant est repris de l'en-tête `x-request-id` entrant quand l'amont en fournit
/// un exploitable, sinon un ULID est généré. Il reste lisible par [`current`] pendant
/// toute la requête.
pub async fn middleware(request: Request, next: Next) -> Response {
    let id = reprendre(request.headers().get(&X_REQUEST_ID))
        .unwrap_or_else(|| Ulid::generate().to_string());

    let mut response = scope(id.clone(), next.run(request)).await;

    // `reprendre` n'accepte que de l'ASCII imprimable et un ULID n'en sort pas : la
    // conversion ne peut pas échouer, mais on ne la force pas pour autant.
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(X_REQUEST_ID, value);
    }

    response
}

/// Retient un identifiant amont, s'il est exploitable.
///
/// Cette valeur part dans chaque ligne de log de la requête et revient au client. Un
/// `HeaderValue` peut porter des octets arbitraires : sans borne, l'amont choisirait la
/// taille du journal et ce qu'on y écrit.
fn reprendre(entrant: Option<&HeaderValue>) -> Option<String> {
    let entrant = entrant?.to_str().ok()?;

    let exploitable = !entrant.is_empty()
        && entrant.len() <= LONGUEUR_MAX
        && entrant.chars().all(|c| c.is_ascii_graphic());

    exploitable.then(|| entrant.to_owned())
}

/// Identifiant de la requête en cours, `None` en dehors d'une requête.
pub fn current() -> Option<String> {
    REQUEST_ID.try_with(String::clone).ok()
}

/// Exécute `future` en lui associant `id` comme identifiant de requête.
pub async fn scope<F>(id: String, future: F) -> F::Output
where
    F: Future,
{
    REQUEST_ID.scope(id, future).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use tower::ServiceExt;

    /// Envoie une requête au travers du middleware et renvoie
    /// `(en-tête de réponse, identifiant vu par le handler)`.
    async fn call(entrant: Option<&str>) -> (String, String) {
        async fn handler() -> String {
            current().unwrap_or_default()
        }

        let app = Router::new()
            .route("/", get(handler))
            .layer(axum::middleware::from_fn(middleware));

        let mut requete = Request::builder().uri("/");
        if let Some(entrant) = entrant {
            requete = requete.header(X_REQUEST_ID, entrant);
        }

        let response = app
            .oneshot(requete.body(Body::empty()).expect("requête valide"))
            .await
            .expect("le router doit répondre");

        let header = response
            .headers()
            .get(X_REQUEST_ID)
            .expect("la réponse doit porter l'en-tête")
            .to_str()
            .expect("en-tête ASCII")
            .to_owned();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (
            header,
            String::from_utf8(body.to_vec()).expect("corps UTF-8"),
        )
    }

    #[tokio::test]
    async fn current_returns_none_outside_a_request() {
        assert_eq!(current(), None);
    }

    #[tokio::test]
    async fn current_returns_the_scope_identifier() {
        let vu = scope("01JQ3F8K2P".to_string(), async { current() }).await;

        assert_eq!(vu.as_deref(), Some("01JQ3F8K2P"));
    }

    #[tokio::test]
    async fn two_requests_receive_two_distinct_identifiers() {
        let (premier, _) = call(None).await;
        let (second, _) = call(None).await;

        assert_ne!(premier, second);
        assert_eq!(premier.len(), 26, "un ULID fait 26 caractères : {premier}");
    }

    #[tokio::test]
    async fn an_incoming_header_is_kept_as_is_in_the_response() {
        let (header, _) = call(Some("trace-amont-42")).await;

        assert_eq!(header, "trace-amont-42");
    }

    #[tokio::test]
    async fn an_aberrant_header_is_ignored_in_favour_of_a_generated_ulid() {
        // Un saut de ligne ne peut pas franchir `HeaderValue` : la garde couvre ce que
        // la couche HTTP laisse passer, soit la longueur et les octets non-ASCII.
        for aberrant in [
            "x".repeat(LONGUEUR_MAX + 1),
            "trace-ÿ".to_owned(),
            String::new(),
        ] {
            let (header, _) = call(Some(&aberrant)).await;

            assert_ne!(header, aberrant, "en-tête aberrant repris : {aberrant:?}");
            assert_eq!(header.len(), 26, "un ULID était attendu : {header}");
        }
    }

    #[tokio::test]
    async fn the_handler_reads_the_identifier_of_its_own_request() {
        let (header, vu_par_le_handler) = call(None).await;

        assert_eq!(header, vu_par_le_handler);
    }
}
