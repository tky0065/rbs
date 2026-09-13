use serde::Deserialize;

/// Section `[scheduler]` de la configuration du projet.
///
/// Le défaut est porté ici plutôt que par le noyau : il est lisible et modifiable à
/// l'endroit même où la section est déclarée. `config/{env}.toml` et la variable
/// `RBS_SCHEDULER__POLL_INTERVAL_SECS` le surchargent comme pour toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Attente entre deux examens du calendrier, en secondes.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[scheduler]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("scheduler")
    }
}

fn default_poll_interval() -> u64 {
    30
}
