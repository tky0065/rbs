//! Mise en forme du rapport de diagnostic, écrite au fil des contrôles.
//!
//! Un remède se lit sous le constat qui l'appelle, indenté : un diagnostic qui renvoie
//! ses remèdes en bas de page oblige à faire l'aller-retour.
//!
//! Le rapport n'est pas assemblé puis affiché d'un bloc : il s'écrit constat par constat,
//! ce qui est la condition pour qu'un contrôle sur le point de bloquer une minute puisse
//! le dire avant, et non après.

use std::io::Write;

use crate::ui;

use super::{Check, Sortie, State};

/// Retrait des remèdes, sous le constat qui les appelle.
const RETRAIT: &str = "      ";

/// Ce qui sépare le marqueur, le titre et le détail : `  ✓ `, puis `   `.
const ENTOUR: usize = 7;

/// Rendu texte, écrit dans `sortie` à mesure que les contrôles rendent leur verdict.
pub(crate) struct Texte<W: Write> {
    sortie: W,
    /// Largeur de la colonne des titres, fixée par [`Sortie::debut`].
    width: usize,
}

impl<W: Write> Texte<W> {
    /// Un rendu qui écrit dans `sortie`.
    pub(crate) fn new(sortie: W) -> Self {
        Self { sortie, width: 0 }
    }

    /// Écrit une ligne, l'échec d'écriture laissé de côté.
    ///
    /// Une sortie fermée — `rbs doctor | head -3` — n'est pas une faute du projet
    /// diagnostiqué, et s'interrompre pour le dire perdrait les contrôles restants.
    fn ligne(&mut self, ligne: &str) {
        let _ = writeln!(self.sortie, "{ligne}");
    }

    /// Le début d'une ligne de constat : marqueur, titre, et l'espace jusqu'au détail.
    fn tete(&self, marqueur: &str, titre: &str) -> String {
        let width = self.width;

        format!("  {marqueur} {titre:width$}   ")
    }
}

