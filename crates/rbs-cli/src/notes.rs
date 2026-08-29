//! Les notes de migration : ce qu'une version rompt, dit à qui la traverse.
//!
//! Elles vivent dans la crate qui les embarque, comme les templates : `cargo package`
//! n'emporte aucun fichier extérieur au paquet, et `include = [...]` ne lève pas la règle.
//!
//! Leur longueur mesure la qualité du gel de l'API : une note longue dirait que le noyau
//! a été mal figé.

use include_dir::{Dir, include_dir};

use crate::upgrade;

/// Les notes embarquées, une par version qui rompt quelque chose.
static NOTES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/notes");

/// Les notes que le saut `depuis` → `vers` traverse, de la plus ancienne à la plus récente.
///
/// Une note appartient à la version qu'elle introduit, et non à un couple de versions :
/// une mise à niveau qui enjambe plusieurs versions les affiche toutes, sinon la rupture
/// d'une version intermédiaire disparaîtrait au seul motif qu'on ne s'y est pas arrêté.
pub(crate) fn traversees(depuis: &str, vers: &str) -> Vec<&'static str> {
    let mut notes: Vec<([u64; 3], &'static str)> = NOTES
        .files()
        .filter_map(|file| {
            let version = file.path().file_stem()?.to_str()?;

            if !upgrade::posterieure(version, depuis) || upgrade::posterieure(version, vers) {
                return None;
            }

            Some((upgrade::nombres(version)?, file.contents_utf8()?))
        })
        .collect();

    notes.sort_by_key(|(numeros, _)| *numeros);

    notes.into_iter().map(|(_, contenu)| contenu).collect()
}

// La complétude du catalogue n'a rien à vérifier à l'exécution : elle se contrôle avant
// de publier, et c'est la CI qui la fait respecter. Le contrôle vit donc avec son test.
#[cfg(test)]
mod tests {
    use super::*;

    /// Version du CLI qui met à niveau.
    const CLI: &str = env!("CARGO_PKG_VERSION");

    /// Dernière version de rbs publiée sur crates.io.
    ///
    /// Elle ne se lit pas dans le dépôt : entre deux publications, le workspace porte
    /// déjà le numéro à venir. Comme `NOYAU_PUBLIE` du diagnostic, la constante bascule
    /// au moment de publier — et c'est cette bascule qui met le catalogue en demeure.
    const PUBLIEE: &str = "1.0.0";

    /// Suffixe d'une note ; son radical est la version qu'elle introduit.
    const SUFFIXE: &str = "md";

    /// Le nom de la note qui manque au catalogue avant de publier `cli`, s'il en manque
    /// une.
    ///
    /// Une version qui rompt sans le dire est une version dont le développeur découvre la
    /// rupture à la compilation. Le saut depuis la dernière version publiée doit donc
    /// toujours porter la sienne, fût-elle de deux lignes.
    ///
    /// Les deux versions sont des paramètres : sans cela, le contrôle passerait à vide
    /// tant que le workspace et la dernière publication portent le même numéro, et un
    /// test vert qui n'a rien examiné ne prouve rien.
    fn manquante(publiee: &str, cli: &str) -> Option<String> {
        let attendue = format!("{cli}.{SUFFIXE}");

        (publiee != cli && NOTES.get_file(&attendue).is_none()).then_some(attendue)
    }

    /// Le test qui empêche le catalogue de pourrir en silence.
    #[test]
    fn the_jump_since_the_last_published_version_carries_its_note() {
        assert_eq!(
            manquante(PUBLIEE, CLI),
            None,
            "rbs {PUBLIEE} → {CLI} romprait sans le dire : écrivez crates/rbs-cli/notes/{CLI}.{SUFFIXE}"
        );
    }

    #[test]
    fn a_missing_note_is_named_by_the_file_it_expects() {
        assert_eq!(manquante("0.4.0", "2.0.0").as_deref(), Some("2.0.0.md"));
    }

    /// Deux numéros égaux ne sont pas un saut : rien n'est exigé entre deux publications.
    #[test]
    fn a_version_that_is_not_a_jump_expects_nothing() {
        assert_eq!(manquante(CLI, CLI), None);
    }

    #[test]
    fn the_jump_to_1_0_0_tells_the_freeze() {
        let notes = traversees("0.4.0", "1.0.0");

        assert_eq!(notes.len(), 1, "une seule note sur ce saut");
        assert!(notes[0].contains("non_exhaustive"), "{}", notes[0]);
        assert!(notes[0].contains("`_ =>`"), "{}", notes[0]);
    }

    #[test]
    fn a_jump_that_stops_short_of_a_note_shows_nothing() {
        assert!(traversees("0.4.0", "0.9.0").is_empty());
    }

    /// Un saut qui enjambe une version en montre quand même la note : sans cela, la
    /// rupture d'une version intermédiaire disparaîtrait au motif qu'on ne s'y est pas
    /// arrêté.
    ///
    /// Le compte se dérive du catalogue plutôt que d'être figé : chaque publication en
    /// ajoute une, et un nombre écrit ici serait faux dès la version suivante.
    #[test]
    fn a_jump_that_steps_over_a_version_still_shows_its_note() {
        let depuis_le_debut = traversees("0.0.1", "99.0.0").len();

        assert!(depuis_le_debut > 0, "le catalogue n'est jamais vide");
        assert_eq!(traversees("0.4.0", "99.0.0").len(), depuis_le_debut);
    }

    /// Le projet est déjà passé par la version : sa note ne le concerne plus.
    #[test]
    fn a_project_already_past_the_note_does_not_see_it_again() {
        let toutes = traversees("0.0.1", "99.0.0").len();

        assert_eq!(
            traversees("1.0.0", "99.0.0").len(),
            toutes - 1,
            "seule la note de 1.0.0 doit tomber"
        );
    }
}
