//! Lecture du `.env` d'un projet généré.
//!
//! rbs lit lui-même le `.env` et transmet les variables au sous-processus qu'il lance :
//! le projet de l'utilisateur n'a ainsi pas à dépendre de `dotenvy` pour que ses
//! migrations tournent.

use std::fs;
use std::path::Path;

/// Ce qui peut empêcher de lire un `.env`.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Le fichier n'existe pas ou n'a pas pu être lu.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin du fichier.
        chemin: String,
        /// Cause système.
        source: std::io::Error,
    },
}

/// Lit un `.env` et rend ses paires dans l'ordre du fichier.
pub fn lire(chemin: &Path) -> Result<Vec<(String, String)>, Erreur> {
    let contenu = fs::read_to_string(chemin).map_err(|source| Erreur::Acces {
        chemin: chemin.display().to_string(),
        source,
    })?;

    Ok(analyser(&contenu))
}

/// Découpe le contenu d'un `.env` en paires clé/valeur.
///
/// Les lignes vides, les commentaires et les lignes sans `=` sont ignorés : un `.env`
/// annoté à la main ne doit pas faire échouer une commande.
pub fn analyser(contenu: &str) -> Vec<(String, String)> {
    contenu.lines().filter_map(analyser_ligne).collect()
}

fn analyser_ligne(ligne: &str) -> Option<(String, String)> {
    let ligne = ligne.trim();
    if ligne.is_empty() || ligne.starts_with('#') {
        return None;
    }

    let ligne = ligne.strip_prefix("export ").unwrap_or(ligne).trim_start();
    let (cle, valeur) = ligne.split_once('=')?;

    let cle = cle.trim();
    if cle.is_empty() {
        return None;
    }

    Some((cle.to_string(), deguillemeter(valeur.trim()).to_string()))
}

fn deguillemeter(valeur: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(interieur) = valeur
            .strip_prefix(quote)
            .and_then(|reste| reste.strip_suffix(quote))
        {
            return interieur;
        }
    }

    valeur
}

/// Cherche la valeur d'une clé dans des paires déjà lues.
pub fn valeur<'a>(paires: &'a [(String, String)], cle: &str) -> Option<&'a str> {
    paires
        .iter()
        .find(|(nom, _)| nom == cle)
        .map(|(_, valeur)| valeur.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_paires_sont_rendues_dans_l_ordre_du_fichier() {
        let paires = analyser("DATABASE_URL=postgres://u@localhost/db\nRUST_LOG=info\n");

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
    fn les_commentaires_et_les_lignes_vides_sont_ignores() {
        let paires = analyser("# la base visée\n\n  # indenté\nPORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn le_prefixe_export_est_retire() {
        let paires = analyser("export PORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn les_guillemets_englobants_sont_retires() {
        let paires = analyser("A=\"info,api=debug\"\nB='simple'\n");

        assert_eq!(valeur(&paires, "A"), Some("info,api=debug"));
        assert_eq!(valeur(&paires, "B"), Some("simple"));
    }

    #[test]
    fn le_premier_egal_seul_separe_la_cle_de_la_valeur() {
        let paires = analyser("DATABASE_URL=postgres://u:p@h/db?opt=1\n");

        assert_eq!(
            valeur(&paires, "DATABASE_URL"),
            Some("postgres://u:p@h/db?opt=1")
        );
    }

    #[test]
    fn une_ligne_sans_egal_est_ignoree() {
        let paires = analyser("ceci n'est pas une affectation\nPORT=3000\n");

        assert_eq!(paires, vec![("PORT".to_string(), "3000".to_string())]);
    }

    #[test]
    fn un_fichier_absent_est_signale_avec_son_chemin() {
        let erreur = lire(Path::new("/inexistant/.env")).expect_err("le fichier n'existe pas");

        assert!(erreur.to_string().contains("/inexistant/.env"));
    }

    #[test]
    fn un_fichier_est_lu_depuis_le_disque() {
        let repertoire = tempfile::tempdir().expect("répertoire temporaire");
        let chemin = repertoire.path().join(".env");
        fs::write(&chemin, "DATABASE_URL=postgres://u@localhost/db\n").expect("écriture");

        let paires = lire(&chemin).expect("le fichier est lisible");

        assert_eq!(
            valeur(&paires, "DATABASE_URL"),
            Some("postgres://u@localhost/db")
        );
    }
}
