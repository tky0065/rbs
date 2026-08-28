use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
    // <rbs:state_champs>
    pub cache: crate::cache::Cache,
    // Le transport n'a pas encore de lecteur : ce sont les handlers du projet qui en seront.
    #[allow(dead_code)]
    pub mail: crate::mail::Mailer,
    #[allow(dead_code)]
    pub storage: std::sync::Arc<dyn crate::storage::Storage>,
    // </rbs:state_champs>
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            core: CoreState::new(db, config),
            // <rbs:state_init>
            cache: crate::cache::Cache::depuis_config()?,
            mail: crate::mail::Mailer::depuis_config()?,
            storage: crate::storage::depuis_config()?,
            // </rbs:state_init>
        })
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
