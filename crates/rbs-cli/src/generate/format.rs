//! Passage du rendu par rustfmt, avant écriture.
//!
//! Les templates posent des lignes dont la longueur dépend du nom de la feature, et
//! rustfmt bascule à 100 colonnes : aucune forme écrite en dur n'est juste pour tous les
//! noms. La signature de `list` mesure `95 + len(entity)` — elle tient sur une ligne pour
//! `Tag`, elle déborde pour `AdministrativeDocument`. Écrire ce que rustfmt écrirait, sans
//! l'appeler, revient à réimplanter sa règle dans le Jinja et à ne la prouver que pour les
//! noms qu'un test cite.
//!
//! Le projet généré porte un `cargo fmt --check` dès que `rbs add ci` y passe : ce que le
//! CLI écrit doit le traverser.

use std::io::Write;
use std::process::{Command, Stdio};

/// Ce que rustfmt n'a pas pu faire, à dire à l'utilisateur.
pub(crate) type Avertissement = String;

/// Formate chaque source, ou les rend telles quelles en disant pourquoi.
///
/// L'avertissement est unique pour le lot : le même rustfmt manquant répété sept fois
/// n'apprend rien de plus.
pub(crate) fn format_batch<'a>(
    sources: impl Iterator<Item = &'a mut String>,
) -> Option<Avertissement> {
    let mut avertissement = None;

    for source in sources {
        match formatted(source) {
            Ok(formatee) => *source = formatee,
            Err(raison) => {
                avertissement.get_or_insert(raison);
            }
        }
    }

    avertissement
}

/// Rend `source` telle que rustfmt l'écrirait.
fn formatted(source: &str) -> Result<String, Avertissement> {
    let mut rustfmt = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| {
            format!(
                "rustfmt introuvable ({source}) : le code généré est écrit tel quel et \
                 votre premier `cargo fmt` le reformatera.\n  \
                 `rustup component add rustfmt` l'installe."
            )
        })?;

    rustfmt
        .stdin
        .take()
        .expect("l'entrée de rustfmt vient d'être demandée")
        .write_all(source.as_bytes())
        .map_err(|error| format!("rustfmt n'a pas lu le rendu ({error})"))?;

    let output = rustfmt
        .wait_with_output()
        .map_err(|error| format!("rustfmt n'a pas rendu la main ({error})"))?;

    if !output.status.success() {
        // Un rendu que rustfmt refuse est un rendu qui ne compilera pas : l'écrire tel
        // quel donne à l'utilisateur l'erreur du compilateur, qui situe le défaut, là où
        // un abandon ici ne lui laisserait rien à lire.
        return Err(format!(
            "rustfmt a refusé le code généré, écrit tel quel :\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("rustfmt n'a pas rendu d'UTF-8 ({error})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_already_formatted_source_does_not_move() {
        let source = "fn main() {\n    println!(\"bonjour\");\n}\n".to_string();
        let mut sources = [source.clone()];

        assert_eq!(format_batch(sources.iter_mut()), None);
        assert_eq!(sources[0], source);
    }

    #[test]
    fn a_badly_formatted_source_is_straightened() {
        let mut sources = ["fn  main( ) {println!(\"bonjour\") ;}".to_string()];

        assert_eq!(format_batch(sources.iter_mut()), None);
        assert_eq!(sources[0], "fn main() {\n    println!(\"bonjour\");\n}\n");
    }

    /// Ce que rustfmt refuse est écrit tel quel : l'erreur du compilateur situera le
    /// défaut mieux qu'un abandon silencieux ici.
    #[test]
    fn a_source_rustfmt_refuses_is_returned_intact_with_a_warning() {
        let source = "fn main( {".to_string();
        let mut sources = [source.clone()];

        let avertissement = format_batch(sources.iter_mut()).expect("rustfmt doit se plaindre");

        assert!(avertissement.contains("refusé"), "{avertissement}");
        assert_eq!(sources[0], source);
    }

    /// Sept fichiers rendus sans rustfmt ne valent pas sept fois le même message.
    #[test]
    fn a_whole_batch_yields_only_one_warning() {
        let mut sources = ["fn main( {".to_string(), "fn autre( {".to_string()];

        let avertissement = format_batch(sources.iter_mut()).expect("rustfmt doit se plaindre");

        assert_eq!(
            avertissement.matches("refusé").count(),
            1,
            "{avertissement}"
        );
    }
}
