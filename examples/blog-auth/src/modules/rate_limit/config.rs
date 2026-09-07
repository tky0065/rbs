use std::time::Duration;

use serde::Deserialize;

/// Section `[rate_limit]` de la configuration du projet.
///
/// Les défauts sont portés ici plutôt que par le noyau : ils sont lisibles et modifiables
/// à l'endroit même où la section est déclarée. `config/{env}.toml` et les variables
/// `RBS_RATE_LIMIT__*` les surchargent comme celles de toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Requêtes tolérées par fenêtre, pour une adresse cliente et hors règle de route.
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Durée de la fenêtre de comptage, en secondes.
    #[serde(default = "default_window")]
    pub window_secs: u64,
    /// Prend l'adresse cliente dans `X-Forwarded-For` plutôt que sur la connexion.
    ///
    /// À ne lever que derrière un proxy qui réécrit l'en-tête : exposée en direct, une
    /// API qui le croit laisse chaque client se choisir une identité par requête, et ne
    /// limite donc plus rien.
    #[serde(default)]
    pub trust_forwarded_for: bool,
    /// Limites propres à un préfixe de chemin, essayées dans l'ordre.
    #[serde(default)]
    pub routes: Vec<Route>,
}

/// Une limite plus stricte que la globale, sur les chemins que `path` préfixe.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    /// Préfixe de chemin auquel la règle s'applique : `/auth/login`.
    pub path: String,
    /// Requêtes tolérées par fenêtre sur ce préfixe.
    pub limit: u64,
    /// Durée de la fenêtre de comptage, en secondes.
    pub window_secs: u64,
}

/// La limite qui s'applique à une requête, et la portée sur laquelle elle compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule<'a> {
    /// Ce qui distingue ce compteur des autres pour une même adresse : le préfixe de la
    /// règle, ou la chaîne vide pour la limite globale.
    pub scope: &'a str,
    /// Requêtes tolérées par fenêtre.
    pub limit: u64,
    /// Durée de la fenêtre de comptage.
    pub window: Duration,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[rate_limit]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("rate_limit")
    }

    /// La règle qui s'applique à `path` : la première route qui le préfixe, la limite
    /// globale à défaut.
    ///
    /// L'ordre de la liste est celui du fichier, et il décide : deux préfixes qui se
    /// recouvrent — `/auth` et `/auth/login` — se rangent du plus précis au plus large.
    pub fn rule(&self, path: &str) -> Rule<'_> {
        self.routes
            .iter()
            .find(|route| path.starts_with(&route.path))
            .map_or(
                Rule {
                    scope: "",
                    limit: self.limit,
                    window: Duration::from_secs(self.window_secs),
                },
                |route| Rule {
                    scope: &route.path,
                    limit: route.limit,
                    window: Duration::from_secs(route.window_secs),
                },
            )
    }
}

// Dérivée, `Default` rendrait une limite nulle : toute requête serait refusée dès la
// première, ce qu'aucune section absente n'a jamais voulu dire.
impl Default for Config {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            window_secs: default_window(),
            trust_forwarded_for: false,
            routes: Vec::new(),
        }
    }
}

fn default_limit() -> u64 {
    120
}

fn default_window() -> u64 {
    60
}
