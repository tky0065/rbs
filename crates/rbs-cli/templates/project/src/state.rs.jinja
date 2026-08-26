use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        Self {
            core: CoreState::new(db, config),
        }
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
