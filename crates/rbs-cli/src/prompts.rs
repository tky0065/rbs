use std::fmt;

use inquire::{InquireError, MultiSelect, Text};

use crate::database::Database;

/// Nom retenu quand ni le flag ni la question ne l'ont fixé.
const NOM_DEFAUT: &str = "mon-api";

/// Les réponses aux trois questions de `rbs new`, d'où qu'elles viennent.
#[derive(Debug, PartialEq)]
pub struct ProjectOptions {
    /// Nom du projet, qui est aussi celui du répertoire créé.
    pub name: String,
    /// URL de connexion écrite dans le `.env` du projet.
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
    fn database_url(&self, database: Database, defaut: &str) -> Result<String, PromptError>;
    fn features(&self, disponibles: &[String]) -> Result<Vec<String>, PromptError>;
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

    fn database_url(&self, database: Database, defaut: &str) -> Result<String, PromptError> {
        let question = format!("URL de la base {database} ?");
        let mut champ = Text::new(&question).with_default(defaut);

        if let Some(aide) = help_for(database) {
            champ = champ.with_help_message(aide);
        }

        champ.prompt().map_err(translate)
    }

    fn features(&self, disponibles: &[String]) -> Result<Vec<String>, PromptError> {
        MultiSelect::new("Features à installer ?", disponibles.to_vec())
            .with_help_message(
                "espace pour cocher, entrée pour valider — `rbs add` en ajoute plus tard",
            )
            .prompt()
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

/// Ce que la question ajoute pour le moteur choisi, quand elle a quelque chose à dire.
fn help_for(database: Database) -> Option<&'static str> {
    match database {
        Database::Postgres => {
            Some("PostgreSQL 14 minimum : les versions antérieures sont hors support")
        }
        Database::Mysql => None,
        Database::Sqlite => Some("un chemin de fichier : le serveur n'existe pas"),
    }
}

/// URL par défaut, dérivée du moteur et du nom du projet.
fn default_database_url(database: Database, name: &str) -> String {
    // Un identifiant de base non entre guillemets n'admet pas le tiret.
    database.default_url(&name.replace('-', "_"))
}

/// Complète les valeurs absentes des flags, en questionnant l'utilisateur sauf si `yes`.
pub fn resolve(
    name: Option<String>,
    database_url: Option<String>,
    database: Database,
    features: Option<Vec<String>>,
    disponibles: &[String],
    yes: bool,
) -> Result<ProjectOptions, PromptError> {
    resolve_with(
        &Interactive,
        name,
        database_url,
        database,
        features,
        disponibles,
        yes,
    )
}

/// `yes` court-circuite avant toute question : la résolution devient purement
/// calculatoire. Configurer `inquire` pour qu'il « prenne le défaut » ne marcherait pas —
/// il échoue de lui-même sans TTY, et le CLI cesserait d'être utilisable en CI.
fn resolve_with<Q: Questions>(
    questions: &Q,
    name: Option<String>,
    database_url: Option<String>,
    database: Database,
    features: Option<Vec<String>>,
    disponibles: &[String],
    yes: bool,
) -> Result<ProjectOptions, PromptError> {
    let name = match name {
        Some(name) => name,
        None if yes => NOM_DEFAUT.to_string(),
        None => questions.name(NOM_DEFAUT)?,
    };

    let defaut_url = default_database_url(database, &name);
    let database_url = match database_url {
        Some(url) => url,
        None if yes => defaut_url,
        None => questions.database_url(database, &defaut_url)?,
    };

    let features = match features {
        Some(features) => features,
        None if yes => Vec::new(),
        None => questions.features(disponibles)?,
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
        /// Ce que la question « features » a reçu à proposer, pour prouver qu'elle
        /// reçoit bien la liste dérivée des fragments et non une liste écrite à la main.
        features_recues: RefCell<Vec<String>>,
    }

    impl Spy {
        fn written(&self) -> Vec<&'static str> {
            self.written.borrow().clone()
        }

        fn features_proposees(&self) -> Vec<String> {
            self.features_recues.borrow().clone()
        }
    }

    impl Questions for Spy {
        fn name(&self, _defaut: &str) -> Result<String, PromptError> {
            self.written.borrow_mut().push("name");
            Ok("repondu".to_string())
        }

        fn database_url(&self, _database: Database, _defaut: &str) -> Result<String, PromptError> {
            self.written.borrow_mut().push("database_url");
            Ok("postgres://repondu".to_string())
        }

        fn features(&self, disponibles: &[String]) -> Result<Vec<String>, PromptError> {
            self.written.borrow_mut().push("features");
            *self.features_recues.borrow_mut() = disponibles.to_vec();
            Ok(vec!["repondu".to_string()])
        }
    }

    /// Une liste minimale pour les tests dont l'assertion ne porte pas sur son contenu.
    fn disponibles() -> Vec<String> {
        vec!["docker".to_string(), "ci".to_string()]
    }

    /// Une liste écrite à la main se désynchronise : celle-ci se dérive des fragments
    /// que le binaire embarque.
    #[test]
    fn the_question_offers_every_embedded_feature() {
        let spy = Spy::default();

        resolve_with(
            &spy,
            Some("demo".into()),
            Some("postgres://x".into()),
            Database::Postgres,
            None,
            &crate::templates::feature_names(None),
            false,
        )
        .expect("les questions doivent aboutir");

        assert_eq!(
            spy.features_proposees(),
            crate::templates::feature_names(None),
            "la question doit proposer les sept fragments"
        );
    }

    #[test]
    fn with_yes_resolution_returns_the_defaults_without_asking() {
        let espion = Spy::default();

        let options = resolve_with(
            &espion,
            None,
            None,
            Database::default(),
            None,
            &disponibles(),
            true,
        )
        .unwrap();

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

        let options = resolve_with(
            &espion,
            Some("mon-projet".to_string()),
            None,
            Database::default(),
            None,
            &disponibles(),
            true,
        )
        .unwrap();

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
            Database::default(),
            None,
            &disponibles(),
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
            Database::default(),
            Some(vec!["docker".to_string(), "ci".to_string()]),
            &disponibles(),
            true,
        )
        .unwrap();

        assert!(espion.written().is_empty());
        assert_eq!(options.features, ["docker", "ci"]);
    }

    #[test]
    fn without_yes_each_missing_value_becomes_a_question() {
        let espion = Spy::default();

        let options = resolve_with(
            &espion,
            None,
            None,
            Database::default(),
            None,
            &disponibles(),
            false,
        )
        .unwrap();

        assert_eq!(espion.written(), ["name", "database_url", "features"]);
        assert_eq!(options.name, "repondu");
        assert_eq!(options.database_url, "postgres://repondu");
        assert_eq!(options.features, ["repondu"]);
    }

    #[test]
    fn without_yes_a_supplied_flag_skips_its_question() {
        let espion = Spy::default();

        resolve_with(
            &espion,
            Some("api".to_string()),
            None,
            Database::default(),
            None,
            &disponibles(),
            false,
        )
        .unwrap();

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
