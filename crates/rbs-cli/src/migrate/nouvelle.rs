//! `rbs migrate new` : un fichier de migration vide, monté dans la crate `migration`.
//!
//! Le générateur de SeaORM n'est pas sollicité : rbs a déjà sa convention de nommage et
//! son moteur d'ancres, et une migration écrite à la main n'a pas besoin d'une base
//! démarrée pour exister.

use std::fs;
use std::io;
use std::path::Path;

use crate::ancres;
use crate::generate::montage;
use crate::template::Renderer;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/migration/vide.rs.jinja"
));

/// Une migration créée.
#[derive(Debug)]
pub(crate) struct Nouvelle {
    /// Chemin du fichier écrit, relatif à la racine du projet.
    ///
    /// Le nom du module s'en déduit : `DeriveMigrationName` le tire du fichier.
    pub fichier: String,
}

/// Ce qui peut empêcher de créer une migration.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Le nom donné ne peut pas devenir un module Rust.
    #[error("« {nom} » n'est pas un nom de migration : {raison}")]
    Nom {
        /// Le nom refusé.
        nom: String,
        /// Ce qui cloche.
        raison: &'static str,
    },

    /// Une migration porte déjà ce chemin.
    #[error("{fichier} existe déjà")]
    DejaLa {
        /// Chemin occupé, relatif à la racine.
        fichier: String,
    },

    /// Une ancre manque dans la crate `migration`.
    #[error(transparent)]
    Ancre(#[from] ancres::Absente),

    /// Un fichier du projet n'a pas pu être lu ou écrit.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin concerné.
        chemin: String,
        /// Cause système.
        source: io::Error,
    },

    /// La template de migration vide n'a pas pu être rendue.
    #[error("la migration n'a pas pu être rendue : {0}")]
    Rendu(#[from] minijinja::Error),
}

/// Crée la migration `nom`, datée de `horodatage`, dans le projet enraciné en `racine`.
///
/// L'horodatage est reçu et non lu de l'horloge : un test doit pouvoir viser un nom.
pub(crate) fn executer(racine: &Path, nom: &str, horodatage: &str) -> Result<Nouvelle, Erreur> {
    valider(nom)?;

    let module = format!("m{horodatage}_{nom}");
    let fichier = format!("migration/src/{module}.rs");
    let chemin = racine.join(&fichier);

    if chemin.exists() {
        return Err(Erreur::DejaLa { fichier });
    }

    let contenu = Renderer::new().rendre(TEMPLATE, minijinja::context! {})?;

    // Le lib.rs est monté avant la première écriture : une ancre absente ne doit pas
    // laisser un fichier de migration orphelin derrière elle.
    let lib = racine.join("migration/src/lib.rs");
    let mut source = lire(&lib)?;
    for montage in montage::pour_migration(&module) {
        source = ancres::inserer(&source, montage.ancre, &montage.lignes)?;
    }

    ecrire(&chemin, &contenu)?;
    ecrire(&lib, &source)?;

    Ok(Nouvelle { fichier })
}

/// Refuse ce qui ne peut pas devenir un module Rust.
///
/// `DeriveMigrationName` tire le nom en base de celui du fichier : un nom invalide se
/// verrait à la compilation, mais après écriture.
fn valider(nom: &str) -> Result<(), Erreur> {
    let refus = |raison| {
        Err(Erreur::Nom {
            nom: nom.to_string(),
            raison,
        })
    };

    if nom.is_empty() {
        return refus("il est vide");
    }

    if nom.starts_with(|c: char| c.is_ascii_digit()) {
        return refus("un identifiant Rust ne commence pas par un chiffre");
    }

    if !nom
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return refus("seuls les minuscules, les chiffres et « _ » sont admis");
    }

    Ok(())
}

fn lire(chemin: &Path) -> Result<String, Erreur> {
    fs::read_to_string(chemin).map_err(|source| Erreur::Acces {
        chemin: chemin.display().to_string(),
        source,
    })
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), Erreur> {
    fs::write(chemin, contenu).map_err(|source| Erreur::Acces {
        chemin: chemin.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    const HORODATAGE: &str = "20260826_143000";

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
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

    fn lire(chemin: &Path) -> String {
        fs::read_to_string(chemin)
            .unwrap_or_else(|erreur| panic!("{} illisible : {erreur}", chemin.display()))
    }

    #[test]
    fn le_fichier_porte_l_horodatage_et_le_nom_donne() {
        let (_parent, racine) = projet();

        let nouvelle = executer(&racine, "ajout_index", HORODATAGE).expect("la migration se crée");

        assert_eq!(
            nouvelle.fichier,
            "migration/src/m20260826_143000_ajout_index.rs"
        );
        assert!(racine.join(&nouvelle.fichier).is_file());
    }

    #[test]
    fn la_migration_est_declaree_puis_inscrite_dans_le_migrator() {
        let (_parent, racine) = projet();

        executer(&racine, "ajout_index", HORODATAGE).expect("la migration se crée");

        let lib = lire(&racine.join("migration/src/lib.rs"));
        assert!(lib.contains("mod m20260826_143000_ajout_index;"));
        assert!(lib.contains("Box::new(m20260826_143000_ajout_index::Migration),"));
    }

    #[test]
    fn la_migration_creee_est_du_rust_qui_declare_up_et_down() {
        let (_parent, racine) = projet();

        let nouvelle = executer(&racine, "ajout_index", HORODATAGE).expect("la migration se crée");
        let source = lire(&racine.join(&nouvelle.fichier));

        assert!(source.contains("impl MigrationTrait for Migration"));
        assert!(source.contains("async fn up("));
        assert!(source.contains("async fn down("));
    }

    #[test]
    fn un_nom_qui_n_est_pas_un_identifiant_rust_est_refuse_sans_rien_ecrire() {
        let (_parent, racine) = projet();
        let avant = lire(&racine.join("migration/src/lib.rs"));

        let erreur = executer(&racine, "ajout-index", HORODATAGE).expect_err("le tiret est refusé");

        assert!(erreur.to_string().contains("ajout-index"));
        assert_eq!(lire(&racine.join("migration/src/lib.rs")), avant);
    }

    #[test]
    fn un_nom_vide_est_refuse() {
        let (_parent, racine) = projet();

        executer(&racine, "", HORODATAGE).expect_err("un nom vide est refusé");
    }

    #[test]
    fn un_nom_commencant_par_un_chiffre_est_refuse() {
        let (_parent, racine) = projet();

        executer(&racine, "2_index", HORODATAGE).expect_err("un chiffre initial est refusé");
    }

    #[test]
    fn deux_migrations_du_meme_nom_dans_la_meme_seconde_ne_s_ecrasent_pas() {
        let (_parent, racine) = projet();

        executer(&racine, "ajout_index", HORODATAGE).expect("la première se crée");
        let erreur =
            executer(&racine, "ajout_index", HORODATAGE).expect_err("la seconde est refusée");

        assert!(erreur.to_string().contains("existe déjà"));
    }
}
