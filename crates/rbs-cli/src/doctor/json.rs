//! Rendu machine du rapport de diagnostic.
//!
//! Le code de sortie dit qu'il y a quelque chose ; il ne dit pas quoi. Un script qui veut
//! le savoir n'a pas à lire des glyphes colorés.

use serde::Serialize;

use super::anchors::{Laissee, Repair};
use super::{Check, Report};

/// Le rapport tel qu'un script le lit.
#[derive(Serialize)]
struct Document<'a> {
    /// Le verdict d'ensemble, celui-là même que porte le code de sortie.
    ///
    /// Sans lui, un lecteur devrait le recalculer sur le tableau, en sachant qu'un
    /// avertissement n'y fait pas obstacle — règle qu'aucun champ du document n'énonce.
    sain: bool,
    /// Ce que `--fix` a reposé, et ce qu'il a laissé. Absent sans `--fix`.
    ///
    /// Séparé du tableau des contrôles : `ancres` y dit l'état du projet *après* la
    /// réparation, et un script qui veut savoir si quelque chose a été écrit n'a pas à le
    /// déduire d'un verdict devenu vert.
    #[serde(rename = "reparation", skip_serializing_if = "Option::is_none")]
    repair: Option<Reparation<'a>>,
    /// Les constats, dans l'ordre où ils ont été faits.
    checks: &'a [Check],
}

/// Ce qu'une réparation a fait, tel qu'un script le lit.
#[derive(Serialize)]
struct Reparation<'a> {
    /// Les noms des ancres reposées.
    reposees: &'a [String],
    /// Les ancres laissées absentes, et pourquoi.
    laissees: &'a [Laissee],
}

/// Rend le rapport en JSON, seul document de la sortie standard.
pub(crate) fn report(report: &Report, repair: Option<&Repair>) -> String {
    let document = Document {
        sain: report.succeeded(),
        repair: repair.map(|repair| Reparation {
            reposees: &repair.reposees,
            laissees: &repair.laissees,
        }),
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
        let rendu = report(&Report { checks }, None);

        serde_json::from_str(&rendu)
            .unwrap_or_else(|faute| panic!("le rendu doit être un JSON valide ({faute}) : {rendu}"))
    }

    /// Sans `--fix`, aucun objet de réparation : un lecteur aurait à le filtrer alors
    /// qu'aucune réparation n'a été demandée.
    #[test]
    fn a_report_without_a_repair_carries_no_repair_object() {
        let document = document(vec![Check::ok("ancres", "les 11 sont en place")]);

        assert!(document.get("reparation").is_none(), "{document}");
    }

    /// Avec `--fix`, le document dit ce qui a été reposé et ce qui ne l'a pas été : le
    /// tableau des contrôles, lui, décrit le projet *après* la réparation, et un `ancres`
    /// devenu vert ne dirait pas qu'un octet a été écrit.
    #[test]
    fn a_repair_names_what_it_put_back_and_what_it_left() {
        let repair = Repair {
            plan: crate::plan::Builder::new("/aucun-projet-ici").finir(),
            reposees: vec!["routes".to_string()],
            laissees: vec![Laissee {
                anchor: "services".to_string(),
                raison: "la ligne d'accroche `services:` est introuvable".to_string(),
            }],
        };
        let rendu = report(
            &Report {
                checks: vec![Check::ok("ancres", "les 11 sont en place")],
            },
            Some(&repair),
        );
        let document: serde_json::Value =
            serde_json::from_str(&rendu).expect("le rendu doit être un JSON valide");

        assert_eq!(document["reparation"]["reposees"][0], "routes");
        assert_eq!(document["reparation"]["laissees"][0]["ancre"], "services");
        assert!(
            document["reparation"]["laissees"][0]["raison"]
                .as_str()
                .is_some_and(|raison| raison.contains("services:")),
            "{document}"
        );
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
