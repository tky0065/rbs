use std::fmt;

use inquire::{InquireError, MultiSelect, Text};

/// Nom retenu quand ni le flag ni la question ne l'ont fixé.
const NOM_DEFAUT: &str = "mon-api";

/// Features proposées à la création. Les autres arrivent en v0.2.
const FEATURES_DISPONIBLES: &[&str] = &["docker", "ci"];

/// Les réponses aux trois questions de `rbs new`, d'où qu'elles viennent.
#[derive(Debug, PartialEq)]
pub struct OptionsProjet {
    /// Nom du projet, qui est aussi celui du répertoire créé.
    pub nom: String,
    /// URL de connexion PostgreSQL écrite dans le `.env` du projet.
    pub database_url: String,
    /// Features à installer à la création.
    pub features: Vec<String>,
}

/// Ce qui empêche une question d'aboutir, traduit en conseil actionnable.
#[derive(Debug, PartialEq)]
pub enum ErreurPrompt {
    /// Aucun terminal interactif : seuls les flags peuvent encore fournir les réponses.
    SansTerminal,
    /// L'utilisateur a coupé court (Ctrl-C ou Échap).
    Interrompu,
    /// Tout autre échec remonté par `inquire`.
    Autre(String),
}

impl fmt::Display for ErreurPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SansTerminal => f.write_str(
                "aucun terminal interactif pour poser les questions : relancez avec `--yes` \
                 pour prendre les défauts, ou donnez les réponses en flags — le nom en \
                 argument, `--database-url` et `--with`",
            ),
            Self::Interrompu => f.write_str("questions interrompues : aucun projet n'a été créé"),
            Self::Autre(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ErreurPrompt {}

/// Les trois questions de `rbs new`, isolées derrière un trait pour que la résolution
/// soit testable sans terminal — et que l'absence d'appel soit observable.
trait Questions {
    fn nom(&self, defaut: &str) -> Result<String, ErreurPrompt>;
    fn database_url(&self, defaut: &str) -> Result<String, ErreurPrompt>;
    fn features(&self, disponibles: &[&str]) -> Result<Vec<String>, ErreurPrompt>;
}

/// Les questions telles que l'utilisateur les voit.
struct Interactif;

impl Questions for Interactif {
    fn nom(&self, defaut: &str) -> Result<String, ErreurPrompt> {
        Text::new("Nom du projet ?")
            .with_default(defaut)
            .prompt()
            .map_err(traduire)
    }

    fn database_url(&self, defaut: &str) -> Result<String, ErreurPrompt> {
        Text::new("URL de la base PostgreSQL ?")
            .with_default(defaut)
            .with_help_message("PostgreSQL 18 minimum : `uuidv7()` y est natif")
            .prompt()
            .map_err(traduire)
    }

    fn features(&self, disponibles: &[&str]) -> Result<Vec<String>, ErreurPrompt> {
        MultiSelect::new("Features à installer ?", disponibles.to_vec())
            .with_help_message(
                "espace pour cocher, entrée pour valider — `rbs add` en ajoute plus tard",
            )
            .prompt()
            .map(|choisies| choisies.into_iter().map(str::to_string).collect())
            .map_err(traduire)
    }
}

/// Traduit l'échec d'`inquire` en cause que l'utilisateur peut corriger. Sans cette
/// étape, l'absence de TTY remonte comme un descripteur fermé, qui ne dit pas que
/// `--yes` existe.
fn traduire(erreur: InquireError) -> ErreurPrompt {
    match erreur {
        InquireError::NotTTY | InquireError::IO(_) => ErreurPrompt::SansTerminal,
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            ErreurPrompt::Interrompu
        }
        autre => ErreurPrompt::Autre(autre.to_string()),
    }
}

/// URL par défaut, dérivée du nom du projet.
fn database_url_defaut(nom: &str) -> String {
    // Un identifiant PostgreSQL non entre guillemets n'admet pas le tiret.
    let base = nom.replace('-', "_");
    format!("postgres://postgres:postgres@localhost:5432/{base}")
}

/// Complète les valeurs absentes des flags, en questionnant l'utilisateur sauf si `yes`.
pub fn resoudre(
    nom: Option<String>,
    database_url: Option<String>,
    features: Option<Vec<String>>,
    yes: bool,
) -> Result<OptionsProjet, ErreurPrompt> {
    resoudre_avec(&Interactif, nom, database_url, features, yes)
}

