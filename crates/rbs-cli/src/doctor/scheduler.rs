//! Contrôle de la feature `scheduler`.
//!
//! Le fragment compile chaque expression de son calendrier au démarrage, et une expression
//! illisible l'arrête — c'est voulu : détachée, elle laisserait l'API répondre avec un
//! calendrier qui ne déclenchera jamais rien. Ce contrôle dit la même chose à froid, avant
//! qu'un déploiement ne la découvre, et avec le verdict même du démarrage : `crate::cron`
//! emploie la crate et la normalisation du fragment.
//!
//! Seules les expressions littérales se lisent sans compiler : une expression tirée d'une
//! constante reste au jugement du démarrage.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "scheduler";
const FICHIER: &str = "src/modules/scheduler/mod.rs";
const APPEL: &str = "Schedule::every::<";

/// Vérifie que chaque expression littérale du calendrier se lit.
pub(crate) fn check(root: &Path) -> Check {
    let source = match super::lire(root, TITRE, FICHIER) {
        Ok(source) => source,
        Err(constat) => return constat,
    };

    let expressions = expressions(&source);
    let fautes: Vec<String> = expressions
        .iter()
        .filter_map(|expression| crate::cron::valider(expression).err())
        .map(super::une_ligne)
        .collect();

    if !fautes.is_empty() {
        return Check::failed(
            TITRE,
            fautes.join(" ; "),
            format!(
                "corrigez-les dans {FICHIER} : cinq champs (minute heure jour mois \
                 jour-de-semaine) ou six, la seconde en tête — le démarrage s'arrête sur la \
                 première qu'il ne lit pas"
            ),
        );
    }

    let constat = match expressions.len() {
        0 => "aucune expression littérale au calendrier".to_string(),
        1 => "1 expression du calendrier se lit".to_string(),
        nombre => format!("{nombre} expressions du calendrier se lisent"),
    };

    Check::ok(TITRE, constat)
}

/// Les expressions littérales passées à `Schedule::every`, dans l'ordre du fichier.
///
/// Les lignes de commentaire sont écartées d'abord : un exemple cité dans une doc n'est pas
/// une échéance. Les chevrons sont comptés, le type d'un job pouvant porter les siens.
fn expressions(source: &str) -> Vec<String> {
    let code = source
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut trouvees = Vec::new();
    let mut reste = code.as_str();

    while let Some(debut) = reste.find(APPEL) {
        reste = &reste[debut + APPEL.len()..];

        let mut profondeur = 1;
        let Some(fin) = reste.char_indices().find_map(|(rang, caractere)| {
            match caractere {
                '<' => profondeur += 1,
                '>' => profondeur -= 1,
                _ => {}
            }
            (profondeur == 0).then_some(rang)
        }) else {
            break;
        };
        reste = &reste[fin + 1..];

        if let Some(argument) = reste.strip_prefix('(')
            && let Some(litteral) = argument.trim_start().strip_prefix('"')
            && let Some(longueur) = litteral.find('"')
        {
            trouvees.push(litteral[..longueur].to_string());
        }
    }

    trouvees
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Le calendrier tel que le fragment le livre : sa template ne porte aucune directive.
    const LIVRE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/features/scheduler/mod.rs.jinja"
    ));

    /// Un projet réduit au fichier que ce contrôle lit.
    fn projet(source: &str) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        let fichier = racine.path().join(FICHIER);
        fs::create_dir_all(fichier.parent().expect("le fichier a un parent"))
            .expect("répertoire du module créable");
        fs::write(&fichier, source).expect("fichier du module inscriptible");

        racine
    }

    #[test]
    fn the_calendar_the_fragment_ships_reads() {
        let racine = projet(LIVRE);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(check.detail.contains("1 expression"), "{}", check.detail);
    }

    /// Le défaut que le démarrage découvrirait : l'expression est nommée, avec le refus de
    /// la crate, plutôt qu'un projet qui s'arrête au boot.
    #[test]
    fn an_unparsable_expression_is_named_with_the_refusal_of_the_crate() {
        let racine = projet(&LIVRE.replace("\"0 3 * * *\"", "\"0 99 * * *\""));

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("`0 99 * * *`"), "{}", check.detail);
        assert!(!check.detail.contains('\n'), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains(FICHIER)),
            "{:?}",
            check.remedy
        );
    }

    #[test]
    fn a_missing_file_is_named_and_restored_from_git() {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains("git checkout")),
            "{:?}",
            check.remedy
        );
    }

    /// Un appel en commentaire n'est pas une échéance : le démarrage ne le lira jamais.
    #[test]
    fn a_commented_out_call_is_not_read() {
        let racine = projet("// Schedule::every::<Job>(\"n'importe quoi\", fabrique)\n");

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// rustfmt pose l'expression sur la ligne qui suit l'appel dès qu'il ne tient plus sur
    /// une seule.
    #[test]
    fn an_expression_on_the_line_after_the_call_is_read() {
        let racine = projet(
            "    calendrier.push(Schedule::every::<Job>(\n        \"0 99 * * *\",\n        || Job {},\n    ));\n",
        );

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("`0 99 * * *`"), "{}", check.detail);
    }

    /// Les chevrons du type visé ne ferment pas ceux de l'appel.
    #[test]
    fn a_job_type_with_its_own_angle_brackets_is_read() {
        let racine = projet("Schedule::every::<Enveloppe<Job>>(\"0 99 * * *\", fabrique)\n");

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("`0 99 * * *`"), "{}", check.detail);
    }

    /// Toutes les fautes à la fois : un diagnostic qui n'en nomme qu'une oblige à le
    /// relancer autant de fois qu'il y en a.
    #[test]
    fn every_faulty_expression_is_named_on_one_line() {
        let racine =
            projet("Schedule::every::<A>(\"1 2 3\", a)\nSchedule::every::<B>(\"0 99 * * *\", b)\n");

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("`1 2 3`"), "{}", check.detail);
        assert!(check.detail.contains("`0 99 * * *`"), "{}", check.detail);
        assert!(check.detail.contains(" ; "), "{}", check.detail);
        assert!(!check.detail.contains('\n'), "{}", check.detail);
    }

    /// Une expression tirée d'une constante ne se lit pas sans compiler : le contrôle ne
    /// juge que les littéraux, et ne condamne pas ce qu'il ne voit pas.
    #[test]
    fn a_non_literal_expression_is_left_to_the_startup() {
        let racine = projet("Schedule::every::<Job>(EXPRESSION, fabrique)\n");

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }
}
