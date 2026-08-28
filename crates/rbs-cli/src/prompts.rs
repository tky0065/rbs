use std::fmt;

use inquire::{InquireError, MultiSelect, Text};

/// Nom retenu quand ni le flag ni la question ne l'ont fixé.
const NOM_DEFAUT: &str = "mon-api";

/// Features proposées à la création. Les autres arrivent en v0.2.
const FEATURES_DISPONIBLES: &[&str] = &["docker", "ci"];

/// Les réponses aux trois questions de `rbs new`, d'où qu'elles viennent.
#[derive(Debug, PartialEq)]
pub struct ProjectOptions {
    /// Nom du projet, qui est aussi celui du répertoire créé.
    pub name: String,
    /// URL de connexion PostgreSQL écrite dans le `.env` du projet.
    pub database_url: String,
    /// Features à installer à la création.
    pub features: Vec<String>,
}

/// Ce qui empêche une question d'aboutir, traduit en conseil actionnable.
#[derive(Debug, PartialEq)]
pub enum PromptError {
    /// Aucun terminal interactif : seuls les flags peuvent encore fournir les réponses.
    SansTerminal,
    /// L'utilisateur a coupé court (Ctrl-C ou Échap).
    Interrompu,
    /// Tout autre échec remonté par `inquire`.
    Autre(String),
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SansTerminal => f.write_str(
                "aucun terminal interactif pour poser les questions : relancez avec `--yes` \
                 pour prendre les défauts, ou donnez les réponses en flags — le name en \
                 argument, `--database-url` et `--with`",
            ),
            Self::Interrompu => f.write_str("questions interrompues : aucun projet n'a été créé"),
            Self::Autre(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PromptError {}

/// Les trois questions de `rbs new`, isolées derrière un trait pour que la résolution
/// soit testable sans terminal — et que l'absence d'appel soit observable.
trait Questions {
    fn name(&self, defaut: &str) -> Result<String, PromptError>;
    fn database_url(&self, defaut: &str) -> Result<String, PromptError>;
    fn features(&self, disponibles: &[&str]) -> Result<Vec<String>, PromptError>;
}

/// Les questions telles que l'utilisateur les voit.
struct Interactive;

impl Questions for Interactive {
    fn name(&self, defaut: &str) -> Result<String, PromptError> {
        Text::new("Nom du projet ?")
            .with_default(defaut)
            .prompt()
            .map_err(translate)
    }

    fn database_url(&self, defaut: &str) -> Result<String, PromptError> {
        Text::new("URL de la base PostgreSQL ?")
            .with_default(defaut)
            .with_help_message("PostgreSQL 18 minimum : `uuidv7()` y est natif")
            .prompt()
            .map_err(translate)
    }

    fn features(&self, disponibles: &[&str]) -> Result<Vec<String>, PromptError> {
        MultiSelect::new("Features à installer ?", disponibles.to_vec())
            .with_help_message(
                "espace pour cocher, entrée pour valider — `rbs add` en ajoute plus tard",
            )
            .prompt()
            .map(|choisies| choisies.into_iter().map(str::to_string).collect())
            .map_err(translate)
    }
}

/// Traduit l'échec d'`inquire` en cause que l'utilisateur peut corriger. Sans cette
/// étape, l'absence de TTY remonte comme un descripteur fermé, qui ne dit pas que
/// `--yes` existe.
fn translate(error: InquireError) -> PromptError {
    match error {
        InquireError::NotTTY | InquireError::IO(_) => PromptError::SansTerminal,
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            PromptError::Interrompu
        }
        other => PromptError::Autre(other.to_string()),
    }
}

/// URL par défaut, dérivée du nom du projet.
fn default_database_url(name: &str) -> String {
    // Un identifiant PostgreSQL non entre guillemets n'admet pas le tiret.
    let base = name.replace('-', "_");
    format!("postgres://postgres:postgres@localhost:5432/{base}")
}

/// Complète les valeurs absentes des flags, en questionnant l'utilisateur sauf si `yes`.
pub fn resolve(
    name: Option<String>,
    database_url: Option<String>,
    features: Option<Vec<String>>,
    yes: bool,
) -> Result<ProjectOptions, PromptError> {
    resolve_with(&Interactive, name, database_url, features, yes)
}

/// `yes` court-circuite avant toute question : la résolution devient purement
/// calculatoire. Configurer `inquire` pour qu'il « prenne le défaut » ne marcherait pas —
/// il échoue de lui-même sans TTY, et le CLI cesserait d'être utilisable en CI.
fn resolve_with<Q: Questions>(
    questions: &Q,
    name: Option<String>,
    database_url: Option<String>,
    features: Option<Vec<String>>,
    yes: bool,
) -> Result<ProjectOptions, PromptError> {
    let name = match name {
        Some(name) => name,
        None if yes => NOM_DEFAUT.to_string(),
        None => questions.name(NOM_DEFAUT)?,
    };

    let defaut_url = default_database_url(&name);
    let database_url = match database_url {
        Some(url) => url,
        None if yes => defaut_url,
        None => questions.database_url(&defaut_url)?,
    };

    let features = match features {
        Some(features) => features,
        None if yes => Vec::new(),
        None => questions.features(FEATURES_DISPONIBLES)?,
    };

    Ok(ProjectOptions {
        name,
        database_url,
        features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Compte les questions posées au lieu d'y répondre : un test n'a pas de terminal,
    /// et c'est justement l'absence d'appel qu'on cherche à prouver.
    #[derive(Default)]
    struct Spy {
        written: RefCell<Vec<&'static str>>,
    }

    impl Spy {
        fn written(&self) -> Vec<&'static str> {
            self.written.borrow().clone()
        }
    }

    impl Questions for Spy {
        fn name(&self, _defaut: &str) -> Result<String, PromptError> {
            self.written.borrow_mut().push("name");
            Ok("repondu".to_string())
        }

        fn database_url(&self, _defaut: &str) -> Result<String, PromptError> {
            self.written.borrow_mut().push("database_url");
            Ok("postgres://repondu".to_string())
        }

        fn features(&self, _disponibles: &[&str]) -> Result<Vec<String>, PromptError> {
            self.written.borrow_mut().push("features");
            Ok(vec!["repondu".to_string()])
        }
    }

    #[test]
    fn with_yes_resolution_returns_the_defaults_without_asking() {
        let espion = Spy::default();

        let options = resolve_with(&espion, None, None, None, true).unwrap();

        assert!(
            espion.written().is_empty(),
            "des questions ont été posées : {:?}",
            espion.written()
        );
        assert_eq!(options.name, NOM_DEFAUT);
        assert_eq!(
            options.database_url,
            "postgres://postgres:postgres@localhost:5432/mon_api"
        );
        assert!(options.features.is_empty());
    }

    #[test]
    fn the_name_flag_wins_over_the_default_and_names_the_database() {
        let espion = Spy::default();

        let options =
            resolve_with(&espion, Some("mon-projet".to_string()), None, None, true).unwrap();

        assert!(espion.written().is_empty());
        assert_eq!(options.name, "mon-projet");
        // Un tiret n'est pas un caractère de nom de base sans guillemets.
        assert_eq!(
            options.database_url,
            "postgres://postgres:postgres@localhost:5432/mon_projet"
        );
    }

    #[test]
    fn the_url_flag_wins_over_the_default() {
        let espion = Spy::default();

        let options = resolve_with(
            &espion,
            None,
            Some("postgres://ailleurs:5432/db".to_string()),
            None,
            true,
        )
        .unwrap();

        assert!(espion.written().is_empty());
        assert_eq!(options.database_url, "postgres://ailleurs:5432/db");
    }

    #[test]
    fn the_feature_flags_win_over_the_default() {
        let espion = Spy::default();

        let options = resolve_with(
            &espion,
            None,
            None,
            Some(vec!["docker".to_string(), "ci".to_string()]),
            true,
        )
        .unwrap();

        assert!(espion.written().is_empty());
        assert_eq!(options.features, ["docker", "ci"]);
    }

    #[test]
    fn without_yes_each_missing_value_becomes_a_question() {
        let espion = Spy::default();

        let options = resolve_with(&espion, None, None, None, false).unwrap();

        assert_eq!(espion.written(), ["name", "database_url", "features"]);
        assert_eq!(options.name, "repondu");
        assert_eq!(options.database_url, "postgres://repondu");
        assert_eq!(options.features, ["repondu"]);
    }

    #[test]
    fn without_yes_a_supplied_flag_skips_its_question() {
        let espion = Spy::default();

        resolve_with(&espion, Some("api".to_string()), None, None, false).unwrap();

        assert_eq!(espion.written(), ["database_url", "features"]);
    }

    #[test]
    fn without_a_terminal_the_error_says_how_to_do_without_one() {
        let message = translate(inquire::InquireError::NotTTY).to_string();

        assert!(
            message.contains("--yes"),
            "l'erreur ne dit pas quoi faire :\n{message}"
        );
        assert!(
            message.contains("--database-url") && message.contains("--with"),
            "l'erreur ne nomme pas les flags équivalents :\n{message}"
        );
    }

    #[test]
    fn an_interrupt_is_not_confused_with_the_absence_of_a_terminal() {
        let interrompu = translate(inquire::InquireError::OperationInterrupted);

        assert_eq!(interrompu, PromptError::Interrompu);
    }
}
