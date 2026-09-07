use serde::Deserialize;

/// Section `[observability]` de la configuration du projet.
///
/// Le port seul y figure. Le collecteur de traces et le nom du service viennent de
/// `OTEL_EXPORTER_OTLP_ENDPOINT` et `OTEL_SERVICE_NAME` : `rbs_core::logs::init()` les
/// lit à la première ligne du `main`, avant qu'aucune configuration ne soit chargée, et
/// ce sont les noms que tout collecteur connaît déjà.
///
/// Les défauts sont portés ici plutôt que par le noyau : ils sont lisibles et modifiables
/// à l'endroit même où la section est déclarée. `config/{env}.toml` et les variables
/// `RBS_OBSERVABILITY__*` les surchargent comme celles de toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Port du listener qui sert `/metrics`, distinct de `server.port`.
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[observability]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("observability")
    }
}

// Dérivée, `Default` rendrait le port 0 : le listener prendrait un port libre au hasard,
// que rien ne saurait plus interroger.
impl Default for Config {
    fn default() -> Self {
        Self {
            metrics_port: default_metrics_port(),
        }
    }
}

fn default_metrics_port() -> u16 {
    9090
}
