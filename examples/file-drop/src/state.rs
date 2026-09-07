use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
    // <rbs:state_champs>
    pub cache: crate::modules::cache::Cache,
    pub mail: crate::modules::mail::Mailer,
    pub storage: std::sync::Arc<dyn crate::modules::storage::Storage>,
    // </rbs:state_champs>
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            core: CoreState::new(db, config),
            // <rbs:state_init>
            cache: crate::modules::cache::Cache::from_config()?,
            mail: crate::modules::mail::Mailer::from_config()?,
            storage: crate::modules::storage::from_config()?,
            // </rbs:state_init>
        })
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
