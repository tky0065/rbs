//! `rbs migrate new` : un fichier de migration vide, monté dans la crate `migration`.
//!
//! Le générateur de SeaORM n'est pas sollicité : rbs a déjà sa convention de nommage et
//! son moteur d'ancres, et une migration écrite à la main n'a pas besoin d'une base
//! démarrée pour exister.

use std::fs;
use std::io;
use std::path::Path;

use crate::anchors;
use crate::generate::mount;
use crate::template::Renderer;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/migration/vide.rs.jinja"
));

/// Une migration créée.
#[derive(Debug)]
pub(crate) struct Fresh {
    /// Chemin du fichier écrit, relatif à la racine du projet.
    ///
    /// Le nom du module s'en déduit : `DeriveMigrationName` le tire du fichier.
    pub file: String,
}

/// Ce qui peut empêcher de créer une migration.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Le nom donné ne peut pas devenir un module Rust.
    #[error("« {name} » n'est pas un nom de migration : {raison}")]
    Nom {
        /// Le nom refusé.
        name: String,
        /// Ce qui cloche.
        raison: &'static str,
    },

    /// Une migration porte déjà ce chemin.
    #[error("{file} existe déjà")]
    DejaLa {
        /// Chemin occupé, relatif à la racine.
        file: String,
    },

    /// Une ancre manque dans la crate `migration`.
    #[error(transparent)]
    Anchor(#[from] anchors::Missing),

    /// Un fichier du projet n'a pas pu être lu ou écrit.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin concerné.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// La template de migration vide n'a pas pu être rendue.
    #[error("la migration n'a pas pu être rendue : {0}")]
    Rendu(#[from] minijinja::Error),
}

/// Crée la migration `name`, datée de `timestamp`, dans le projet enraciné en `root`.
///
/// L'horodatage est reçu et non lu de l'horloge : un test doit pouvoir viser un nom.
pub(crate) fn run(root: &Path, name: &str, timestamp: &str) -> Result<Fresh, Error> {
    validate(name)?;

    let module = format!("m{timestamp}_{name}");
    let file = format!("migration/src/{module}.rs");
    let path = root.join(&file);

    if path.exists() {
        return Err(Error::DejaLa { file });
    }

    let content = Renderer::new().render(TEMPLATE, minijinja::context! {})?;

    // Le lib.rs est monté avant la première écriture : une ancre absente ne doit pas
    // laisser un fichier de migration orphelin derrière elle.
    let lib = root.join("migration/src/lib.rs");
    let mut source = read(&lib)?;
    for mount in mount::for_migration(&module) {
        source = anchors::insert(&source, mount.anchor, &mount.lines)?;
    }

    write(&path, &content)?;
    write(&lib, &source)?;

    Ok(Fresh { file })
}

/// Refuse ce qui ne peut pas devenir un module Rust.
///
/// `DeriveMigrationName` tire le nom en base de celui du fichier : un nom invalide se
/// verrait à la compilation, mais après écriture.
fn validate(name: &str) -> Result<(), Error> {
    let refus = |raison| {
        Err(Error::Nom {
            name: name.to_string(),
            raison,
        })
    };

    if name.is_empty() {
        return refus("il est vide");
    }

    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return refus("un identifiant Rust ne commence pas par un chiffre");
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return refus("seuls les minuscules, les chiffres et « _ » sont admis");
    }

    Ok(())
}

fn read(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|source| Error::Acces {
        path: path.display().to_string(),
        source,
    })
}

fn write(path: &Path, content: &str) -> Result<(), Error> {
    fs::write(path, content).map_err(|source| Error::Acces {
        path: path.display().to_string(),
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
    fn project() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} illisible : {error}", path.display()))
    }

    #[test]
    fn the_file_carries_the_timestamp_and_the_given_name() {
        let (_parent, root) = project();

        let fresh = run(&root, "ajout_index", HORODATAGE).expect("la migration se crée");

        assert_eq!(fresh.file, "migration/src/m20260826_143000_ajout_index.rs");
        assert!(root.join(&fresh.file).is_file());
    }

    #[test]
    fn the_migration_is_declared_then_recorded_in_the_migrator() {
        let (_parent, root) = project();

        run(&root, "ajout_index", HORODATAGE).expect("la migration se crée");

        let lib = read(&root.join("migration/src/lib.rs"));
        assert!(lib.contains("mod m20260826_143000_ajout_index;"));
        assert!(lib.contains("Box::new(m20260826_143000_ajout_index::Migration),"));
    }

    #[test]
    fn the_created_migration_is_rust_declaring_up_and_down() {
        let (_parent, root) = project();

        let fresh = run(&root, "ajout_index", HORODATAGE).expect("la migration se crée");
        let source = read(&root.join(&fresh.file));

        assert!(source.contains("impl MigrationTrait for Migration"));
        assert!(source.contains("async fn up("));
        assert!(source.contains("async fn down("));
    }

    #[test]
    fn a_name_that_is_not_a_rust_identifier_is_rejected_without_writing_anything() {
        let (_parent, root) = project();
        let before = read(&root.join("migration/src/lib.rs"));

        let error = run(&root, "ajout-index", HORODATAGE).expect_err("le tiret est refusé");

        assert!(error.to_string().contains("ajout-index"));
        assert_eq!(read(&root.join("migration/src/lib.rs")), before);
    }

    #[test]
    fn an_empty_name_is_rejected() {
        let (_parent, root) = project();

        run(&root, "", HORODATAGE).expect_err("un nom vide est refusé");
    }

    #[test]
    fn a_name_starting_with_a_digit_is_rejected() {
        let (_parent, root) = project();

        run(&root, "2_index", HORODATAGE).expect_err("un chiffre initial est refusé");
    }

    #[test]
    fn two_migrations_of_the_same_name_in_the_same_second_do_not_overwrite_each_other() {
        let (_parent, root) = project();

        run(&root, "ajout_index", HORODATAGE).expect("la première se crée");
        let error = run(&root, "ajout_index", HORODATAGE).expect_err("la seconde est refusée");

        assert!(error.to_string().contains("existe déjà"));
    }
}
