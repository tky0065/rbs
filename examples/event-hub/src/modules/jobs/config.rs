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
    /// Attente avant qu'une tentative ratée redevienne exécutable, en secondes — doublée
    /// à chaque tentative, jusqu'à `retry_max_delay_secs`.
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,
    /// Plafond du délai de reprise, en secondes.
    #[serde(default = "default_retry_max_delay")]
    pub retry_max_delay_secs: u64,
    /// Attente du worker quand la file est vide, en secondes.
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// Durée au-delà de laquelle une réservation sans suite est tenue pour abandonnée,
    /// en secondes. À régler au-dessus du plus long job : un job encore en cours au-delà
    /// du bail est rendu à la file, et rejoué.
    #[serde(default = "default_lease")]
    pub lease_secs: u64,
    /// Jobs exécutés de front par ce worker. Zéro vaut un.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
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

fn default_retry_max_delay() -> u64 {
    3600
}

fn default_concurrency() -> usize {
    4
}

fn default_poll_interval() -> u64 {
    1
}

fn default_lease() -> u64 {
    300
}
