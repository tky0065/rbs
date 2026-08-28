//! État d'une migration, tel que rapporté par la crate `migration` du projet.
//!
//! Le binaire du projet écrit un état par ligne, `applied` ou `pending` suivi du nom.
//! Ce format existe pour être analysé : la mise en forme lisible est le travail de rbs,
//! qui la contrôle ainsi sans dépendre de ce que le projet imprime.

/// Une migration et son état vis-à-vis de la base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Nom du module de migration, tel que déclaré dans `migration/src/lib.rs`.
    pub name: String,
    /// Vrai si la migration est déjà appliquée sur la base visée.
    pub applied: bool,
}

/// Ce qui peut empêcher de comprendre la sortie du sous-processus.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Une ligne ne suit pas le format attendu.
    #[error("la crate migration a répondu une ligne incomprise : {line}")]
    Ligne {
        /// La ligne fautive, telle que reçue.
        line: String,
    },
}

/// Analyse la sortie du binaire de migration du projet.
pub fn parse(output: &str) -> Result<Vec<State>, Error> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_line)
        .collect()
}

fn parse_line(line: &str) -> Result<State, Error> {
    let incomprise = || Error::Ligne {
        line: line.to_string(),
    };

    let (state, name) = line.trim_end().split_once('\t').ok_or_else(incomprise)?;

    let applied = match state {
        "applied" => true,
        "pending" => false,
        _ => return Err(incomprise()),
    };

    Ok(State {
        name: name.to_string(),
        applied,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_states_are_recognised_in_order() {
        let states = parse(
            "applied\tm20260826_120000_creer_carnets\npending\tm20260826_133000_creer_notes\n",
        )
        .expect("la sortie est bien formée");

        assert_eq!(
            states,
            vec![
                State {
                    name: "m20260826_120000_creer_carnets".to_string(),
                    applied: true,
                },
                State {
                    name: "m20260826_133000_creer_notes".to_string(),
                    applied: false,
                },
            ]
        );
    }

    #[test]
    fn an_empty_output_yields_no_migration() {
        assert_eq!(parse("\n").expect("le vide est valide"), vec![]);
    }

    #[test]
    fn an_unparsed_line_is_reported_with_its_content() {
        let error = parse("error: connexion refusée\n").expect_err("la ligne est invalide");

        assert!(error.to_string().contains("error: connexion refusée"));
    }

    #[test]
    fn an_unknown_state_is_rejected_rather_than_assumed_pending() {
        let error = parse("skipped\tm20260826_120000_creer_carnets\n")
            .expect_err("`skipped` n'est pas un état connu");

        assert!(error.to_string().contains("skipped"));
    }
}
