//! La langue du projet : son `AGENTS.md` et ses réponses HTTP.
//!
//! Les deux se choisissent séparément. `[package.metadata.rbs] lang` ne gouverne plus que
//! `AGENTS.md` : sans elle, `add` et `upgrade` réécriraient un guide français par-dessus un
//! guide anglais selon l'environnement de celui qui les lance. `[server] lang` de
//! `config/default.toml` gouverne tout le reste — les réponses HTTP que `rbs-core` lit au
//! démarrage, et les messages destinés au client que `add` et `generate` engendrent : avant
//! elles lisaient la métadonnée, qui pouvait diverger (elle se remplissait de la locale
//! avant 1.5.0) et produire des messages en deux langues dans un même projet. L'environnement
//! n'est jamais lu à la génération, pour la même raison que `from_locale` ne l'est pas pour
//! `AGENTS.md` : une génération ne doit rien devoir au shell de celui qui la lance.

use std::fmt;
use std::path::Path;

/// Chemin, relatif à la racine du projet, du fichier où vit `[server] lang`.
const CONFIG: &str = "config/default.toml";

/// Langue du projet : son `AGENTS.md` et ses réponses HTTP.
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

    /// L'étiquette BCP 47 que prend une API d'internationalisation du navigateur.
    ///
    /// Un territoire et non la seule langue : `Intl.DateTimeFormat` rend `20/09/2026` sur
    /// `fr-FR` et `2026-09-20` sur le `fr` nu, que l'implémentation résout où elle veut.
    /// Épinglée à la génération plutôt que laissée au navigateur de l'opérateur : l'écran
    /// engendré porte déjà tous ses libellés dans la langue du projet, et une date au
    /// format d'un autre pays y jurerait.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Fr => "fr-FR",
            Self::En => "en-US",
        }
    }

    /// La langue que ce nom désigne, ou `None` s'il n'en désigne aucune.
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
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(locale) if locale.starts_with("fr") => Self::Fr,
            Some(locale) if !locale.is_empty() => Self::En,
            _ => Self::Fr,
        }
    }

    /// La langue des messages qu'`add` et `generate` engendrent pour le projet à `root`.
    ///
    /// Lue dans `[server] lang` de `config/default.toml`, la même clé que `rbs-core` lit
    /// au démarrage pour choisir la langue des réponses HTTP — jamais dans les métadonnées
    /// ni dans l'environnement, qui peuvent diverger de ce que le serveur rendra vraiment.
    /// Fichier absent, illisible, mal formé, table ou clé absente, ou valeur inconnue :
    /// French, comme la résolution paresseuse du runtime.
    pub fn of_project(root: &Path) -> Self {
        std::fs::read_to_string(root.join(CONFIG))
            .ok()
            .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
            .and_then(|document| {
                document
                    .get("server")?
                    .get("lang")?
                    .as_str()
                    .map(String::from)
            })
            .and_then(|lang| Self::parse(&lang))
            .unwrap_or_default()
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

    /// Le territoire compte : `fr` nu laisserait le format de date à l'implémentation.
    #[test]
    fn each_language_carries_a_territory_in_its_tag() {
        assert_eq!(Lang::Fr.tag(), "fr-FR");
        assert_eq!(Lang::En.tag(), "en-US");
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

    /// Sans fichier, la résolution est celle du runtime : French, pas une panne.
    #[test]
    fn a_project_without_config_file_speaks_french() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    #[test]
    fn a_config_without_the_server_table_speaks_french() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[docs]\nswagger_ui = true\n");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    #[test]
    fn a_server_table_without_lang_speaks_french() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[server]\nport = 8080\n");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    #[test]
    fn server_lang_english_is_read_back() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[server]\nlang = \"en\"\n");

        assert_eq!(Lang::of_project(root.path()), Lang::En);
    }

    #[test]
    fn server_lang_french_is_read_back() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[server]\nlang = \"fr\"\n");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    /// Une valeur écrite à la main peut porter n'importe quoi : le contrôleur retombe sur
    /// le français plutôt que de faire échouer la commande, comme le fait déjà la lecture
    /// paresseuse du runtime.
    #[test]
    fn an_unknown_server_lang_falls_back_to_french() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[server]\nlang = \"de\"\n");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    /// Un fichier mal formé ne doit pas non plus faire échouer la commande : c'est le même
    /// arbitrage que l'absence de fichier.
    #[test]
    fn an_unparseable_config_speaks_french() {
        let root = tempfile::TempDir::new().expect("répertoire temporaire créable");
        write_config(root.path(), "[server\nlang = \"en\"\n");

        assert_eq!(Lang::of_project(root.path()), Lang::Fr);
    }

    fn write_config(root: &std::path::Path, content: &str) {
        std::fs::create_dir_all(root.join("config")).expect("le répertoire config se crée");
        std::fs::write(root.join(CONFIG), content).expect("le fichier de configuration s'écrit");
    }
}