impl<W: Write> Sortie for Texte<W> {
    fn debut(&mut self, titres: &[&'static str]) {
        self.width = titres
            .iter()
            .map(|titre| titre.chars().count())
            .max()
            .unwrap_or(0);
    }

    fn annonce(&mut self, titre: &'static str, raison: &str) {
        let mut raisons = raison.lines();
        let premiere = raisons.next().unwrap_or_default();
        let tete = self.tete(&ui::dimmed("…"), titre);
        self.ligne(&format!("{tete}{}", ui::dimmed(premiere)));

        let colonne = " ".repeat(self.width + ENTOUR);
        for suite in raisons {
            self.ligne(&format!("{colonne}{}", ui::dimmed(suite)));
        }

        // Une annonce qui arriverait après le travail qu'elle annonce n'annoncerait
        // rien : elle ne doit pas attendre le tampon d'un appelant.
        let _ = self.sortie.flush();
    }

    fn constat(&mut self, check: &Check) {
        let marqueur = match check.state {
            State::Bon => ui::green("✓"),
            State::Avertissement => ui::yellow("!"),
            State::Echec => ui::red("✗"),
        };
        let tete = self.tete(&marqueur, check.title);
        self.ligne(&format!("{tete}{}", check.detail));

        let Some(remedy) = &check.remedy else {
            return;
        };

        for ligne in remedy
            .lines()
            .map(|line| format!("{RETRAIT}{}", ui::dimmed(line)))
        {
            self.ligne(&ligne);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Check;
    use super::*;

    /// Le rendu texte d'un rapport, écrit par le puits contrôle par contrôle.
    fn rendu(checks: Vec<Check>) -> String {
        let mut octets = Vec::new();
        let titres: Vec<&'static str> = checks.iter().map(|check| check.title).collect();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&titres);
            for check in &checks {
                texte.constat(check);
            }
        }

        String::from_utf8(octets).expect("le rendu est de l'UTF-8")
    }

    #[test]
    fn the_two_verdicts_carry_distinct_markers_without_colour() {
        let rendered = rendu(vec![
            Check::ok("ancres", "les 5 sont en place"),
            Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]);
        let mut lines = rendered.lines();

        let ok = lines.next().expect("le premier contrôle est rendu");
        let failed = lines.next().expect("le second contrôle est rendu");

        assert!(ok.contains('✓') && ok.contains("ancres"));
        assert!(failed.contains('✗') && failed.contains(".env"));
        assert!(!ok.contains('✗'));
    }

    /// Le rapport se lit aussi sans couleur — journaux de CI, terminaux monochromes.
    #[test]
    fn the_three_verdicts_carry_distinct_markers_without_colour() {
        let rendered = rendu(vec![
            Check::ok("ancres", "les 10 sont en place"),
            Check::warned("cli", "1 module hors CLI", "rbs generate, ou rien"),
            Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]);
        let mut lines = rendered.lines();

        let ok = lines.next().expect("le premier contrôle est rendu");
        let warned = lines.next().expect("le deuxième contrôle est rendu");

        assert!(ok.contains('✓'));
        assert!(warned.contains('!') && warned.contains("cli"));
        assert!(!warned.contains('✓') && !warned.contains('✗'));
    }

    #[test]
    fn the_remedy_follows_its_finding_indented() {
        let rendered = rendu(vec![Check::failed(
            ".env",
            "RBS_ENV manque",
            "ajoutez RBS_ENV=development",
        )]);

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
        let rendered = rendu(vec![Check::failed(
            "ancres",
            "routes manque",
            "dans src/router.rs :\n// <rbs:routes>\n// </rbs:routes>",
        )]);

        for line in rendered.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            assert!(
                line.starts_with("      "),
                "chaque ligne du remède est en retrait : « {line} »"
            );
        }
    }

    #[test]
    fn a_spotless_check_adds_no_line() {
        let rendered = rendu(vec![Check::ok("ancres", "les 5 sont là")]);

        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn the_details_align_on_the_longest_title() {
        let rendered = rendu(vec![
            Check::ok("base", "PostgreSQL 18.1"),
            Check::ok("versions", "alignées"),
        ]);

        let mut lines = rendered.lines();
        let premiere = lines.next().expect("première ligne");
        let seconde = lines.next().expect("seconde ligne");

        assert_eq!(
            colonne(premiere, "PostgreSQL 18.1"),
            colonne(seconde, "alignées")
        );
    }

    /// La colonne, en caractères, où `detail` commence dans `line`.
    fn colonne(line: &str, detail: &str) -> usize {
        let octets = line.find(detail).expect("le détail est présent");

        line[..octets].chars().count()
    }

    /// L'annonce n'a de sens que devant le constat du contrôle qu'elle annonce : c'est
    /// tout son objet, et le rendu en bloc l'interdisait.
    #[test]
    fn the_announcement_precedes_the_finding_of_its_own_check() {
        let mut octets = Vec::new();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&["ancres", "base"]);
            texte.constat(&Check::ok("ancres", "les 11 sont en place"));
            texte.annonce("base", "compilation de la crate migration");
            texte.constat(&Check::ok("base", "postgres 18.6 répond"));
        }

        let rendered = String::from_utf8(octets).expect("le rendu est de l'UTF-8");
        let lignes: Vec<&str> = rendered.lines().collect();
        let annonce = lignes
            .iter()
            .position(|line| line.contains("compilation de la crate migration"))
            .expect("l'annonce est rendue");
        let constat = lignes
            .iter()
            .position(|line| line.contains("postgres 18.6 répond"))
            .expect("le constat est rendu");

        assert!(annonce < constat, "{rendered}");
        assert!(lignes[annonce].contains("base"), "{rendered}");
    }

    /// La suite d'une annonce se lit sous son propre début, et non sous le marqueur.
    #[test]
    fn a_multi_line_announcement_aligns_on_the_detail_column() {
        let mut octets = Vec::new();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&["ancres", "base"]);
            texte.annonce("base", "compilation de la crate migration,\nune minute…");
        }

        let rendered = String::from_utf8(octets).expect("le rendu est de l'UTF-8");
        let mut lignes = rendered.lines();
        let premiere = lignes.next().expect("la première ligne de l'annonce");
        let suite = lignes.next().expect("la suite de l'annonce");

        assert_eq!(
            colonne(premiere, "compilation de la crate migration,"),
            colonne(suite, "une minute…")
        );
    }
}
