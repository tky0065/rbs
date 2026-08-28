//! Lecture du `.env` d'un projet généré.
//!
//! rbs lit lui-même le `.env` et transmet les variables au sous-processus qu'il lance :
//! le projet de l'utilisateur n'a ainsi pas à dépendre de `dotenvy` pour que ses
//! migrations tournent.

use std::fs;
use std::path::Path;

/// Ce qui peut empêcher de lire un `.env`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Le fichier n'existe pas ou n'a pas pu être lu.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin du fichier.
        path: String,
        /// Cause système.
        source: std::io::Error,
    },
}

/// Lit un `.env` et rend ses paires dans l'ordre du fichier.
pub fn read(path: &Path) -> Result<Vec<(String, String)>, Error> {
    let content = fs::read_to_string(path).map_err(|source| Error::Acces {
        path: path.display().to_string(),
        source,
    })?;

    Ok(parse(&content))
}

/// Découpe le contenu d'un `.env` en paires clé/valeur.
///
/// Les lignes vides, les commentaires et les lignes sans `=` sont ignorés : un `.env`
/// annoté à la main ne doit pas faire échouer une commande.
pub fn parse(content: &str) -> Vec<(String, String)> {
    content.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
    let (key, value) = line.split_once('=')?;

    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    Some((key.to_string(), unquote(value.trim()).to_string()))
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(interieur) = value
            .strip_prefix(quote)
            .and_then(|reste| reste.strip_suffix(quote))
        {
            return interieur;
        }
    }

    value
}

/// Cherche la valeur d'une clé dans des paires déjà lues.
pub fn value<'a>(paires: &'a [(String, String)], key: &str) -> Option<&'a str> {
    paires
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pairs_are_returned_in_file_order() {
        let paires = parse("DATABASE_URL=postgres://u@localhost/db\nRUST_LOG=info\n");

        assert_eq!(
            paires,
            vec![
                (
                    "DATABASE_URL".to_string(),
                    "postgres://u@localhost/db".to_string()
                ),
                ("RUST_LOG".to_string(), "info".to_string()),
            ]
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let paires = parse("# la base visée\n\n  # indenté\nPORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn the_export_prefix_is_stripped() {
        let paires = parse("export PORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn the_surrounding_quotes_are_stripped() {
        let paires = parse("A=\"info,api=debug\"\nB='simple'\n");

        assert_eq!(value(&paires, "A"), Some("info,api=debug"));
        assert_eq!(value(&paires, "B"), Some("simple"));
    }

    #[test]
    fn only_the_first_equals_separates_the_key_from_the_value() {
        let paires = parse("DATABASE_URL=postgres://u:p@h/db?opt=1\n");

        assert_eq!(
            value(&paires, "DATABASE_URL"),
            Some("postgres://u:p@h/db?opt=1")
        );
    }

    #[test]
    fn a_line_without_an_equals_is_ignored() {
        let paires = parse("ceci n'est pas une affectation\nPORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn a_missing_file_is_reported_with_its_path() {
        let error = read(Path::new("/inexistant/.env")).expect_err("le fichier n'existe pas");

        assert!(error.to_string().contains("/inexistant/.env"));
    }

    #[test]
    fn a_file_is_read_from_disk() {
        let directory = tempfile::tempdir().expect("répertoire temporaire");
        let path = directory.path().join(".env");
        fs::write(&path, "DATABASE_URL=postgres://u@localhost/db\n").expect("écriture");

        let paires = read(&path).expect("le fichier est lisible");

        assert_eq!(
            value(&paires, "DATABASE_URL"),
            Some("postgres://u@localhost/db")
        );
    }
}
