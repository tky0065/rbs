use serde::Deserialize;

/// Section `[jobs]` de la configuration du projet.
///
/// Les défauts sont portés ici plutôt que par le noyau : ils sont lisibles et modifiables
/// à l'endroit même où la section est déclarée. `config/{env}.toml` et les variables
/// `RBS_JOBS__*` les surchargent comme celles de toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Tentatives d'un job avant l'échec définitif.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: i32,
    /// Attente avant qu'une tentative ratée redevienne exécutable, en secondes.
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,
    /// Attente du worker quand la file est vide, en secondes.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[jobs]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("jobs")
    }
}

fn default_max_attempts() -> i32 {
    5
}

fn default_retry_delay() -> u64 {
    30
}

fn default_poll_interval() -> u64 {
    1
}
