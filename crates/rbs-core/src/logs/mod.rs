//! Journalisation du runtime : deux formateurs et la bascule qui les choisit.

#[cfg(test)]
pub(crate) mod aide;
mod json;
mod pretty;

use std::env::{self, VarError};
use std::str::FromStr;

use tracing_subscriber::EnvFilter;

pub use json::JsonFormat;
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
pub enum LogError {
    /// Valeur de [`VARIABLE_FORMAT`] hors des formats connus.
    #[error("`{VARIABLE_FORMAT}` porte une valeur invalide : `{0}` (attendu `pretty` ou `json`)")]
    FormatInconnu(String),
    /// Un abonné global est déjà posé.
    #[error("abonné de journalisation déjà posé : {0}")]
    DejaInitialise(#[from] tracing::subscriber::SetGlobalDefaultError),
}

/// Format de rendu des logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Une ligne lisible par événement, colorée sur un terminal.
    #[default]
    Pretty,
    /// Un objet JSON par ligne.
    Json,
}

impl FromStr for LogFormat {
    type Err = LogError;

    fn from_str(valeur: &str) -> Result<Self, Self::Err> {
        match valeur.trim().to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(LogError::FormatInconnu(valeur.to_owned())),
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
        Ok(valeur) => valeur.parse()?,
        Err(VarError::NotPresent) => LogFormat::default(),
        Err(VarError::NotUnicode(valeur)) => {
            return Err(LogError::FormatInconnu(
                valeur.to_string_lossy().into_owned(),
            ));
        }
    };

    let filtre =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(NIVEAU_PAR_DEFAUT));

    match format {
        LogFormat::Pretty => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(filtre)
                .fmt_fields(PrettyFormat::new())
                .event_format(PrettyFormat::new())
                .finish(),
        )?,
        LogFormat::Json => tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_env_filter(filtre)
                .fmt_fields(JsonFormat::new())
                .event_format(JsonFormat::new())
                .finish(),
        )?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_format_se_lit_depuis_son_nom() {
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(" JSON ".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }

    #[test]
    fn un_format_inconnu_est_refuse_en_nommant_la_variable_et_les_valeurs_admises() {
        let erreur = "texte".parse::<LogFormat>().unwrap_err().to_string();

        assert!(
            erreur.contains("RBS_LOG_FORMAT"),
            "variable non nommée : {erreur}"
        );
        assert!(
            erreur.contains("texte"),
            "valeur fautive absente : {erreur}"
        );
        assert!(
            erreur.contains("pretty") && erreur.contains("json"),
            "valeurs admises absentes : {erreur}"
        );
    }
}
