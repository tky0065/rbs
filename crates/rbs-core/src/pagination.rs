//! Pagination des listes.
//!
//! Les bornes sont des constantes du noyau plutôt qu'une configuration : l'extracteur
//! reste ainsi sans état, montable sur n'importe quel routeur. Un projet qui aurait
//! vraiment besoin d'autres bornes écrit son propre extracteur — la frontière du projet
//! l'y encourage déjà.

use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::Error;

/// Première page, quand le client n'en demande aucune.
pub const PAGE_PAR_DEFAUT: u64 = 1;

/// Taille de page appliquée par défaut.
pub const PAR_PAGE_PAR_DEFAUT: u64 = 20;

/// Taille de page maximale qu'un client peut obtenir.
pub const PAR_PAGE_MAX: u64 = 100;

/// Fenêtre de pagination demandée par le client, déjà bornée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pagination {
    page: u64,
    per_page: u64,
}

/// Ce que le client a écrit dans la chaîne de requête, avant bornage.
#[derive(Debug, Deserialize)]
struct Params {
    page: Option<u64>,
    per_page: Option<u64>,
}

impl Pagination {
    /// Construit une fenêtre en ramenant `page` et `per_page` dans leurs bornes.
    pub fn new(page: u64, per_page: u64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, PAR_PAGE_MAX),
        }
    }

    /// Numéro de page demandé, à partir de 1.
    pub fn page(&self) -> u64 {
        self.page
    }

    /// Nombre d'éléments par page.
    pub fn per_page(&self) -> u64 {
        self.per_page
    }

    /// Nombre d'éléments à sauter pour atteindre cette page.
    ///
    /// Exposé pour que chaque repository généré ne recalcule pas `(page - 1) * per_page`,
    /// avec une chance sur deux de se tromper d'une unité.
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.per_page
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self::new(PAGE_PAR_DEFAUT, PAR_PAGE_PAR_DEFAUT)
    }
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Traitement asymétrique voulu : une valeur hors bornes est ramenée en silence,
        // mais une valeur illisible est signalée. Ignorer `per_page=abc` ferait débugger
        // au client une pagination qui « ne marche pas », sans rien pour l'aider.
        let Query(parametres) = Query::<Params>::from_request_parts(parts, state)
            .await
            .map_err(|rejet| Error::BadRequest(rejet.body_text()))?;

        Ok(Self::new(
            parametres.page.unwrap_or(PAGE_PAR_DEFAUT),
            parametres.per_page.unwrap_or(PAR_PAGE_PAR_DEFAUT),
        ))
    }
}

/// Une page de résultats et de quoi situer la suivante.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[non_exhaustive]
pub struct Page<T> {
    data: Vec<T>,
    meta: Meta,
}

/// Description de la page rendue.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
struct Meta {
    page: u64,
    per_page: u64,
    total: u64,
    total_pages: u64,
}

impl<T> Page<T> {
    /// Enveloppe `data` en décrivant sa place dans un ensemble de `total` éléments.
    pub fn new(data: Vec<T>, pagination: &Pagination, total: u64) -> Self {
        Self {
            data,
            meta: Meta {
                page: pagination.page(),
                per_page: pagination.per_page(),
                total,
                total_pages: total.div_ceil(pagination.per_page()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    /// Interroge `/` avec la chaîne de requête donnée et rend `(statut, body JSON)`.
    async fn query(query: &str) -> (StatusCode, Value) {
        async fn handler(pagination: Pagination) -> axum::Json<Value> {
            axum::Json(json!({
                "page": pagination.page(),
                "per_page": pagination.per_page(),
                "offset": pagination.offset(),
            }))
        }

        let response = Router::new()
            .route("/", get(handler))
            .oneshot(
                Request::builder()
                    .uri(format!("/?{query}"))
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (status, serde_json::from_slice(&bytes).expect("corps JSON"))
    }

    #[tokio::test]
    async fn the_default_values_apply_without_a_parameter() {
        let (status, body) = query("").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["page"], PAGE_PAR_DEFAUT);
        assert_eq!(body["per_page"], PAR_PAGE_PAR_DEFAUT);
    }

    #[tokio::test]
    async fn per_page_beyond_the_maximum_is_capped_without_an_error() {
        let (status, body) = query("per_page=5000").await;

        assert_eq!(status, StatusCode::OK, "le plafonnement est muet : {body}");
        assert_eq!(body["per_page"], PAR_PAGE_MAX);
    }

    #[tokio::test]
    async fn page_zero_is_brought_back_to_the_first_page() {
        let (status, body) = query("page=0&per_page=0").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["page"], 1);
        assert_eq!(body["per_page"], 1, "une page vide n'aurait aucun sens");
    }

    #[tokio::test]
    async fn a_non_numeric_parameter_answers_400() {
        let (status, body) = query("per_page=abc").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["status"], 400);
    }

    #[tokio::test]
    async fn the_offset_follows_the_requested_page() {
        let (_, body) = query("page=3&per_page=20").await;

        assert_eq!(body["offset"], 40);
    }

    #[test]
    fn the_envelope_carries_the_data_and_their_meta() {
        let pagination = Pagination::new(2, 20);

        let page = Page::new(vec!["a", "b"], &pagination, 143);

        assert_eq!(
            serde_json::to_value(&page).expect("sérialisable"),
            json!({
                "data": ["a", "b"],
                "meta": { "page": 2, "per_page": 20, "total": 143, "total_pages": 8 },
            })
        );
    }

    #[test]
    fn an_empty_set_counts_no_page() {
        let page: Page<&str> = Page::new(Vec::new(), &Pagination::new(1, 20), 0);

        let rendered = serde_json::to_value(&page).expect("sérialisable");

        assert_eq!(rendered["meta"]["total_pages"], 0);
        assert_eq!(rendered["data"], json!([]));
    }
}
