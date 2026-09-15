//! État partagé du runtime.
//!
//! La spec range `state.rs` du côté généré : le noyau ne possède donc pas l'`AppState`
//! du projet, qui doit rester libre d'accueillir un client Redis ou un service mail. Il
//! porte le couple pool + configuration, et le trait par lequel ses propres handlers
//! l'atteignent quel que soit l'état qui l'enveloppe.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::shutdown::Shutdown;

/// Pool, configuration et signal d'arrêt, partagés par toutes les requêtes.
///
/// Clonable à coût nul : `DatabaseConnection` clone un `Arc` interne, la configuration
/// n'est jamais recopiée, et tous les clones portent le même signal.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CoreState {
    // Champs privés : le noyau doit pouvoir en gagner un sans casser les projets qui
    // l'ont composé. Un accès direct figerait sa disposition interne.
    db: DatabaseConnection,
    config: Arc<Config>,
    shutdown: Shutdown,
}

impl CoreState {
    /// Assemble l'état à partir d'un pool ouvert et de la configuration chargée.
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        // L'`Arc` est posé ici, et non exigé de l'appelant : le partage est un détail du
        // noyau, pas une contrainte imposée au projet.
        Self {
            db,
            config: Arc::new(config),
            shutdown: Shutdown::new(),
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

    /// Signal d'arrêt du processus, par lequel les tâches de fond apprennent qu'il faut
    /// rendre la main et par lequel `main` les attend.
    pub fn shutdown(&self) -> &Shutdown {
        &self.shutdown
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

/// État applicatif donnant accès à la configuration d'authentification.
///
/// La méthode a un corps par défaut, mais le trait n'est **pas** implémenté pour tout
/// [`HasCoreState`] : une implémentation générale interdirait à un projet de tirer son
/// secret d'ailleurs, d'un gestionnaire de secrets par exemple. Le projet généré écrit
/// `impl HasAuth for AppState {}`, une ligne.
#[cfg(feature = "auth")]
pub trait HasAuth: HasCoreState {
    /// Configuration d'authentification portée par cet état.
    fn auth(&self) -> &crate::config::AuthConfig {
        &self.core().config().auth
    }

    /// Dernier mot du projet sur un jeton dont la signature est bonne.
    ///
    /// Le noyau ne connaît ni la table des comptes ni ce qu'une révocation y écrit : il
    /// vérifie la signature, puis demande. Le défaut accepte tout, et c'est ce qu'un
    /// projet sans révocation obtient sans rien écrire.
    fn accept(
        &self,
        claims: &crate::jwt::Claims,
    ) -> impl std::future::Future<Output = Result<(), crate::Error>> + Send {
        let _ = claims;
        async { Ok(()) }
    }

    /// Ce que l'extracteur [`Identity`](crate::Identity) appelle, les extensions de la
    /// requête à portée : un projet qui relit le compte pour juger le jeton peut l'y
    /// laisser, et l'extracteur suivant le reprendre. Le défaut s'en remet à `accept`.
    fn accept_in(
        &self,
        claims: &crate::jwt::Claims,
        extensions: &mut axum::http::Extensions,
    ) -> impl std::future::Future<Output = Result<(), crate::Error>> + Send {
        let _ = extensions;
        self.accept(claims)
    }
}

#[cfg(feature = "auth")]
impl HasAuth for CoreState {}

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
                timeout_secs: 30,
                shutdown_timeout_secs: 30,
                lang: crate::lang::Lang::Fr,
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
    fn state(salutation: &'static str) -> AppState {
        AppState {
            core: CoreState::new(DatabaseConnection::default(), config()),
            salutation,
        }
    }

    #[tokio::test]
    async fn a_handler_extracts_the_project_state_and_answers() {
        async fn handler(State(state): State<AppState>) -> String {
            format!("{} {}", state.salutation, state.core().config().env)
        }

        let app = Router::new()
            .route("/", get(handler))
            .with_state(state("bonjour"));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("le router doit répondre");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        assert_eq!(&body[..], b"bonjour development");
    }

    #[test]
    fn the_state_clones_without_copying_the_configuration() {
        let state = state("bonjour");
        let clone = state.clone();

        assert!(
            Arc::ptr_eq(&state.core.config, &clone.core.config),
            "le clone doit partager la configuration, pas la recopier"
        );
    }

    #[test]
    fn the_shutdown_signal_is_shared_by_the_clones_of_the_state() {
        let state = state("bonjour");
        let clone = state.clone();

        state.core().shutdown().request();

        assert!(
            clone.core().shutdown().is_requested(),
            "le clone doit porter le même signal, pas un signal à lui"
        );
    }

    #[test]
    fn core_state_serves_as_a_state_on_its_own() {
        let core = state("bonjour").core;

        assert_eq!(core.core().config().env, core.config().env);
    }
}
