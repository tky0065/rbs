//! État partagé du runtime.
//!
//! La spec range `state.rs` du côté généré : le noyau ne possède donc pas l'`AppState`
//! du projet, qui doit rester libre d'accueillir un client Redis ou un service mail. Il
//! porte le couple pool + configuration, et le trait par lequel ses propres handlers
//! l'atteignent quel que soit l'état qui l'enveloppe.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;

/// Pool et configuration, partagés par toutes les requêtes.
///
/// Clonable à coût nul : `DatabaseConnection` clone un `Arc` interne, et la
/// configuration n'est jamais recopiée.
#[derive(Debug, Clone)]
pub struct CoreState {
    // Champs privés : le noyau doit pouvoir en gagner un sans casser les projets qui
    // l'ont composé. Un accès direct figerait sa disposition interne.
    db: DatabaseConnection,
    config: Arc<Config>,
}

impl CoreState {
    /// Assemble l'état à partir d'un pool ouvert et de la configuration chargée.
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        // L'`Arc` est posé ici, et non exigé de l'appelant : le partage est un détail du
        // noyau, pas une contrainte imposée au projet.
        Self {
            db,
            config: Arc::new(config),
        }
    }

    /// Pool de connexions à la base.
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Configuration de l'application.
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// État applicatif donnant accès au [`CoreState`] du runtime.
///
/// Les bornes sont celles qu'Axum exige d'un état partagé ; les porter ici évite de les
/// répéter sur chaque handler du noyau.
pub trait HasCoreState: Clone + Send + Sync + 'static {
    /// État du runtime porté par cet état applicatif.
    fn core(&self) -> &CoreState;
}

// Un projet sans état propre monte les handlers du noyau directement sur `CoreState`.
impl HasCoreState for CoreState {
    fn core(&self) -> &CoreState {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, DocsConfig, ServerConfig};
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::extract::State;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    use tower::ServiceExt;

    /// Ce que `state.rs` générera dans le projet : `CoreState` composé, plus ses champs.
    #[derive(Clone)]
    struct AppState {
        core: CoreState,
        salutation: &'static str,
    }

    impl HasCoreState for AppState {
        fn core(&self) -> &CoreState {
            &self.core
        }
    }

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

    /// `DatabaseConnection::default()` est un état déconnecté : aucun test n'a besoin
    /// d'une base démarrée pour manipuler l'état.
    fn etat(salutation: &'static str) -> AppState {
        AppState {
            core: CoreState::new(DatabaseConnection::default(), config()),
            salutation,
        }
    }

    #[tokio::test]
    async fn un_handler_extrait_l_etat_du_projet_et_repond() {
        async fn handler(State(state): State<AppState>) -> String {
            format!("{} {}", state.salutation, state.core().config().env)
        }

        let app = Router::new()
            .route("/", get(handler))
            .with_state(etat("bonjour"));

        let reponse = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("le routeur doit répondre");

        assert_eq!(reponse.status(), StatusCode::OK);
        let corps = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        assert_eq!(&corps[..], b"bonjour development");
    }

    #[test]
    fn l_etat_se_clone_sans_copier_la_configuration() {
        let etat = etat("bonjour");
        let clone = etat.clone();

        assert!(
            Arc::ptr_eq(&etat.core.config, &clone.core.config),
            "le clone doit partager la configuration, pas la recopier"
        );
    }

    #[test]
    fn core_state_sert_d_etat_a_lui_seul() {
        let core = etat("bonjour").core;

        assert_eq!(core.core().config().env, core.config().env);
    }
}
