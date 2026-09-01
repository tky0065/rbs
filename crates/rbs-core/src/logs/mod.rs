//! Journalisation du runtime : deux formateurs et la bascule qui les choisit.

#[cfg(test)]
pub(crate) mod aide;
mod json;
#[cfg(feature = "observability")]
mod otel;
mod pretty;

use std::env::{self, VarError};
use std::str::FromStr;

use tracing_subscriber::EnvFilter;

pub use json::JsonFormat;
#[cfg(feature = "observability")]
pub use otel::{VARIABLE_ENDPOINT, VARIABLE_SERVICE};
pub use pretty::PrettyFormat;

/// Variable d'environnement choisissant le formateur.
pub const VARIABLE_FORMAT: &str = "RBS_LOG_FORMAT";

const NIVEAU_PAR_DEFAUT: &str = "info";

/// Échec de mise en place de la journalisation.
///
/// Distincte d'[`Error`](crate::Error) pour la même raison que
/// [`ConfigError`](crate::config::ConfigError) : une erreur survenue au démarrage ne
/// devient jamais une réponse HTTP.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LogError {
    /// Valeur de [`VARIABLE_FORMAT`] hors des formats connus.
    #[error("`{VARIABLE_FORMAT}` porte une valeur invalide : `{0}` (attendu `pretty` ou `json`)")]
    FormatInconnu(String),
    /// Un abonné global est déjà posé.
    #[error("abonné de journalisation déjà posé : {0}")]
    DejaInitialise(#[from] tracing::subscriber::SetGlobalDefaultError),
    /// L'exportateur OTLP ne se construit pas sur l'endpoint nommé.
    ///
    /// Le démarrage échoue plutôt qu'il ne continue muet : nommer un collecteur est une
    /// décision d'exploitation, et une API qui n'exporte rien après qu'on l'a prise ne
    /// se découvrirait qu'au premier incident.
    #[cfg(feature = "observability")]
    #[error("exportateur OTLP non construit : {0}")]
    Exportateur(#[from] opentelemetry_otlp::ExporterBuildError),
}

/// Format de rendu des logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LogFormat {
    /// Une ligne lisible par événement, colorée sur un terminal.
    #[default]
    Pretty,
    /// Un objet JSON par ligne.
    Json,
}

impl FromStr for LogFormat {
    type Err = LogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(LogError::FormatInconnu(value.to_owned())),
        }
    }
}

/// Pose l'abonné global de journalisation.
///
/// Le formateur vient de [`VARIABLE_FORMAT`], le filtrage de `RUST_LOG`.
///
/// # Erreurs
///
/// Échoue si [`VARIABLE_FORMAT`] porte une valeur inconnue — une faute de frappe ne doit
/// pas faire tourner la production dans un format inattendu — ou si un abonné global est
/// déjà posé.
pub fn init() -> Result<(), LogError> {
    let format = match env::var(VARIABLE_FORMAT) {
        Ok(value) => value.parse()?,
        Err(VarError::NotPresent) => LogFormat::default(),
        Err(VarError::NotUnicode(value)) => {
            return Err(LogError::FormatInconnu(
                value.to_string_lossy().into_owned(),
            ));
        }
    };

    let filtre =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(NIVEAU_PAR_DEFAUT));

    match format {
        LogFormat::Pretty => poser(
            tracing_subscriber::fmt()
                .with_env_filter(filtre)
                .fmt_fields(PrettyFormat::new())
                .event_format(PrettyFormat::new())
                .finish(),
        ),
        LogFormat::Json => poser(
            tracing_subscriber::fmt()
                .with_env_filter(filtre)
                .fmt_fields(JsonFormat::new())
                .event_format(JsonFormat::new())
                .finish(),
        ),
    }
}

/// Pose l'abonné global, l'export OTLP greffé dessus quand un collecteur est nommé.
///
/// La greffe se fait ici et non depuis le projet engendré : `set_global_default` ne
/// s'appelle qu'une fois, et cette ligne est la première de son `main`.
fn poser<S>(subscriber: S) -> Result<(), LogError>
where
    S: tracing::Subscriber
        + Send
        + Sync
        + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    #[cfg(feature = "observability")]
    let subscriber = {
        use tracing_subscriber::layer::SubscriberExt as _;

        subscriber.with(otel::couche()?)
    };

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// Vide les lots de spans encore en attente, et rend la main quand ils sont partis.
///
/// Sans la feature `observability`, ne fait rien. Un arrêt brutal sans cet appel perd le
/// dernier lot : le processus meurt avant que l'exportateur ne l'ait poussé.
pub fn shutdown() {
    #[cfg(feature = "observability")]
    otel::shutdown();
}

#[cfg(all(test, feature = "observability"))]
mod observability_tests {
    use super::*;

    /// Un seul test pour les quatre assertions : l'abonné global comme le fournisseur de
    /// traces ne se posent qu'une fois par processus. Répartis sur plusieurs tests,
    /// l'ordre d'exécution déciderait lesquels échouent.
    ///
    /// `tokio::test` et non `test` : l'exportateur gRPC se construit dans le contexte
    /// d'un runtime, comme le `main` engendré le lui donne.
    #[tokio::test]
    async fn the_collector_is_named_by_the_environment_or_nothing_is_exported() {
        // Avant tout `init` : un arrêt demandé sur un export jamais monté ne doit pas
        // tomber. C'est le cas du projet qui appelle `shutdown` dans un chemin d'erreur
        // atteint avant le démarrage.
        shutdown();

        // SAFETY: aucun autre test de cette crate ne lit ni n'écrit ces variables.
        unsafe { env::remove_var(otel::VARIABLE_ENDPOINT) };

        init().expect("l'abonné doit se poser quand aucun collecteur n'est nommé");

        assert!(
            !otel::exportateur_installe(),
            "un exportateur a été monté alors qu'aucun collecteur n'est nommé"
        );

        // SAFETY: idem — et la pose d'abonné, seule concurrente possible, est faite.
        unsafe { env::set_var(otel::VARIABLE_ENDPOINT, "http://127.0.0.1:4317") };

        otel::couche::<tracing_subscriber::Registry>()
            .expect("l'exportateur doit se construire sur un endpoint nommé");

        assert!(
            otel::exportateur_installe(),
            "aucun exportateur monté alors qu'un collecteur est nommé"
        );

        shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_reads_from_its_name() {
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(" JSON ".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }

    #[test]
    fn an_unknown_format_is_rejected_naming_the_variable_and_the_allowed_values() {
        let error = "text".parse::<LogFormat>().unwrap_err().to_string();

        assert!(
            error.contains("RBS_LOG_FORMAT"),
            "variable non nommée : {error}"
        );
        assert!(error.contains("text"), "valeur fautive absente : {error}");
        assert!(
            error.contains("pretty") && error.contains("json"),
            "valeurs admises absentes : {error}"
        );
    }
}
