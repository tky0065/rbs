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

    let mut reponse = scope(id.clone(), next.run(request)).await;

    // `reprendre` n'accepte que de l'ASCII imprimable et un ULID n'en sort pas : la
    // conversion ne peut pas échouer, mais on ne la force pas pour autant.
    if let Ok(valeur) = HeaderValue::from_str(&id) {
        reponse.headers_mut().insert(X_REQUEST_ID, valeur);
    }

    reponse
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
    async fn appeler(entrant: Option<&str>) -> (String, String) {
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

        let reponse = app
            .oneshot(requete.body(Body::empty()).expect("requête valide"))
            .await
            .expect("le routeur doit répondre");

        let en_tete = reponse
            .headers()
            .get(X_REQUEST_ID)
            .expect("la réponse doit porter l'en-tête")
            .to_str()
            .expect("en-tête ASCII")
            .to_owned();
        let corps = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (
            en_tete,
            String::from_utf8(corps.to_vec()).expect("corps UTF-8"),
        )
    }

    #[tokio::test]
    async fn current_retourne_none_hors_requete() {
        assert_eq!(current(), None);
    }

    #[tokio::test]
    async fn current_retourne_l_identifiant_du_scope() {
        let vu = scope("01JQ3F8K2P".to_string(), async { current() }).await;

        assert_eq!(vu.as_deref(), Some("01JQ3F8K2P"));
    }

    #[tokio::test]
    async fn deux_requetes_recoivent_deux_identifiants_distincts() {
        let (premier, _) = appeler(None).await;
        let (second, _) = appeler(None).await;

        assert_ne!(premier, second);
        assert_eq!(premier.len(), 26, "un ULID fait 26 caractères : {premier}");
    }

    #[tokio::test]
    async fn un_en_tete_entrant_est_conserve_tel_quel_dans_la_reponse() {
        let (en_tete, _) = appeler(Some("trace-amont-42")).await;

        assert_eq!(en_tete, "trace-amont-42");
    }

    #[tokio::test]
    async fn un_en_tete_aberrant_est_ignore_au_profit_d_un_ulid_genere() {
        // Un saut de ligne ne peut pas franchir `HeaderValue` : la garde couvre ce que
        // la couche HTTP laisse passer, soit la longueur et les octets non-ASCII.
        for aberrant in [
            "x".repeat(LONGUEUR_MAX + 1),
            "trace-ÿ".to_owned(),
            String::new(),
        ] {
            let (en_tete, _) = appeler(Some(&aberrant)).await;

            assert_ne!(en_tete, aberrant, "en-tête aberrant repris : {aberrant:?}");
            assert_eq!(en_tete.len(), 26, "un ULID était attendu : {en_tete}");
        }
    }

    #[tokio::test]
    async fn le_handler_lit_l_identifiant_de_sa_propre_requete() {
        let (en_tete, vu_par_le_handler) = appeler(None).await;

        assert_eq!(en_tete, vu_par_le_handler);
    }
}
