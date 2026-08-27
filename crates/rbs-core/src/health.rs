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
pub struct Sante {
    /// Verdict d'ensemble.
    pub status: Statut,
    /// Détail par dépendance.
    pub checks: Controles,
}

/// Verdict d'ensemble d'un contrôle de santé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Statut {
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
pub struct Controles {
    /// État de la base de données.
    pub database: Controle,
}

/// Résultat d'un contrôle de dépendance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Controle {
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
    let (status, sante) = etat(state.core().db().ping().await);

    (status, axum::Json(sante)).into_response()
}

/// Traduit le résultat du ping en verdict.
///
/// Séparée du transport pour que la branche « base saine » reste couverte : sans base
/// démarrée, seule la branche 503 est atteignable par une requête réelle.
fn etat(ping: Result<(), DbErr>) -> (StatusCode, Sante) {
    match ping {
        Ok(()) => (
            StatusCode::OK,
            Sante {
                status: Statut::Ok,
                checks: Controles {
                    database: Controle::Ok,
                },
            },
        ),
        Err(erreur) => {
            // La cause part au journal et nulle part ailleurs : un contrôle de santé est
            // souvent exposé sans authentification.
            tracing::error!(erreur = %erreur, "base de données injoignable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Sante {
                    status: Statut::Unavailable,
                    checks: Controles {
                        database: Controle::Unreachable,
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
                secret: "un secret de test qui porte au moins trente-deux octets".to_owned(),
                access_ttl_secs: 900,
                refresh_ttl_secs: 2_592_000,
            },
        }
    }

    #[tokio::test]
    async fn une_base_indisponible_repond_503_pas_200() {
        // `DatabaseConnection::default()` est un pool déconnecté : `ping` y échoue sans
        // qu'aucune base n'ait à tourner.
        let etat = CoreState::new(DatabaseConnection::default(), config());

        let reponse = routes()
            .with_state(etat)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le routeur doit répondre");

        assert_eq!(reponse.status(), StatusCode::SERVICE_UNAVAILABLE);

        let octets = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let corps: Value = serde_json::from_slice(&octets).expect("corps JSON");
        assert_eq!(
            corps,
            json!({ "status": "unavailable", "checks": { "database": "unreachable" } })
        );
    }

    #[test]
    fn une_base_saine_donne_200_et_un_statut_ok() {
        let (status, sante) = etat(Ok(()));

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::to_value(sante).expect("sérialisable"),
            json!({ "status": "ok", "checks": { "database": "ok" } })
        );
    }

    #[test]
    fn une_base_injoignable_donne_503_et_nomme_le_controle_en_echec() {
        let (status, sante) = etat(Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "connexion refusée".to_owned(),
        ))));

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(sante.checks.database, Controle::Unreachable);
    }

    #[test]
    fn le_detail_de_l_erreur_base_ne_fuit_pas_dans_la_reponse() {
        let (_, sante) = etat(Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
            "postgres://alice:s3cr3t@localhost/app injoignable".to_owned(),
        ))));

        let rendu = serde_json::to_string(&sante).expect("sérialisable");

        assert!(
            !rendu.contains("s3cr3t") && !rendu.contains("injoignable"),
            "le détail de l'erreur ne doit pas atteindre le client : {rendu}"
        );
    }
}
