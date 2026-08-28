use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
    // <rbs:state_champs>
    // Le transport n'a pas encore de lecteur : ce sont les handlers du projet qui en seront.
    #[allow(dead_code)]
    pub mail: crate::mail::Mailer,
    // </rbs:state_champs>
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {
        Ok(Self {
            core: CoreState::new(db, config),
            // <rbs:state_init>
            mail: crate::mail::Mailer::from_config()?,
            // </rbs:state_init>
        })
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
