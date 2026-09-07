use serde::Deserialize;

/// Section `[cache]` de la configuration du projet.
///
/// Les défauts sont portés ici plutôt que par le noyau : ils sont lisibles et modifiables
/// à l'endroit même où la section est déclarée. `config/{env}.toml` et les variables
/// `RBS_CACHE__*` les surchargent comme celles de toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// URL du serveur, mot de passe compris : `redis://:secret@hote:6379/0`.
    #[serde(default = "default_url")]
    pub url: String,
    /// Durée de vie que `Cache::set` applique, en secondes. Zéro : aucune expiration.
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[cache]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("cache")
    }
}

fn default_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_ttl() -> u64 {
    300
}
