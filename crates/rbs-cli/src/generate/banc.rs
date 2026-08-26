//! Banc d'essai des générateurs : un vrai projet, créé par le vrai binaire, compilé.
//!
//! Les tests de rendu vérifient des chaînes ; seul `rustc` prouve que ces chaînes forment
//! du Rust valide contre SeaORM, utoipa et validator. Le projet est donc créé par
//! `rbs new`, sa feature écrite, puis compilé.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
pub(crate) fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

/// Un projet neuf, créé par le binaire livré, prêt à recevoir une feature.
pub(crate) struct Projet {
    _parent: TempDir,
    racine: PathBuf,
}

impl Projet {
    pub(crate) fn neuf() -> Self {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let noyau = depot().join("crates/rbs-core");

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(parent.path())
            .args([
                "new",
                "demo-api",
                "--database-url",
                "postgres://rbs:rbs@localhost:5432/demo_api",
                "--core-path",
                noyau.to_str().expect("chemin du noyau représentable"),
                "--yes",
            ])
            .assert()
            .success();

        let racine = parent.path().join("demo-api");

        Self {
            _parent: parent,
            racine,
        }
    }

    pub(crate) fn racine(&self) -> &Path {
        &self.racine
    }

    /// Écrit `src/features/<module>/` avec les fichiers donnés, et déclare le module.
    ///
    /// L'insertion dans l'ancre est faite à la main : le moteur d'ancres est une tâche
    /// distincte, dont ce banc ne doit pas dépendre.
    pub(crate) fn poser_feature(&self, module: &str, fichiers: &[(&str, &str)]) {
        let repertoire = self.racine.join("src/features").join(module);
        fs::create_dir_all(&repertoire).expect("répertoire de feature créable");

        let declarations: String = fichiers
            .iter()
            .map(|(nom, _)| {
                let module = nom.trim_end_matches(".rs");
                format!("pub mod {module};\n")
            })
            .collect();

        fs::write(repertoire.join("mod.rs"), declarations).expect("mod.rs écrivable");

        for (nom, contenu) in fichiers {
            fs::write(repertoire.join(nom), contenu).expect("fichier de feature écrivable");
        }

        let index = self.racine.join("src/features/mod.rs");
        let source = fs::read_to_string(&index).expect("index des features lisible");

        fs::write(
            &index,
            source.replace(
                "// <rbs:features>",
                &format!("// <rbs:features>\npub mod {module};"),
            ),
        )
        .expect("index des features écrivable");
    }

    /// Ajoute une migration au projet et l'inscrit dans l'ancre du `Migrator`.
    pub(crate) fn poser_migration(&self, module: &str, contenu: &str) {
        let sources = self.racine.join("migration/src");
        fs::write(sources.join(format!("{module}.rs")), contenu).expect("migration écrivable");

        let lib = sources.join("lib.rs");
        let source = fs::read_to_string(&lib).expect("lib.rs de migration lisible");

        fs::write(
            &lib,
            format!("mod {module};\n\n{}", source).replace(
                "// <rbs:migrations>",
                &format!("// <rbs:migrations>\n            Box::new({module}::Migration),"),
            ),
        )
        .expect("lib.rs de migration écrivable");
    }

    /// Compile le projet, et échoue en rapportant la sortie de `cargo` telle quelle.
    pub(crate) fn compiler(&self) {
        let sortie = std::process::Command::new("cargo")
            .current_dir(&self.racine)
            .env("CARGO_TARGET_DIR", depot().join("target/rbs-integration"))
            .args(["build", "--workspace"])
            .output()
            .expect("cargo doit être lançable");

        assert!(
            sortie.status.success(),
            "le projet ne compile pas :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
    }
}
