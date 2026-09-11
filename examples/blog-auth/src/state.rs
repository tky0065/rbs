use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
    // <rbs:state_champs>
    pub mail: crate::modules::mail::Mailer,
    pub rate_limit: crate::modules::rate_limit::RateLimiter,
    pub flows: crate::auth::config::FlowConfig,
    // </rbs:state_champs>
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            core: CoreState::new(db, config),
            // <rbs:state_init>
            mail: crate::modules::mail::Mailer::from_config()?,
            rate_limit: crate::modules::rate_limit::RateLimiter::from_config()?,
            flows: crate::auth::config::FlowConfig::from_config()?,
            // </rbs:state_init>
        })
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
