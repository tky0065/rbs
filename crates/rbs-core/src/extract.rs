//! Extracteurs de requête du runtime.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::Error;

/// Corps JSON désérialisé **puis** validé.
///
/// Un controller qui l'extrait reçoit un DTO déjà conforme à ses annotations
/// `validator` : il n'a plus à s'en soucier, et l'échec est rendu au client en
/// `application/problem+json` sans passe-plat.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extraire d'abord : un corps illisible ne peut pas être validé.
        let Json(recu) = Json::<T>::from_request(request, state)
            .await
            .map_err(corps_illisible)?;

        recu.validate()?;

        Ok(Self(recu))
    }
}

/// Traduit un rejet d'extraction en [`Error::BadRequest`].
///
/// Tout rejet devient 400, là où axum distingue 400, 415 et 422. La frontière est alors
/// lisible pour qui débogue une API générée : 400 « je n'ai pas pu lire ton corps »,
/// 422 « je l'ai lu, il ne respecte pas les règles ». Seul `body_text()` est repris, pour
/// garder `JsonRejection` hors de la signature d'[`Error`] : une mise à jour d'axum ne
/// doit pas rompre le noyau.
fn corps_illisible(rejet: JsonRejection) -> Error {
    Error::BadRequest(rejet.body_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use axum::routing::post;
    use serde::Deserialize;
    use serde_json::Value;
    use tower::ServiceExt;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct Inscription {
        #[validate(email(message = "adresse électronique invalide"))]
        email: String,
        #[validate(range(min = 18, message = "âge minimum : 18 ans"))]
        age: u8,
    }

    /// Poste `corps` sur un handler qui exige un [`Inscription`] validé.
    async fn poster(corps: &'static str, content_type: Option<&str>) -> (StatusCode, Value) {
        async fn handler(ValidatedJson(recu): ValidatedJson<Inscription>) -> String {
            recu.email
        }

        let mut requete = Request::builder().method("POST").uri("/");
        if let Some(content_type) = content_type {
            requete = requete.header(header::CONTENT_TYPE, content_type);
        }

        let reponse = Router::new()
            .route("/", post(handler))
            .oneshot(requete.body(Body::from(corps)).expect("requête valide"))
            .await
            .expect("le routeur doit répondre");

        let status = reponse.status();
        let octets = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let corps = serde_json::from_slice(&octets)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&octets).into_owned()));

        (status, corps)
    }

    #[tokio::test]
    async fn un_corps_valide_est_extrait_tel_quel() {
        let (status, corps) = poster(
            r#"{"email":"alice@exemple.fr","age":30}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(corps, Value::String("alice@exemple.fr".to_owned()));
    }

    #[tokio::test]
    async fn un_corps_invalide_repond_422_avec_le_detail_par_champ() {
        let (status, corps) = poster(
            r#"{"email":"pas-une-adresse","age":12}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(corps["status"], 422);
        assert_eq!(corps["errors"]["email"][0], "adresse électronique invalide");
        assert_eq!(corps["errors"]["age"][0], "âge minimum : 18 ans");
    }

    #[tokio::test]
    async fn un_json_malforme_repond_400_pas_500() {
        let (status, corps) =
            poster(r#"{"email":"alice@exemple.fr","#, Some("application/json")).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {corps}");
        assert_eq!(corps["status"], 400);
        assert!(
            corps["detail"].is_string(),
            "la cause doit rester lisible : {corps}"
        );
    }

    #[tokio::test]
    async fn un_content_type_absent_repond_400_pas_500() {
        let (status, corps) = poster(r#"{"email":"alice@exemple.fr","age":30}"#, None).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {corps}");
        assert_eq!(corps["status"], 400);
    }
}
