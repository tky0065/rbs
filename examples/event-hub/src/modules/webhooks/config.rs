use serde::Deserialize;

/// Section `[webhooks]` de la configuration du projet.
///
/// Le défaut est porté ici plutôt que par le noyau : il est lisible et modifiable à
/// l'endroit même où la section est déclarée. `config/{env}.toml` et la variable
/// `RBS_WEBHOOKS__TIMEOUT_SECS` le surchargent comme pour toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Temps laissé au receveur pour répondre, en secondes.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[webhooks]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("webhooks")
    }
}

fn default_timeout() -> u64 {
    10
}