/// `yes` court-circuite avant toute question : la résolution devient purement
/// calculatoire. Configurer `inquire` pour qu'il « prenne le défaut » ne marcherait pas —
/// il échoue de lui-même sans TTY, et le CLI cesserait d'être utilisable en CI.
fn resoudre_avec<Q: Questions>(
    questions: &Q,
    nom: Option<String>,
    database_url: Option<String>,
    features: Option<Vec<String>>,
    yes: bool,
) -> Result<OptionsProjet, ErreurPrompt> {
    let nom = match nom {
        Some(nom) => nom,
        None if yes => NOM_DEFAUT.to_string(),
        None => questions.nom(NOM_DEFAUT)?,
    };

    let defaut_url = database_url_defaut(&nom);
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

    Ok(OptionsProjet {
        nom,
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
    struct Espion {
        posees: RefCell<Vec<&'static str>>,
    }

    impl Espion {
        fn posees(&self) -> Vec<&'static str> {
            self.posees.borrow().clone()
        }
    }

    impl Questions for Espion {
        fn nom(&self, _defaut: &str) -> Result<String, ErreurPrompt> {
            self.posees.borrow_mut().push("nom");
            Ok("repondu".to_string())
        }

        fn database_url(&self, _defaut: &str) -> Result<String, ErreurPrompt> {
            self.posees.borrow_mut().push("database_url");
            Ok("postgres://repondu".to_string())
        }

        fn features(&self, _disponibles: &[&str]) -> Result<Vec<String>, ErreurPrompt> {
            self.posees.borrow_mut().push("features");
            Ok(vec!["repondu".to_string()])
        }
    }

    #[test]
    fn avec_yes_la_resolution_rend_les_defauts_sans_rien_demander() {
        let espion = Espion::default();

        let options = resoudre_avec(&espion, None, None, None, true).unwrap();

        assert!(
            espion.posees().is_empty(),
            "des questions ont été posées : {:?}",
            espion.posees()
        );
        assert_eq!(options.nom, NOM_DEFAUT);
        assert_eq!(
            options.database_url,
            "postgres://postgres:postgres@localhost:5432/mon_api"
        );
        assert!(options.features.is_empty());
    }

    #[test]
    fn le_nom_en_flag_prend_le_pas_sur_le_defaut_et_nomme_la_base() {
        let espion = Espion::default();

        let options =
            resoudre_avec(&espion, Some("mon-projet".to_string()), None, None, true).unwrap();

        assert!(espion.posees().is_empty());
        assert_eq!(options.nom, "mon-projet");
        // Un tiret n'est pas un caractère de nom de base sans guillemets.
        assert_eq!(
            options.database_url,
            "postgres://postgres:postgres@localhost:5432/mon_projet"
        );
    }

    #[test]
    fn l_url_en_flag_prend_le_pas_sur_le_defaut() {
        let espion = Espion::default();

        let options = resoudre_avec(
            &espion,
            None,
            Some("postgres://ailleurs:5432/db".to_string()),
            None,
            true,
        )
        .unwrap();

        assert!(espion.posees().is_empty());
        assert_eq!(options.database_url, "postgres://ailleurs:5432/db");
    }

    #[test]
    fn les_features_en_flag_prennent_le_pas_sur_le_defaut() {
        let espion = Espion::default();

        let options = resoudre_avec(
            &espion,
            None,
            None,
            Some(vec!["docker".to_string(), "ci".to_string()]),
            true,
        )
        .unwrap();

        assert!(espion.posees().is_empty());
        assert_eq!(options.features, ["docker", "ci"]);
    }

    #[test]
    fn sans_yes_chaque_valeur_absente_devient_une_question() {
        let espion = Espion::default();

        let options = resoudre_avec(&espion, None, None, None, false).unwrap();

        assert_eq!(espion.posees(), ["nom", "database_url", "features"]);
        assert_eq!(options.nom, "repondu");
        assert_eq!(options.database_url, "postgres://repondu");
        assert_eq!(options.features, ["repondu"]);
    }

    #[test]
    fn sans_yes_un_flag_fourni_evite_sa_question() {
        let espion = Espion::default();

        resoudre_avec(&espion, Some("api".to_string()), None, None, false).unwrap();

        assert_eq!(espion.posees(), ["database_url", "features"]);
    }

    #[test]
    fn sans_terminal_l_erreur_dit_comment_s_en_passer() {
        let message = traduire(inquire::InquireError::NotTTY).to_string();

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
    fn une_interruption_ne_se_confond_pas_avec_l_absence_de_terminal() {
        let interrompu = traduire(inquire::InquireError::OperationInterrupted);

        assert_eq!(interrompu, ErreurPrompt::Interrompu);
    }
}
