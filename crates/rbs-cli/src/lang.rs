//! La langue dans laquelle le projet reçoit son `AGENTS.md`.
//!
//! Le choix est celui du projet, non celui de la session : il s'inscrit dans
//! `[package.metadata.rbs]`, sans quoi `add` et `upgrade` réécriraient un guide français
//! par-dessus un guide anglais selon l'environnement de celui qui les lance.

use std::fmt;

/// Langue du guide engendré dans le projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Lang {
    /// Français, la langue du dépôt et du code engendré.
    #[default]
    Fr,
    /// Anglais.
    En,
}

impl Lang {
    /// Nom de la langue, tel qu'il s'écrit au flag et dans `[package.metadata.rbs]`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
        }
    }

    /// La langue que ce nom désigne, ou `None` s'il n'en désigne aucune.
    #[allow(dead_code)]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "fr" => Some(Self::Fr),
            "en" => Some(Self::En),
            _ => None,
        }
    }

    /// La langue que suggère une locale POSIX, le français à défaut.
    ///
    /// Seul le préfixe est lu : une locale porte son territoire et son encodage
    /// (`fr_FR.UTF-8`), qu'une comparaison stricte manquerait.
    #[allow(dead_code)]
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(locale) if locale.starts_with("fr") => Self::Fr,
            Some(locale) if !locale.is_empty() => Self::En,
            _ => Self::Fr,
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_language_carries_the_name_written_in_the_manifest() {
        assert_eq!(Lang::Fr.name(), "fr");
        assert_eq!(Lang::En.name(), "en");
    }

    #[test]
    fn a_known_name_parses_back_to_its_language() {
        assert_eq!(Lang::parse("fr"), Some(Lang::Fr));
        assert_eq!(Lang::parse("en"), Some(Lang::En));
    }

    /// Une clé écrite à la main peut porter n'importe quoi : la refuser ici laisse
    /// l'appelant décider s'il retombe sur un défaut ou s'il échoue.
    #[test]
    fn an_unknown_name_parses_to_nothing() {
        assert_eq!(Lang::parse("de"), None);
        assert_eq!(Lang::parse(""), None);
    }

    /// Les locales POSIX portent le territoire et l'encodage : `fr_FR.UTF-8` désigne bien
    /// le français, et une comparaison stricte le manquerait.
    #[test]
    fn a_french_locale_is_recognised_with_its_territory_and_encoding() {
        assert_eq!(Lang::from_locale(Some("fr_FR.UTF-8")), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("fr")), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("fr_CA")), Lang::Fr);
    }

    #[test]
    fn every_other_locale_gives_english() {
        assert_eq!(Lang::from_locale(Some("en_US.UTF-8")), Lang::En);
        assert_eq!(Lang::from_locale(Some("de_DE")), Lang::En);
        assert_eq!(Lang::from_locale(Some("C")), Lang::En);
    }

    /// Un environnement sans locale ne dit rien de la langue de l'utilisateur : le défaut
    /// est celui du dépôt.
    #[test]
    fn an_absent_locale_falls_back_to_french() {
        assert_eq!(Lang::from_locale(None), Lang::Fr);
        assert_eq!(Lang::from_locale(Some("")), Lang::Fr);
    }

    #[test]
    fn the_display_is_the_name_written_in_the_manifest() {
        assert_eq!(Lang::En.to_string(), "en");
    }
}
