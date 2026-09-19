use serde::Deserialize;

/// Section `[cors]` de la configuration du projet.
///
/// Les défauts sont portés ici plutôt que par le noyau : ils sont lisibles et modifiables
/// à l'endroit même où la section est déclarée. `config/{env}.toml` et les variables
/// `RBS_CORS__*` les surchargent comme celles de toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Origines autorisées, telles qu'un navigateur les envoie : `https://app.exemple.fr`.
    ///
    /// Vide par défaut : aucune requête d'origine croisée n'est autorisée tant que le
    /// projet n'a pas nommé ses clients. La valeur `"*"` ouvre l'API à toutes les
    /// origines, et ne se combine pas avec `credentials`.
    #[serde(default)]
    pub origins: Vec<String>,
    /// Méthodes autorisées sur une requête d'origine croisée.
    #[serde(default = "default_methods")]
    pub methods: Vec<String>,
    /// En-têtes que le client peut envoyer.
    #[serde(default = "default_headers")]
    pub headers: Vec<String>,
    /// Autorise l'envoi des cookies et de l'en-tête `Authorization`.
    #[serde(default)]
    pub credentials: bool,
    /// Durée pendant laquelle le navigateur peut mettre le préflight en cache.
    #[serde(default = "default_max_age")]
    pub max_age_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[cors]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("cors")
    }
}

// Dérivée, `Default` rendrait des listes vides là où serde met les valeurs ci-dessous :
// une section absente et une section vide ne donneraient pas la même couche.
impl Default for Config {
    fn default() -> Self {
        Self {
            origins: Vec::new(),
            methods: default_methods(),
            headers: default_headers(),
            credentials: false,
            max_age_secs: default_max_age(),
        }
    }
}

fn default_methods() -> Vec<String> {
    ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
        .iter()
        .map(|methode| (*methode).to_string())
        .collect()
}

fn default_headers() -> Vec<String> {
    ["authorization", "content-type"]
        .iter()
        .map(|entete| (*entete).to_string())
        .collect()
}

fn default_max_age() -> u64 {
    3600
}
