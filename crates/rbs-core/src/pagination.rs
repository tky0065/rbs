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
pub struct Pagination {
    page: u64,
    per_page: u64,
}

/// Ce que le client a écrit dans la chaîne de requête, avant bornage.
#[derive(Debug, Deserialize)]
struct Parametres {
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
        let Query(parametres) = Query::<Parametres>::from_request_parts(parts, state)
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

    /// Interroge `/` avec la chaîne de requête donnée et rend `(statut, corps JSON)`.
    async fn interroger(query: &str) -> (StatusCode, Value) {
        async fn handler(pagination: Pagination) -> axum::Json<Value> {
            axum::Json(json!({
                "page": pagination.page(),
                "per_page": pagination.per_page(),
                "offset": pagination.offset(),
            }))
        }

        let reponse = Router::new()
            .route("/", get(handler))
            .oneshot(
                Request::builder()
                    .uri(format!("/?{query}"))
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le routeur doit répondre");

        let status = reponse.status();
        let octets = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (status, serde_json::from_slice(&octets).expect("corps JSON"))
    }

    #[tokio::test]
    async fn les_valeurs_par_defaut_s_appliquent_sans_parametre() {
        let (status, corps) = interroger("").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(corps["page"], PAGE_PAR_DEFAUT);
        assert_eq!(corps["per_page"], PAR_PAGE_PAR_DEFAUT);
    }

    #[tokio::test]
    async fn per_page_au_dela_du_maximum_est_plafonne_sans_erreur() {
        let (status, corps) = interroger("per_page=5000").await;

        assert_eq!(status, StatusCode::OK, "le plafonnement est muet : {corps}");
        assert_eq!(corps["per_page"], PAR_PAGE_MAX);
    }

    #[tokio::test]
    async fn page_zero_est_ramenee_a_la_premiere_page() {
        let (status, corps) = interroger("page=0&per_page=0").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(corps["page"], 1);
        assert_eq!(corps["per_page"], 1, "une page vide n'aurait aucun sens");
    }

    #[tokio::test]
    async fn un_parametre_non_numerique_repond_400() {
        let (status, corps) = interroger("per_page=abc").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(corps["status"], 400);
    }

    #[tokio::test]
    async fn l_offset_suit_la_page_demandee() {
        let (_, corps) = interroger("page=3&per_page=20").await;

        assert_eq!(corps["offset"], 40);
    }

    #[test]
    fn l_enveloppe_porte_les_donnees_et_leur_meta() {
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
    fn un_ensemble_vide_ne_compte_aucune_page() {
        let page: Page<&str> = Page::new(Vec::new(), &Pagination::new(1, 20), 0);

        let rendu = serde_json::to_value(&page).expect("sérialisable");

        assert_eq!(rendu["meta"]["total_pages"], 0);
        assert_eq!(rendu["data"], json!([]));
    }
}
