//! Mise en forme du rapport de diagnostic.
//!
//! Un remède se lit sous le constat qui l'appelle, indenté : un diagnostic qui renvoie
//! ses remèdes en bas de page oblige à faire l'aller-retour.

use crate::ui;

use super::{Report, State};

/// Retrait des remèdes, aligné sous le détail des constats.
const RETRAIT: &str = "      ";

/// Rend le rapport, un contrôle par ligne, remèdes compris.
pub(crate) fn report(report: &Report) -> String {
    let width = report
        .checks
        .iter()
        .map(|check| check.title.chars().count())
        .max()
        .unwrap_or(0);

    let mut lines = Vec::new();

    for check in &report.checks {
        let title = format!("{:width$}", check.title);

        lines.push(match check.state {
            State::Bon => format!("  {} {title}   {}", ui::green("✓"), check.detail),
            State::Echec => format!("  {} {title}   {}", ui::red("✗"), check.detail),
        });

        if let Some(remedy) = &check.remedy {
            lines.extend(
                remedy
                    .lines()
                    .map(|line| format!("{RETRAIT}{}", ui::dimmed(line))),
            );
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::Check;
    use super::*;

    fn report_of(checks: Vec<Check>) -> Report {
        Report { checks }
    }

    #[test]
    fn the_two_verdicts_carry_distinct_markers_without_colour() {
        let rendered = report(&report_of(vec![
            Check::ok("ancres", "les 5 sont en place"),
            Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]));
        let mut lines = rendered.lines();

        let ok = lines.next().expect("le premier contrôle est rendu");
        let failed = lines.next().expect("le second contrôle est rendu");

        assert!(ok.contains('✓') && ok.contains("ancres"));
        assert!(failed.contains('✗') && failed.contains(".env"));
        assert!(!ok.contains('✗'));
    }

    #[test]
    fn the_remedy_follows_its_finding_indented() {
        let rendered = report(&report_of(vec![Check::failed(
            ".env",
            "RBS_ENV manque",
            "ajoutez RBS_ENV=development",
        )]));

        let remedy = rendered
            .lines()
            .find(|line| line.contains("ajoutez RBS_ENV=development"))
            .expect("le remède est rendu");

        assert!(
            remedy.starts_with("      "),
            "le remède est en retrait du constat : « {remedy} »"
        );
    }

    #[test]
    fn a_multi_line_remedy_is_indented_throughout() {
        let rendered = report(&report_of(vec![Check::failed(
            "ancres",
            "routes manque",
            "dans src/router.rs :\n// <rbs:routes>\n// </rbs:routes>",
        )]));

        for line in rendered.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            assert!(
                line.starts_with("      "),
                "chaque ligne du remède est en retrait : « {line} »"
            );
        }
    }

    #[test]
    fn a_spotless_check_adds_no_line() {
        let rendered = report(&report_of(vec![Check::ok("ancres", "les 5 sont là")]));

        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn the_details_align_on_the_longest_title() {
        let rendered = report(&report_of(vec![
            Check::ok("base", "PostgreSQL 18.1"),
            Check::ok("versions", "alignées"),
        ]));

        let column = |line: &str, detail: &str| {
            let octets = line.find(detail).expect("le détail est présent");
            line[..octets].chars().count()
        };

        let mut lines = rendered.lines();
        let premiere = lines.next().expect("première ligne");
        let seconde = lines.next().expect("seconde ligne");

        assert_eq!(
            column(premiere, "PostgreSQL 18.1"),
            column(seconde, "alignées")
        );
    }
}
