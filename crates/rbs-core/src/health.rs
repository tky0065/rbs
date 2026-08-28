//! Route de santé de l'application.
//!
//! Le noyau porte le contrôle ; c'est le projet généré qui décide où le monter et ce
//! qu'il y ajoute. Le handler est générique sur [`HasCoreState`], donc utilisable aussi
//! bien avec l'`AppState` d'un projet qu'avec un [`CoreState`](crate::state::CoreState)
//! nu.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use sea_orm::DbErr;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::HasCoreState;

/// Santé de l'application et de ses dépendances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct Health {
    /// Verdict d'ensemble.
    pub status: Status,
    /// Détail par dépendance.
    pub checks: Checks,
}

/// Verdict d'ensemble d'un contrôle de santé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Toutes les dépendances répondent.
    Ok,
    /// Au moins une dépendance ne répond pas.
    Unavailable,
}

/// État de chaque dépendance contrôlée.
///
/// Les contrôles sont imbriqués plutôt qu'à plat pour qu'une dépendance ajoutée plus
/// tard — cache, file, stockage — n'oblige pas à toucher la racine du corps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
pub struct Checks {
    /// État de la base de données.
    pub database: Check,
}

/// Résultat d'un contrôle de dépendance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Check {
    /// La dépendance répond.
    Ok,
    /// La dépendance ne répond pas.
    Unreachable,
}

/// Monte `GET /health` sur un routeur.
pub fn routes<S>() -> Router<S>
where
    S: HasCoreState,
{
    Router::new().route("/health", get(handler::<S>))
}

/// Rend la santé de l'application, `503` dès qu'une dépendance manque à l'appel.
pub async fn handler<S>(State(state): State<S>) -> Response
where
    S: HasCoreState,
{
    let (status, health) = verdict(state.core().db().ping().await);

    (status, axum::Json(health)).into_response()
}

/// Traduit le résultat du ping en verdict.
///
/// Séparée du transport pour que la branche « base saine » reste couverte : sans base
/// démarrée, seule la branche 503 est atteignable par une requête réelle.
fn verdict(ping: Result<(), DbErr>) -> (StatusCode, Health) {
    match ping {
        Ok(()) => (
            StatusCode::OK,
            Health {
                status: Status::Ok,
                checks: Checks {
                    database: Check::Ok,
                },
            },
        ),
        Err(error) => {
            // La cause part au journal et nulle part ailleurs : un contrôle de santé est
            // souvent exposé sans authentification.
            tracing::error!(error = %error, "base de données injoignable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Health {
                    status: Status::Unavailable,
                    checks: Checks {
                        database: Check::Unreachable,
                    },
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, DocsConfig, ServerConfig};
    use crate::state::CoreState;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use sea_orm::DatabaseConnection;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    fn config() -> Config {
        Config {
            env: "development".to_owned(),
            server: ServerConfig {
                host: "127.0.0.1".to_owned(),
                port: 8080,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/app".to_owned(),
                max_connections: 10,
                min_connections: 0,
                connect_timeout_secs: 5,
                acquire_timeout_secs: 5,
                idle_timeout_secs: 600,
                max_lifetime_secs: 1800,
            },
            docs: DocsConfig {
                swagger_ui: true,
                openapi_json: true,
            },
            #[cfg(feature = "auth")]
            auth: crate::config::AuthConfig {
                secret: "un secret de test qui porte au moins trente-deux bytes".to_owned(),
                access_ttl_secs: 900,
                refresh_ttl_secs: 2_592_000,
            },
        }
    }

    #[tokio::test]
    async fn an_unavailable_database_answers_503_not_200() {
        // `DatabaseConnection::default()` est un pool déconnecté : `ping` y échoue sans
        // qu'aucune base n'ait à tourner.
        let verdict = CoreState::new(DatabaseConnection::default(), config());

        let response = routes()
            .with_state(verdict)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body lisible");
        let body: Value = serde_json::from_slice(&bytes).expect("body JSON");
        assert_eq!(
            body,
            json!({ "status": "unavailable", "checks": { "database": "unreachable" } })
        );
    }

    #[test]
    fn a_healthy_database_gives_200_and_an_ok_status() {
        let (status, health) = verdict(Ok(()));

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::to_value(health).expect("sérialisable"),
            json!({ "status": "ok", "checks": { "database": "ok" } })
        );
    }

    #[test]
    fn an_unreachable_database_gives_503_and_names_the_failed_check() {
        let (status, health) = verdict(Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "connexion refusée".to_owned(),
        ))));

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(health.checks.database, Check::Unreachable);
    }

    #[test]
    fn the_database_error_detail_does_not_leak_into_the_response() {
        let (_, health) = verdict(Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "postgres://alice:s3cr3t@localhost/app injoignable".to_owned(),
        ))));

        let rendered = serde_json::to_string(&health).expect("sérialisable");

        assert!(
            !rendered.contains("s3cr3t") && !rendered.contains("injoignable"),
            "le détail de l'error ne doit pas atteindre le client : {rendered}"
        );
    }
}
