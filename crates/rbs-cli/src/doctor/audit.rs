//! Contrôle de la feature `audit`.
//!
//! Le journal écrit dans la table `audit_log`, que la migration `create_audit_log` crée. Une
//! migration déclarée mais absente du `Migrator` compile et ne s'applique jamais : le premier
//! `audit::record` échoue alors dans la transaction même qu'il devait tracer, et l'emporte
//! avec lui.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "audit";
const FICHIER: &str = "migration/src/lib.rs";
const SUFFIXE: &str = "_create_audit_log";

/// Vérifie que la migration du journal est déclarée et inscrite au `Migrator`.
pub(crate) fn check(root: &Path) -> Check {
    let source = match super::lire(root, TITRE, FICHIER) {
        Ok(source) => source,
        Err(constat) => return constat,
    };

    // Les deux préfixes attendus écartent d'eux-mêmes une ligne en commentaire.
    let code: Vec<&str> = source.lines().map(str::trim).collect();
    let declaree = code.iter().find_map(|ligne| {
        ligne
            .strip_prefix("mod ")
            .and_then(|reste| reste.strip_suffix(';'))
            .filter(|module| module.ends_with(SUFFIXE))
    });
    let inscrite = code.iter().any(|ligne| {
        ligne.starts_with("Box::new(") && ligne.contains(&format!("{SUFFIXE}::Migration)"))
    });

    match (declaree, inscrite) {
        (Some(_), true) => Check::ok(TITRE, "la migration create_audit_log est inscrite"),
        (Some(module), false) => Check::failed(
            TITRE,
            format!(
                "`{module}` est déclarée mais absente du Migrator : la table audit_log ne \
                 sera jamais créée"
            ),
            format!(
                "dans {FICHIER}, entre les balises de `// <rbs:migrations>` :\n\
                 Box::new({module}::Migration),"
            ),
        ),
        (None, _) => match migration_presente(root) {
            Some(module) => Check::failed(
                TITRE,
                format!(
                    "la migration `{module}` n'est pas inscrite dans {FICHIER} : la table \
                     audit_log ne sera jamais créée"
                ),
                format!(
                    "dans {FICHIER}, entre les balises de `// <rbs:migration_modules>` :\n\
                     mod {module};\npuis entre celles de `// <rbs:migrations>` :\n\
                     Box::new({module}::Migration),"
                ),
            ),
            None => Check::failed(
                TITRE,
                "aucune migration create_audit_log dans migration/src/ : la table audit_log \
                 ne sera jamais créée",
                "restaurez-la depuis Git (`git checkout -- migration/src/`) : `rbs add` ne \
                 rejoue pas une feature déjà installée",
            ),
        },
    }
}

/// Le module de la migration d'audit que porte `migration/src/`, s'il en porte une.
///
/// Cherché par suffixe : son nom commence par l'horodatage de l'installation.
fn migration_presente(root: &Path) -> Option<String> {
    std::fs::read_dir(root.join("migration/src"))
        .ok()?
        .flatten()
        .filter_map(|entree| entree.file_name().into_string().ok())
        .filter_map(|nom| nom.strip_suffix(".rs").map(str::to_owned))
        .find(|module| module.ends_with(SUFFIXE))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    const MODULE: &str = "m20260913_131820_create_audit_log";

    /// Un projet réduit à sa crate de migration : `lib.rs` s'il en porte un, et les fichiers
    /// de migration nommés.
    fn projet(lib: Option<&str>, migrations: &[&str]) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        let dossier = racine.path().join("migration/src");
        fs::create_dir_all(&dossier).expect("crate de migration créable");

        if let Some(source) = lib {
            fs::write(dossier.join("lib.rs"), source).expect("lib.rs inscriptible");
        }
        for migration in migrations {
            fs::write(dossier.join(format!("{migration}.rs")), "").expect("migration inscriptible");
        }

        racine
    }

    /// `migration/src/lib.rs` tel que le squelette le livre, ses deux ancres portant ce
    /// qu'on leur donne.
    fn lib(modules: &str, migrations: &str) -> String {
        format!(
            "pub use sea_orm_migration::prelude::*;\n\n// <rbs:migration_modules>\n{modules}\
             // </rbs:migration_modules>\n\npub struct Migrator;\n\nimpl MigratorTrait for \
             Migrator {{\n    fn migrations() -> Vec<Box<dyn MigrationTrait>> {{\n        \
             vec![\n            // <rbs:migrations>\n{migrations}            \
             // </rbs:migrations>\n        ]\n    }}\n}}\n"
        )
    }

    fn declaree() -> String {
        format!("mod {MODULE};\n")
    }

    fn inscrite() -> String {
        format!("            Box::new({MODULE}::Migration),\n")
    }

    #[test]
    fn a_declared_and_registered_migration_reports_nothing() {
        let racine = projet(Some(&lib(&declaree(), &inscrite())), &[MODULE]);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Déclarée sans être inscrite, la migration compile et ne s'applique jamais : la table
    /// manque au premier `audit::record`.
    #[test]
    fn a_declared_migration_missing_from_the_migrator_is_named() {
        let racine = projet(Some(&lib(&declaree(), "")), &[MODULE]);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(MODULE), "{}", check.detail);
        let remede = check.remedy.expect("un échec porte son remède");
        assert!(
            remede.contains(&format!("Box::new({MODULE}::Migration),")),
            "{remede}"
        );
    }

    #[test]
    fn a_migration_file_nothing_declares_names_both_lines() {
        let racine = projet(Some(&lib("", "")), &[MODULE]);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        let remede = check.remedy.expect("un échec porte son remède");
        assert!(remede.contains(&format!("mod {MODULE};")), "{remede}");
        assert!(
            remede.contains(&format!("Box::new({MODULE}::Migration),")),
            "{remede}"
        );
    }

    #[test]
    fn without_any_audit_migration_the_table_is_said_to_be_missing() {
        let racine = projet(Some(&lib("", "")), &[]);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("audit_log"), "{}", check.detail);
        assert!(
            check
                .remedy
                .as_deref()
                .is_some_and(|remede| remede.contains("git")),
            "{:?}",
            check.remedy
        );
    }

    #[test]
    fn a_commented_out_registration_does_not_count() {
        let racine = projet(
            Some(&lib(
                &declaree(),
                &format!("            // Box::new({MODULE}::Migration),\n"),
            )),
            &[MODULE],
        );

        assert_eq!(check(racine.path()).state, State::Echec);
    }

    #[test]
    fn a_missing_migration_crate_is_named() {
        let racine = projet(None, &[]);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
    }
}
