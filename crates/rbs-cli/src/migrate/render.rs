//! Mise en forme de l'inventaire des migrations.
//!
//! C'est ici que vit la lisibilité de `rbs migrate status` : la crate `migration` du
//! projet ne rapporte qu'un état par ligne, sans se soucier de l'affichage.

use super::state::State;
use crate::ui;

const APPLIQUEE: &str = "appliquée";
const EN_ATTENTE: &str = "en attente";

/// Rend l'inventaire des migrations, une par ligne.
///
/// Les deux états se distinguent par leur puce et par leur libellé, jamais par la seule
/// couleur : la sortie reste lisible dans un `less`, un fichier de log ou une CI.
pub fn status(states: &[State]) -> String {
    if states.is_empty() {
        return "  aucune migration déclarée".to_string();
    }

    let width = states
        .iter()
        .map(|state| state.name.chars().count())
        .max()
        .unwrap_or(0);

    states
        .iter()
        .map(|state| line(state, width))
        .collect::<Vec<_>>()
        .join("\n")
}

fn line(state: &State, width: usize) -> String {
    let name = format!("{:width$}", state.name);

    if state.applied {
        format!("  {} {name}   {}", ui::green("✓"), ui::green(APPLIQUEE))
    } else {
        format!("  {} {name}   {}", ui::dimmed("·"), ui::dimmed(EN_ATTENTE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Colonne d'affichage d'un libellé, comptée en caractères : `find` rend des octets,
    /// et les deux puces n'en occupent pas le même nombre.
    fn column(line: &str, libelle: &str) -> usize {
        let octets = line.find(libelle).expect("le libellé est présent");
        line[..octets].chars().count()
    }

    fn states() -> Vec<State> {
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
    }

    #[test]
    fn the_two_states_carry_distinct_markers_without_colour() {
        let rendered = status(&states());
        let mut lines = rendered.lines();

        let applied = lines.next().expect("la première migration est rendue");
        let en_attente = lines.next().expect("la seconde migration est rendue");

        assert!(applied.contains('✓') && applied.contains("appliquée"));
        assert!(en_attente.contains('·') && en_attente.contains("en attente"));
        assert!(!applied.contains("en attente"));
        assert!(!en_attente.contains('✓'));
    }

    #[test]
    fn each_migration_is_named() {
        let rendered = status(&states());

        assert!(rendered.contains("m20260826_120000_creer_carnets"));
        assert!(rendered.contains("m20260826_133000_creer_notes"));
    }

    #[test]
    fn the_labels_align_on_the_longest_name() {
        let rendered = status(&[
            State {
                name: "m1_court".to_string(),
                applied: true,
            },
            State {
                name: "m2_un_nom_nettement_plus_long".to_string(),
                applied: false,
            },
        ]);

        let mut lines = rendered.lines();
        let applied = lines.next().expect("la première ligne est rendue");
        let en_attente = lines.next().expect("la seconde ligne est rendue");

        assert_eq!(
            column(applied, APPLIQUEE),
            column(en_attente, EN_ATTENTE),
            "les libellés commencent à la même colonne"
        );
    }

    #[test]
    fn a_project_without_a_migration_says_so_rather_than_returning_nothing() {
        let rendered = status(&[]);

        assert!(rendered.contains("aucune migration"));
    }
}
