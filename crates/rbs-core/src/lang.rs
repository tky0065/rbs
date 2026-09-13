//! Langue dans laquelle le projet parle à ses clients HTTP.
//!
//! Un global de processus plutôt qu'un champ de l'état : `Error::into_response` ne voit
//! pas l'état, et le document OpenAPI se construit hors de toute requête.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::Deserialize;

/// Langue des corps d'erreur et des descriptions communes du document OpenAPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Lang {
    /// Français, la langue d'un projet qui n'en déclare aucune.
    #[default]
    Fr,
    /// Anglais.
    En,
}

/// `0` : rien n'est encore posé ni résolu.
static COURANTE: AtomicU8 = AtomicU8::new(0);

impl Lang {
    fn code(self) -> u8 {
        match self {
            Self::Fr => 1,
            Self::En => 2,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Fr),
            2 => Some(Self::En),
            _ => None,
        }
    }
}

/// Pose la langue du processus ; [`Config::load`](crate::Config::load) l'appelle.
pub fn set(lang: Lang) {
    COURANTE.store(lang.code(), Ordering::Relaxed);
}

/// La langue du processus : celle que [`set`] a posée, à défaut `[server] lang`.
pub fn current() -> Lang {
    if let Some(lang) = Lang::from_code(COURANTE.load(Ordering::Relaxed)) {
        return lang;
    }

    let resolue = resolve();
    // Un `set` survenu entre-temps l'emporte : il vient d'une configuration complète.
    match COURANTE.compare_exchange(0, resolue.code(), Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => resolue,
        Err(posee) => Lang::from_code(posee).unwrap_or(resolue),
    }
}

#[derive(Deserialize)]
struct Server {
    #[serde(default)]
    lang: Lang,
}

/// `[server] lang` par la cascade de configuration, le français sur toute erreur.
///
/// C'est le chemin de `bin/openapi.rs`, qui rend le document sans charger la
/// configuration complète : un `database.url` absent n'y est pas une faute.
fn resolve() -> Lang {
    crate::config::section::<Server>("server")
        .map(|server| server.lang)
        .unwrap_or_default()
}

// Aucun test n'appelle `current()` : une résolution paresseuse sur un thread hors d'un
// `Jail` peut lire le répertoire de travail ou l'environnement d'un `Jail` d'un autre
// test et figer une langue dans `COURANTE` pour le reste du processus de test. Les
// valeurs qui dépendent de la langue ne sont donc éprouvées qu'à travers les fonctions
// pures `parts` (`error.rs`), `declare` (`openapi.rs`) et `resolve` ci-dessous.
#[cfg(test)]
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn a_language_deserialises_from_its_lowercase_name() {
        assert_eq!(serde_json::from_str::<Lang>("\"fr\"").unwrap(), Lang::Fr);
        assert_eq!(serde_json::from_str::<Lang>("\"en\"").unwrap(), Lang::En);
        assert!(serde_json::from_str::<Lang>("\"EN\"").is_err());
    }

    #[test]
    fn without_configuration_the_language_resolves_to_french() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            assert_eq!(resolve(), Lang::Fr);
            Ok(())
        });
    }

    #[test]
    fn the_language_is_read_from_the_server_section() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", "[server]\nlang = \"en\"\n")?;
            assert_eq!(resolve(), Lang::En);
            Ok(())
        });
    }

    #[test]
    fn the_environment_overrides_the_language() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("RBS_SERVER__LANG", "en");
            assert_eq!(resolve(), Lang::En);
            Ok(())
        });
    }

    /// `bin/openapi.rs` n'a pas à échouer sur une valeur que `Config::load` refusera.
    #[test]
    fn an_unknown_language_resolves_to_french() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("RBS_SERVER__LANG", "de");
            assert_eq!(resolve(), Lang::Fr);
            Ok(())
        });
    }
}
