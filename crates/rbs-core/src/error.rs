//! Type d'erreur unique du runtime.
//!
//! Toute la chaîne d'une feature — repository, service, controller — retourne
//! [`Result<T>`]. La conversion en réponse HTTP est portée par l'implémentation
//! `IntoResponse` de [`Error`].

use axum::http::StatusCode;
use sea_orm::DbErr;
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

#[cfg(test)]
mod tests {
    use super::*;
    use validator::ValidationError;

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

    #[test]
    fn result_est_l_alias_du_type_error() {
        fn echoue() -> Result<()> {
            Err(Error::Unauthorized)
        }

        assert!(matches!(echoue(), Err(Error::Unauthorized)));
    }
}
