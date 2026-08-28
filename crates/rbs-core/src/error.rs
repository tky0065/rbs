//! Type d'erreur unique du runtime.
//!
//! Toute la chaîne d'une feature — repository, service, controller — retourne
//! [`Result<T>`]. La conversion en réponse HTTP est portée par l'implémentation
//! `IntoResponse` de [`Error`].

use crate::openapi::ProblemDetails;
use crate::request_id;
use axum::Json;
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, HeaderValue};
use axum::response::{IntoResponse, Response};
use sea_orm::DbErr;
use std::collections::BTreeMap;
use validator::ValidationErrors;

/// Erreur unique du runtime, convertie en réponse `application/problem+json`.
///
/// Les messages ci-dessous s'adressent au journal serveur : `Database` et `Internal` y
/// portent leur source, que la réponse HTTP ne divulgue jamais au client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Ressource absente. Porte le nom de la ressource, pas un message rédigé.
    #[error("{0} introuvable")]
    NotFound(&'static str),

    /// Requête mal formée, indépendamment de toute règle métier.
    ///
    /// Le corps n'a pas pu être lu : syntaxe invalide, type incompatible, en-tête
    /// manquant. Une entrée lisible mais non conforme relève de [`Error::Validation`].
    #[error("requête invalide : {0}")]
    BadRequest(String),

    /// Échec de validation d'une entrée, détaillé champ par champ.
    #[error("validation échouée")]
    Validation(#[from] ValidationErrors),

    /// Authentification absente ou invalide.
    #[error("authentification requise")]
    Unauthorized,

    /// Authentifié, mais sans droit sur la ressource.
    #[error("accès interdit")]
    Forbidden,

    /// Conflit avec l'état courant de la ressource.
    #[error("conflit : {0}")]
    Conflict(String),

    /// Erreur métier propre au projet, qui choisit son statut et son code.
    ///
    /// Cette variante évite aux projets générés d'empiler leur propre hiérarchie
    /// d'erreurs par-dessus celle-ci.
    #[error("{code} : {message}")]
    Domain {
        /// Statut HTTP de la réponse.
        status: StatusCode,
        /// Code stable identifiant l'erreur métier.
        code: &'static str,
        /// Message destiné au client.
        message: String,
    },

    /// Échec d'accès à la base de données.
    #[error("error base de données : {0}")]
    Database(#[from] DbErr),

    /// Toute autre défaillance inattendue.
    #[error("error interne : {0}")]
    Internal(#[from] anyhow::Error),
}

/// Alias de `Result` pointant sur [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

fn fields(errors: &ValidationErrors) -> BTreeMap<String, Vec<String>> {
    errors
        .field_errors()
        .into_iter()
        .map(|(field, erreurs)| {
            let messages = erreurs
                .iter()
                .map(|error| {
                    error
                        .message
                        .clone()
                        .unwrap_or_else(|| error.code.clone())
                        .into_owned()
                })
                .collect();
            (field.to_string(), messages)
        })
        .collect()
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let request_id = request_id::current();

        let (status, title, detail, errors) = match &self {
            Error::NotFound(ressource) => (
                StatusCode::NOT_FOUND,
                "Not Found",
                Some(format!("{ressource} introuvable")),
                None,
            ),
            Error::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                "Bad Request",
                Some(message.clone()),
                None,
            ),
            Error::Validation(source) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Validation failed",
                None,
                Some(fields(source)),
            ),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None, None),
            Error::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", None, None),
            Error::Conflict(message) => (
                StatusCode::CONFLICT,
                "Conflict",
                Some(message.clone()),
                None,
            ),
            Error::Domain {
                status,
                code,
                message,
            } => (*status, *code, Some(message.clone()), None),
            Error::Database(_) | Error::Internal(_) => {
                // La source part au journal et nulle part ailleurs : le client n'obtient
                // que le request_id, qui suffit à retrouver cette ligne.
                tracing::error!(
                    request_id = request_id.as_deref().unwrap_or("-"),
                    error = %self,
                    "error interne"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    Some("une error interne est survenue".to_string()),
                    None,
                )
            }
        };

        let body = ProblemDetails {
            r#type: "about:blank",
            title: title.to_string(),
            status: status.as_u16(),
            detail,
            errors,
            request_id,
        };

        let mut response = (status, Json(body)).into_response();
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_id;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;
    use serde_json::Value;
    use validator::ValidationError;

    async fn response(err: Error) -> (StatusCode, String, Value) {
        let res = err.into_response();
        let status = res.status();
        let content_type = res
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content-type absent")
            .to_str()
            .expect("content-type non ASCII")
            .to_string();
        let body = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("body illisible");

        (
            status,
            content_type,
            serde_json::from_slice(&body).expect("body non JSON"),
        )
    }

    fn validation_errors() -> ValidationErrors {
        let mut errors = ValidationErrors::new();
        errors.add("email", ValidationError::new("format invalide"));
        errors
    }

    #[test]
    fn db_err_converts_to_database() {
        let err: Error = DbErr::Custom("connexion refusée".into()).into();

        assert!(matches!(err, Error::Database(_)));
        assert!(err.to_string().contains("connexion refusée"));
    }

    #[test]
    fn anyhow_converts_to_internal() {
        let err: Error = anyhow::anyhow!("le disque est plein").into();

        assert!(matches!(err, Error::Internal(_)));
        assert!(err.to_string().contains("le disque est plein"));
    }

    #[test]
    fn validation_errors_converts_to_validation_keeping_the_fields() {
        let err: Error = validation_errors().into();

        let Error::Validation(errors) = err else {
            panic!("expected Error::Validation");
        };
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn domain_keeps_its_status_and_code() {
        let err = Error::Domain {
            status: StatusCode::PAYMENT_REQUIRED,
            code: "quota_depasse",
            message: "le quota mensuel est atteint".into(),
        };

        let Error::Domain {
            status,
            code,
            message,
        } = err
        else {
            panic!("expected Error::Domain");
        };
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(code, "quota_depasse");
        assert_eq!(message, "le quota mensuel est atteint");
    }

    #[tokio::test]
    async fn validation_answers_422_with_the_field_detail() {
        let (status, content_type, body) = response(validation_errors().into()).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(content_type, "application/problem+json");
        assert_eq!(body["title"], "Validation failed");
        assert_eq!(body["status"], 422);
        assert_eq!(body["errors"]["email"][0], "format invalide");
    }

    #[tokio::test]
    async fn database_answers_a_generic_500_without_the_source_message() {
        let err: Error = DbErr::Custom("connexion refusée sur 10.0.0.3:5432".into()).into();

        let (status, _, body) = response(err).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["title"], "Internal Server Error");
        let brut = body.to_string();
        assert!(
            !brut.contains("connexion refusée"),
            "body divulgué : {brut}"
        );
        assert!(!brut.contains("10.0.0.3"), "body divulgué : {brut}");
    }

    #[tokio::test]
    async fn internal_answers_a_generic_500_without_the_source_message() {
        let err: Error = anyhow::anyhow!("le secret JWT est absent").into();

        let (status, _, body) = response(err).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !body.to_string().contains("secret JWT"),
            "body divulgué : {body}"
        );
    }

    #[tokio::test]
    async fn not_found_names_the_resource() {
        let (status, _, body) = response(Error::NotFound("utilisateur")).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["title"], "Not Found");
        assert_eq!(body["detail"], "utilisateur introuvable");
    }

    #[tokio::test]
    async fn unauthorized_and_forbidden_carry_their_status() {
        let (status, _, _) = response(Error::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _, _) = response(Error::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn conflict_answers_409_with_its_message() {
        let (status, _, body) = response(Error::Conflict("cet email est déjà pris".into())).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["detail"], "cet email est déjà pris");
    }

    #[tokio::test]
    async fn bad_request_answers_400_with_its_cause() {
        let (status, _, body) =
            response(Error::BadRequest("EOF while parsing an object".into())).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["title"], "Bad Request");
        assert_eq!(body["detail"], "EOF while parsing an object");
    }

    #[tokio::test]
    async fn domain_carries_its_status_code_and_message() {
        let err = Error::Domain {
            status: StatusCode::PAYMENT_REQUIRED,
            code: "quota_depasse",
            message: "le quota mensuel est atteint".into(),
        };

        let (status, _, body) = response(err).await;

        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["status"], 402);
        assert_eq!(body["title"], "quota_depasse");
        assert_eq!(body["detail"], "le quota mensuel est atteint");
    }

    #[tokio::test]
    async fn the_body_carries_the_request_id_of_the_scope() {
        let body = request_id::scope("01JQ3F8K2P".to_string(), async {
            response(Error::Unauthorized).await.2
        })
        .await;

        assert_eq!(body["request_id"], "01JQ3F8K2P");
    }

    #[tokio::test]
    async fn the_request_id_field_is_omitted_outside_a_request() {
        let (_, _, body) = response(Error::Unauthorized).await;

        assert_eq!(body.get("request_id"), None);
        assert_eq!(body["type"], "about:blank");
    }

    #[test]
    fn result_is_the_alias_of_the_error_type() {
        fn failing() -> Result<()> {
            Err(Error::Unauthorized)
        }

        assert!(matches!(failing(), Err(Error::Unauthorized)));
    }
}
