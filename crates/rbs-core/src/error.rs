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
    #[error("erreur base de données : {0}")]
    Database(#[from] DbErr),

    /// Toute autre défaillance inattendue.
    #[error("erreur interne : {0}")]
    Internal(#[from] anyhow::Error),
}

/// Alias de `Result` pointant sur [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

fn champs(errors: &ValidationErrors) -> BTreeMap<String, Vec<String>> {
    errors
        .field_errors()
        .into_iter()
        .map(|(champ, erreurs)| {
            let messages = erreurs
                .iter()
                .map(|erreur| {
                    erreur
                        .message
                        .clone()
                        .unwrap_or_else(|| erreur.code.clone())
                        .into_owned()
                })
                .collect();
            (champ.to_string(), messages)
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
                Some(champs(source)),
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
                    erreur = %self,
                    "erreur interne"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error",
                    Some("une erreur interne est survenue".to_string()),
                    None,
                )
            }
        };

        let corps = ProblemDetails {
            r#type: "about:blank",
            title: title.to_string(),
            status: status.as_u16(),
            detail,
            errors,
            request_id,
        };

        let mut response = (status, Json(corps)).into_response();
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

    async fn reponse(err: Error) -> (StatusCode, String, Value) {
        let res = err.into_response();
        let status = res.status();
        let content_type = res
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .expect("content-type absent")
            .to_str()
            .expect("content-type non ASCII")
            .to_string();
        let corps = to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("corps illisible");

        (
            status,
            content_type,
            serde_json::from_slice(&corps).expect("corps non JSON"),
        )
    }

    fn validation_errors() -> ValidationErrors {
        let mut errors = ValidationErrors::new();
        errors.add("email", ValidationError::new("format invalide"));
        errors
    }

    #[test]
    fn db_err_convertit_en_database() {
        let err: Error = DbErr::Custom("connexion refusée".into()).into();

        assert!(matches!(err, Error::Database(_)));
        assert!(err.to_string().contains("connexion refusée"));
    }

    #[test]
    fn anyhow_convertit_en_internal() {
        let err: Error = anyhow::anyhow!("le disque est plein").into();

        assert!(matches!(err, Error::Internal(_)));
        assert!(err.to_string().contains("le disque est plein"));
    }

    #[test]
    fn validation_errors_convertit_en_validation_en_preservant_les_champs() {
        let err: Error = validation_errors().into();

        let Error::Validation(errors) = err else {
            panic!("attendu Error::Validation");
        };
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn domain_conserve_son_statut_et_son_code() {
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
            panic!("attendu Error::Domain");
        };
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(code, "quota_depasse");
        assert_eq!(message, "le quota mensuel est atteint");
    }

    #[tokio::test]
    async fn validation_repond_422_avec_le_detail_des_champs() {
        let (status, content_type, corps) = reponse(validation_errors().into()).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(content_type, "application/problem+json");
        assert_eq!(corps["title"], "Validation failed");
        assert_eq!(corps["status"], 422);
        assert_eq!(corps["errors"]["email"][0], "format invalide");
    }

    #[tokio::test]
    async fn database_repond_500_generique_sans_le_message_de_la_source() {
        let err: Error = DbErr::Custom("connexion refusée sur 10.0.0.3:5432".into()).into();

        let (status, _, corps) = reponse(err).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(corps["title"], "Internal Server Error");
        let brut = corps.to_string();
        assert!(
            !brut.contains("connexion refusée"),
            "corps divulgué : {brut}"
        );
        assert!(!brut.contains("10.0.0.3"), "corps divulgué : {brut}");
    }

    #[tokio::test]
    async fn internal_repond_500_generique_sans_le_message_de_la_source() {
        let err: Error = anyhow::anyhow!("le secret JWT est absent").into();

        let (status, _, corps) = reponse(err).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !corps.to_string().contains("secret JWT"),
            "corps divulgué : {corps}"
        );
    }

    #[tokio::test]
    async fn not_found_nomme_la_ressource() {
        let (status, _, corps) = reponse(Error::NotFound("utilisateur")).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(corps["title"], "Not Found");
        assert_eq!(corps["detail"], "utilisateur introuvable");
    }

    #[tokio::test]
    async fn unauthorized_et_forbidden_portent_leur_statut() {
        let (status, _, _) = reponse(Error::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _, _) = reponse(Error::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn conflict_repond_409_avec_son_message() {
        let (status, _, corps) = reponse(Error::Conflict("cet email est déjà pris".into())).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(corps["detail"], "cet email est déjà pris");
    }

    #[tokio::test]
    async fn bad_request_repond_400_avec_sa_cause() {
        let (status, _, corps) =
            reponse(Error::BadRequest("EOF while parsing an object".into())).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(corps["title"], "Bad Request");
        assert_eq!(corps["detail"], "EOF while parsing an object");
    }

    #[tokio::test]
    async fn domain_porte_son_statut_son_code_et_son_message() {
        let err = Error::Domain {
            status: StatusCode::PAYMENT_REQUIRED,
            code: "quota_depasse",
            message: "le quota mensuel est atteint".into(),
        };

        let (status, _, corps) = reponse(err).await;

        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(corps["status"], 402);
        assert_eq!(corps["title"], "quota_depasse");
        assert_eq!(corps["detail"], "le quota mensuel est atteint");
    }

    #[tokio::test]
    async fn le_corps_porte_le_request_id_du_scope() {
        let corps = request_id::scope("01JQ3F8K2P".to_string(), async {
            reponse(Error::Unauthorized).await.2
        })
        .await;

        assert_eq!(corps["request_id"], "01JQ3F8K2P");
    }

    #[tokio::test]
    async fn le_champ_request_id_est_omis_hors_requete() {
        let (_, _, corps) = reponse(Error::Unauthorized).await;

        assert_eq!(corps.get("request_id"), None);
        assert_eq!(corps["type"], "about:blank");
    }

    #[test]
    fn result_est_l_alias_du_type_error() {
        fn echoue() -> Result<()> {
            Err(Error::Unauthorized)
        }

        assert!(matches!(echoue(), Err(Error::Unauthorized)));
    }
}
