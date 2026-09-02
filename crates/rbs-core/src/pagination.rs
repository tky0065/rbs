//! Pagination des listes.
//!
//! Les bornes sont des constantes du noyau plutôt qu'une configuration : l'extracteur
//! reste ainsi sans état, montable sur n'importe quel routeur. Un projet qui aurait
//! vraiment besoin d'autres bornes écrit son propre extracteur — la frontière du projet
//! l'y encourage déjà.

use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use sea_orm::prelude::Uuid;
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

/// Fenêtre de pagination par curseur, déjà bornée.
///
/// Là où [`Pagination`] saute `offset` lignes pour atteindre une page, le curseur reprend
/// la marche à l'`id` où elle s'était arrêtée : le moteur ne parcourt plus les lignes
/// qu'il va jeter, et une insertion survenue entre deux requêtes ne décale plus la
/// fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cursor {
    after: Option<Uuid>,
    per_page: u64,
}

/// Ce que le client a écrit dans la chaîne de requête, avant bornage.
#[derive(Debug, Deserialize)]
struct ParamsCurseur {
    after: Option<Uuid>,
    per_page: Option<u64>,
}

impl Cursor {
    /// Construit une fenêtre en ramenant `per_page` dans ses bornes.
    pub fn new(after: Option<Uuid>, per_page: u64) -> Self {
        Self {
            after,
            per_page: per_page.clamp(1, PAR_PAGE_MAX),
        }
    }

    /// Identifiant après lequel reprendre, `None` pour la première page.
    ///
    /// La borne est **exclusive** : le repository écrit `Column::Id.lt(after)`, sans quoi
    /// chaque page réafficherait la dernière ligne de la précédente.
    pub fn after(&self) -> Option<Uuid> {
        self.after
    }

    /// Nombre d'éléments par page.
    pub fn per_page(&self) -> u64 {
        self.per_page
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(None, PAR_PAGE_PAR_DEFAUT)
    }
}

impl<S> FromRequestParts<S> for Cursor
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Même asymétrie que `Pagination`, et pour la même raison : une taille de page
        // hors bornes est ramenée en silence, un curseur illisible est signalé. Repartir
        // du début sur un `after` cassé ferait boucler un client sur la première page
        // sans que rien ne le lui dise.
        let Query(parametres) = Query::<ParamsCurseur>::from_request_parts(parts, state)
            .await
            .map_err(|rejet| Error::BadRequest(rejet.body_text()))?;

        Ok(Self::new(
            parametres.after,
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

/// Une page rendue par curseur, et de quoi demander la suivante.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[non_exhaustive]
pub struct CursorPage<T> {
    data: Vec<T>,
    meta: CursorMeta,
}

/// Description d'une page rendue par curseur.
///
/// Ni `total` ni `total_pages` : le `COUNT(*)` qu'ils exigeraient est précisément ce que
/// le curseur existe pour ne pas payer.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
struct CursorMeta {
    per_page: u64,
    next: Option<Uuid>,
}

impl<T> CursorPage<T> {
    /// Enveloppe `data`, `dernier` étant l'`id` du dernier élément rendu.
    ///
    /// `next` s'éteint dès que la page est plus courte que demandée : c'est la fin de la
    /// marche, et elle se lit sans compter la table. Le dernier `id` est passé plutôt que
    /// déduit — `T` est un DTO quelconque, dont cette crate ignore s'il porte un `id`.
    pub fn new(data: Vec<T>, cursor: &Cursor, dernier: Option<Uuid>) -> Self {
        let complete = data.len() as u64 == cursor.per_page();

        Self {
            meta: CursorMeta {
                per_page: cursor.per_page(),
                next: dernier.filter(|_| complete),
            },
            data,
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

    /// Interroge `/curseur` avec la chaîne de requête donnée et rend `(statut, body JSON)`.
    async fn curseur(query: &str) -> (StatusCode, Value) {
        async fn handler(cursor: Cursor) -> axum::Json<Value> {
            axum::Json(json!({
                "after": cursor.after().map(|id| id.to_string()),
                "per_page": cursor.per_page(),
            }))
        }

        let response = Router::new()
            .route("/curseur", get(handler))
            .oneshot(
                Request::builder()
                    .uri(format!("/curseur?{query}"))
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
    async fn the_first_page_needs_no_cursor() {
        let (status, body) = curseur("").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["after"], Value::Null, "sans `after`, on part du début");
        assert_eq!(body["per_page"], PAR_PAGE_PAR_DEFAUT);
    }

    #[tokio::test]
    async fn a_readable_cursor_is_carried_through() {
        let id = "01926b3e-0000-7000-8000-000000000000";
        let (status, body) = curseur(&format!("after={id}")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["after"], id);
    }

    #[tokio::test]
    async fn an_unreadable_cursor_answers_400() {
        let (status, body) = curseur("after=pas-un-uuid").await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "un curseur illisible se signale, il ne s'ignore pas : {body}"
        );
        assert_eq!(body["status"], 400);
    }

    #[tokio::test]
    async fn the_cursor_shares_the_page_size_bounds() {
        let (status, body) = curseur("per_page=5000").await;

        assert_eq!(status, StatusCode::OK, "le plafonnement est muet : {body}");
        assert_eq!(body["per_page"], PAR_PAGE_MAX);

        let (_, body) = curseur("per_page=0").await;
        assert_eq!(body["per_page"], 1, "une page vide n'aurait aucun sens");
    }

    /// Un identifiant lisible, dont seule l'unicité compte ici.
    fn identifiant(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    #[test]
    fn a_full_page_names_its_successor() {
        let cursor = Cursor::new(None, 3);
        let dernier = identifiant(3);

        let page = CursorPage::new(vec!["a", "b", "c"], &cursor, Some(dernier));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(rendu["meta"]["next"], dernier.to_string());
        assert_eq!(rendu["meta"]["per_page"], 3);
        assert_eq!(rendu["data"], json!(["a", "b", "c"]));
    }

    #[test]
    fn a_short_page_ends_the_walk() {
        let cursor = Cursor::new(None, 3);

        let page = CursorPage::new(vec!["a", "b"], &cursor, Some(identifiant(2)));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(
            rendu["meta"]["next"],
            Value::Null,
            "une page plus courte que demandée est la dernière : {rendu}"
        );
    }

    #[test]
    fn an_empty_page_ends_the_walk() {
        let cursor = Cursor::new(Some(identifiant(9)), 3);

        let page = CursorPage::<&str>::new(Vec::new(), &cursor, None);
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(rendu["meta"]["next"], Value::Null);
        assert_eq!(rendu["data"], json!([]));
    }

    #[test]
    fn the_cursor_page_never_counts_the_rows() {
        let cursor = Cursor::new(None, 2);
        let page = CursorPage::new(vec!["a", "b"], &cursor, Some(identifiant(2)));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        let meta = rendu["meta"].as_object().expect("meta est un objet");
        assert!(
            !meta.contains_key("total") && !meta.contains_key("total_pages"),
            "le curseur existe pour ne pas payer le COUNT(*) : {rendu}"
        );
    }
}
