//! Cohérence entre le projet, son noyau et le CLI qui le diagnostique.
//!
//! Un projet généré par une version de rbs et manipulé par une autre n'est pas fautif en
//! soi — mais c'est la première chose à savoir quand une génération se comporte
//! autrement qu'attendu.

use std::fs;
use std::path::Path;

use super::Controle;

const TITRE: &str = "versions";

/// Version du CLI en train de diagnostiquer.
const CLI: &str = env!("CARGO_PKG_VERSION");

/// Compare la version qui a généré le projet, celle de son noyau et celle du CLI.
pub(crate) fn controler(racine: &Path) -> Controle {
    let manifeste = racine.join("Cargo.toml");

    let metadonnees = match crate::metadata::lire(&manifeste) {
        Ok(metadonnees) => metadonnees,
        Err(erreur) => {
            return Controle::echec(
                TITRE,
                erreur.to_string(),
                "restaurez le manifeste du projet",
            );
        }
    };

    let mut ecarts = Vec::new();

    if metadonnees.version != CLI {
        ecarts.push(format!(
            "projet généré par rbs {}, CLI {CLI}",
            metadonnees.version
        ));
    }

    let noyau = match noyau(&manifeste) {
        Ok(noyau) => noyau,
        Err(detail) => {
            return Controle::echec(
                TITRE,
                detail,
                format!("déclarez rbs-core = \"{CLI}\" dans [dependencies]"),
            );
        }
    };

    let noyau = match noyau {
        Noyau::Local => "rbs-core pris d'un chemin local".to_string(),
        Noyau::Version(version) if version == CLI => format!("rbs-core {version}"),
        Noyau::Version(version) => {
            ecarts.push(format!("rbs-core {version}, CLI {CLI}"));
            String::new()
        }
    };

    if ecarts.is_empty() {
        return Controle::bon(TITRE, format!("projet et {noyau} alignés sur le CLI {CLI}"));
    }

    Controle::echec(
        TITRE,
        ecarts.join(" ; "),
        format!(
            "alignez le projet sur rbs {CLI}, ou relancez la commande avec le CLI qui l'a généré"
        ),
    )
}

/// D'où le projet tire `rbs-core`.
enum Noyau {
    /// Une version publiée.
    Version(String),
    /// Un chemin du disque : le mode de développement de rbs lui-même.
    Local,
}

/// Lit la dépendance `rbs-core` du manifeste.
fn noyau(manifeste: &Path) -> Result<Noyau, String> {
    let source = fs::read_to_string(manifeste).map_err(|erreur| erreur.to_string())?;
    let document: toml_edit::DocumentMut = source
        .parse()
        .map_err(|erreur: toml_edit::TomlError| erreur.to_string())?;

    let Some(dependance) = document
        .get("dependencies")
        .and_then(|table| table.get("rbs-core"))
    else {
        return Err("rbs-core n'est pas une dépendance du projet".to_string());
    };

    if let Some(version) = dependance.as_str() {
        return Ok(Noyau::Version(version.to_string()));
    }

    if dependance.get("path").is_some() {
        return Ok(Noyau::Local);
    }

    match dependance.get("version").and_then(|v| v.as_str()) {
        Some(version) => Ok(Noyau::Version(version.to_string())),
        None => Err("la dépendance rbs-core ne porte ni version ni chemin".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::Etat;
    use super::*;

    fn projet() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let projet = crate::new::creer(
            &crate::new::Options {
                nom: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, projet.racine)
    }

    /// Remplace un fragment du manifeste du projet.
    fn reecrire(racine: &Path, avant: &str, apres: &str) {
        let chemin = racine.join("Cargo.toml");
        let source = fs::read_to_string(&chemin).expect("le manifeste est lisible");
        assert!(source.contains(avant), "« {avant} » absent du manifeste");
        fs::write(&chemin, source.replace(avant, apres)).expect("le manifeste est réécrivable");
    }

    #[test]
    fn un_projet_neuf_est_coherent_avec_le_cli_qui_l_a_genere() {
        let (_parent, racine) = projet();

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
        assert!(controle.detail.contains(CLI));
    }

    #[test]
    fn un_projet_genere_par_une_autre_version_est_signale_avec_les_deux_numeros() {
        let (_parent, racine) = projet();
        reecrire(
            &racine,
            &format!("version = \"{CLI}\"\nfeatures"),
            "version = \"0.0.1\"\nfeatures",
        );

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("0.0.1"));
        assert!(controle.detail.contains(CLI));
    }

    #[test]
    fn un_noyau_d_une_autre_version_que_le_cli_est_signale() {
        let (_parent, racine) = projet();
        reecrire(
            &racine,
            &format!("rbs-core = \"{CLI}\""),
            "rbs-core = \"0.0.1\"",
        );

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("rbs-core"));
        assert!(controle.detail.contains("0.0.1"));
    }

    #[test]
    fn un_noyau_pris_d_un_chemin_local_est_dit_sans_etre_tenu_pour_fautif() {
        let (_parent, racine) = projet();
        reecrire(
            &racine,
            &format!("rbs-core = \"{CLI}\""),
            "rbs-core = { path = \"../../crates/rbs-core\" }",
        );

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
        assert!(
            controle.detail.contains("chemin local"),
            "le mode développement doit rester visible : {}",
            controle.detail
        );
    }

    #[test]
    fn un_manifeste_sans_dependance_au_noyau_est_signale() {
        let (_parent, racine) = projet();
        reecrire(&racine, &format!("rbs-core = \"{CLI}\"\n"), "");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("rbs-core"));
    }
}
