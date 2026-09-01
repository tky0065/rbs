//! Rendu machine du rapport de diagnostic.
//!
//! Le code de sortie dit qu'il y a quelque chose ; il ne dit pas quoi. Un script qui veut
//! le savoir n'a pas à lire des glyphes colorés.

use serde::Serialize;

use super::{Check, Report};

/// Le rapport tel qu'un script le lit.
#[derive(Serialize)]
struct Document<'a> {
    /// Le verdict d'ensemble, celui-là même que porte le code de sortie.
    ///
    /// Sans lui, un lecteur devrait le recalculer sur le tableau, en sachant qu'un
    /// avertissement n'y fait pas obstacle — règle qu'aucun champ du document n'énonce.
    sain: bool,
    /// Les constats, dans l'ordre où ils ont été faits.
    checks: &'a [Check],
}

/// Rend le rapport en JSON, seul document de la sortie standard.
pub(crate) fn report(report: &Report) -> String {
    let document = Document {
        sain: report.succeeded(),
        checks: &report.checks,
    };

    // Ni carte à clés non textuelles ni flottant : la sérialisation ne peut échouer que
    // sur un défaut de programmation, qu'il vaut mieux voir tomber ici.
    serde_json::to_string_pretty(&document).expect("le rapport se sérialise")
}

/// Le puits du mode `--json`, qui ne dit rien pendant le diagnostic.
///
/// La sortie standard ne doit porter que le document : une ligne de rapport ou une
/// annonce d'attente y ferait échouer le premier `jq` venu.
pub(crate) struct Muette;

impl super::Sortie for Muette {
    fn debut(&mut self, _titres: &[&'static str]) {}

    fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

    fn constat(&mut self, _check: &Check) {}
}

#[cfg(test)]
mod tests {
    use super::super::Check;
    use super::*;

    /// Le document, analysé comme un script l'analyserait.
    fn document(checks: Vec<Check>) -> serde_json::Value {
        let rendu = report(&Report { checks });

        serde_json::from_str(&rendu)
            .unwrap_or_else(|faute| panic!("le rendu doit être un JSON valide ({faute}) : {rendu}"))
    }

    /// Les trois verdicts doivent se distinguer : un script qui ne voit que « pas ok »
    /// ne sait pas s'il doit arrêter sa chaîne.
    #[test]
    fn the_three_verdicts_carry_distinct_statuses() {
        let document = document(vec![
            Check::ok("ancres", "les 11 sont en place"),
            Check::warned("agents", "écrit hors du CLI : webhooks", "rien à faire"),
            Check::failed(".env", "RBS_ENV absente", "ajoutez RBS_ENV=development"),
        ]);

        let statuts: Vec<&str> = document["checks"]
            .as_array()
            .expect("checks est un tableau")
            .iter()
            .map(|check| check["status"].as_str().expect("un statut textuel"))
            .collect();

        assert_eq!(statuts, vec!["ok", "avertissement", "erreur"]);
    }

    /// Le nom et le détail sont ce qui permet de savoir *quel* contrôle a échoué.
    #[test]
    fn each_check_names_itself_and_what_it_found() {
        let document = document(vec![Check::failed(
            "base",
            "rien ne répond sur localhost:5432",
            "lancez `docker compose up -d`",
        )]);
        let check = &document["checks"][0];

        assert_eq!(check["name"], "base");
        assert_eq!(check["detail"], "rien ne répond sur localhost:5432");
        assert_eq!(check["remede"], "lancez `docker compose up -d`");
    }

    /// Un remède absent ne se rend pas en `null` : chaque lecteur aurait à le filtrer.
    #[test]
    fn a_check_without_a_remedy_carries_no_remedy_field() {
        let document = document(vec![Check::ok("ancres", "les 11 sont en place")]);

        assert!(
            document["checks"][0].get("remede").is_none(),
            "{}",
            document["checks"][0]
        );
    }

    /// `sain` vaut exactement ce que vaut le code de sortie : un avertissement n'y fait
    /// pas obstacle, un échec si.
    #[test]
    fn the_summary_follows_the_exit_status() {
        let avertissement = document(vec![Check::warned(
            "agents",
            "1 module hors CLI",
            "rien à faire",
        )]);
        assert_eq!(avertissement["sain"], true);

        let echec = document(vec![Check::failed(
            ".env",
            "RBS_ENV absente",
            "ajoutez RBS_ENV=development",
        )]);
        assert_eq!(echec["sain"], false);
    }
}
