//! La planification d'une commande qui modifie un projet, réifiée en valeur.
//!
//! Un plan est une liste d'actions ; chaque action vise un fichier et connaît son contenu
//! avant et son contenu après. Planifier, c'est calculer les « après » sans rien écrire —
//! d'où l'affichage préalable, la restauration en cas d'échec et l'idempotence.

mod action;
pub(crate) mod application;
pub(crate) mod render;
mod text;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::anchors::Anchor;

pub(crate) use action::{Action, Effect, PatchToml, Status};

/// Un fichier que le plan touche, avec ses deux états et ce qu'il en coûtera d'y écrire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct File {
    /// Chemin relatif à la racine du projet.
    pub path: String,
    /// Contenu actuel, ou `None` si le fichier n'existe pas encore.
    pub before: Option<String>,
    /// Contenu que l'application écrira.
    pub after: String,
    /// Statut agrégé des actions qui visent ce fichier.
    ///
    /// Sans lui, un appelant qui écrit `files()` tel quel écraserait un fichier en
    /// conflit sans jamais voir le conflit, resté dans `actions()`.
    pub statut: Status,
}

/// Ce qu'une commande fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    root: PathBuf,
    actions: Vec<Action>,
    files: Vec<File>,
}

impl Plan {
    /// Les actions dans l'ordre où elles ont été planifiées.
    ///
    /// Trace du calcul des statuts, action par action, que les tests du modèle vérifient.
    /// L'affichage et l'application travaillent par fichier : un fichier peut recevoir
    /// plusieurs actions, et seul son statut agrégé décide de ce qui lui arrivera.
    #[allow(dead_code)]
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Les fichiers touchés, un par chemin, dans l'ordre où ils ont été rencontrés.
    pub fn files(&self) -> &[File] {
        &self.files
    }

    /// Racine du projet, à laquelle les chemins des fichiers sont relatifs.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Ce qui peut empêcher de planifier.
///
/// Chaque variante nomme son fichier relativement à la racine, comme `Action::path` :
/// l'emplacement complet du projet est porté une seule fois, par l'en-tête de l'affichage
/// du plan.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Un fichier du projet n'a pas pu être lu.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif, relatif à la racine.
        path: String,
        /// Cause système.
        source: io::Error,
    },
    /// Deux actions du plan prétendent écrire le même fichier de bout en bout.
    ///
    /// Erreur de programmation de l'appelant : deux contenus complets ne se composent
    /// pas, et le second effacerait silencieusement ce que le premier a projeté.
    #[error("{path} est déjà projeté par une action précédente du plan")]
    DejaProjete {
        /// Chemin fautif, relatif à la racine.
        path: String,
    },
    /// Une ancre attendue a disparu du projet.
    #[error("{0}")]
    Anchor(#[source] crate::anchors::Missing),
    /// Le fichier qui porte l'ancre visée n'existe pas.
    ///
    /// Distincte d'`Anchor`, qui suppose au contraire un fichier présent mais dépourvu de
    /// ses balises : ici c'est le fichier entier qui manque, et chercher une balise
    /// dedans n'aurait aucun sens.
    #[error("{path} est introuvable")]
    FichierAbsent {
        /// Chemin du fichier porteur, relatif à la racine.
        path: String,
    },
    /// Le manifeste du projet n'a pas pu être patché.
    #[error("{0}")]
    Metadata(#[source] crate::metadata::Error),
    /// Un document TOML du projet ne s'analyse pas.
    ///
    /// Distincte de `Metadata` : celle-ci vise les documents de configuration, dont
    /// rien ne dit qu'ils portent une section `[package]`.
    #[error("{path} n'est pas un TOML valide : {source}")]
    Toml {
        /// Chemin fautif, relatif à la racine.
        path: String,
        /// Cause de l'analyse.
        source: toml_edit::TomlError,
    },
    /// Le `Cargo.toml` visé par un patch n'existe pas à l'emplacement attendu.
    ///
    /// Distincte de `Metadata(PasUnProjet)`, qui suppose au contraire un fichier
    /// présent mais dépourvu de la section `[package.metadata.rbs]`.
    #[error("{path} est introuvable")]
    ManifesteAbsent {
        /// Chemin du manifeste, relatif à la racine.
        path: String,
    },
}

/// Accumule les actions d'un plan en calculant, pour chaque fichier, son contenu final.
pub(crate) struct Builder {
    root: PathBuf,
    actions: Vec<Action>,
    files: Vec<File>,
}

impl Builder {
    /// Ouvre un plan vide sur le projet enraciné en `root`.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            actions: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Planifie l'écriture de `path` avec `content`.
    ///
    /// Refuse un chemin qu'une action précédente a déjà projeté : voir
    /// [`Error::DejaProjete`].
    pub fn create(&mut self, path: &str, content: &str) -> Result<(), Error> {
        if self.projected(path) {
            return Err(Error::DejaProjete {
                path: path.to_string(),
            });
        }

        let origin = self.read(path)?;

        let statut = match origin.as_deref() {
            None => Status::AFaire,
            Some(actuel) if actuel == content => Status::DejaFait,
            Some(_) => Status::Conflit,
        };

        self.project_onto(path, origin, content.to_string(), statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::Creer {
                content: content.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de `lines` dans `anchor`, juste avant sa balise fermante.
    ///
    /// Le fichier visé est celui que l'ancre désigne : une ancre ne se déplace pas.
    pub fn insert(&mut self, anchor: Anchor, lines: &[String]) -> Result<(), Error> {
        let path = anchor.file;

        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let after = crate::anchors::insert(&courant, anchor, lines).map_err(Error::Anchor)?;
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::Inserer {
                anchor,
                lines: lines.to_vec(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie une modification du `Cargo.toml` de la racine.
    pub fn patch(&mut self, patch: PatchToml) -> Result<(), Error> {
        let path = "Cargo.toml";

        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::ManifesteAbsent {
            path: path.to_string(),
        })?;

        let rendered = match &patch {
            PatchToml::InscrireFeature(feature) => {
                crate::metadata::record_feature(&courant, feature, path)
            }
            PatchToml::AjouterDependance(dependency) => {
                crate::metadata::add_dependency(&courant, dependency, path)
            }
            PatchToml::AjouterFeatureADependance {
                dependency,
                feature,
            } => crate::metadata::add_feature_to_dependency(&courant, dependency, feature, path),
        }
        .map_err(Error::Metadata)?;

        let after = rendered.unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::PatcherToml { patch },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de la section `section` au document TOML `path`.
    pub fn add_section(&mut self, path: &str, section: &str, content: &str) -> Result<(), Error> {
        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let rendered =
            text::add_section(&courant, section, content).map_err(|source| Error::Toml {
                path: path.to_string(),
                source,
            })?;

        let after = rendered.unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::AjouterSection {
                section: section.to_string(),
                content: content.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de la variable `key` au fichier d'environnement `path`.
    pub fn add_variable(
        &mut self,
        path: &str,
        key: &str,
        value: &str,
        comment: Option<&str>,
    ) -> Result<(), Error> {
        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let after = text::add_variable(&courant, key, value, comment).unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::AjouterVariable {
                key: key.to_string(),
                value: value.to_string(),
                comment: comment.map(str::to_string),
            },
            statut,
        });

        Ok(())
    }

    /// Clôt le plan.
    pub fn finir(self) -> Plan {
        Plan {
            root: self.root,
            actions: self.actions,
            files: self.files,
        }
    }

    /// Une action précédente a-t-elle déjà calculé le contenu final de ce fichier ?
    fn projected(&self, path: &str) -> bool {
        self.files.iter().any(|file| file.path == path)
    }

    /// Ce qu'une action trouve du fichier qu'elle vise.
    ///
    /// Les deux états ne se confondent qu'au premier passage sur un fichier : ensuite,
    /// l'action compose avec ce que la précédente a produit, mais son statut se décide
    /// toujours contre l'origine.
    fn states(&self, path: &str) -> Result<States, Error> {
        if let Some(file) = self.files.iter().find(|f| f.path == path) {
            return Ok(States {
                origin: file.before.clone(),
                courant: Some(file.after.clone()),
            });
        }

        let disque = self.read(path)?;

        Ok(States {
            origin: disque.clone(),
            courant: disque,
        })
    }

    /// Contenu du fichier sur le disque, ou `None` s'il n'existe pas.
    fn read(&self, path: &str) -> Result<Option<String>, Error> {
        match fs::read_to_string(self.root.join(path)) {
            Ok(content) => Ok(Some(content)),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Error::Acces {
                path: path.to_string(),
                source,
            }),
        }
    }

    /// Enregistre le contenu final du fichier, en conservant son état d'origine et en
    /// agrégeant le statut des actions qui le visent.
    fn project_onto(&mut self, path: &str, before: Option<String>, after: String, statut: Status) {
        match self.files.iter_mut().find(|f| f.path == path) {
            Some(file) => {
                file.after = after;
                file.statut = file.statut.merge(statut);
            }
            None => self.files.push(File {
                path: path.to_string(),
                before,
                after,
                statut,
            }),
        }
    }
}

/// Les deux lectures dont une action a besoin pour se planifier.
struct States {
    /// Contenu du fichier tel que la planification a trouvé le projet, `None` s'il n'y
    /// existait pas.
    origin: Option<String>,
    /// Contenu du fichier tel que les actions déjà planifiées le laisseront.
    courant: Option<String>,
}

/// Statut d'une action qui compose avec ce qu'elle trouve — une insertion, un patch.
///
/// Elle n'est sans effet que si le projet d'origine porte déjà ce qu'elle produit ; elle
/// n'entre jamais en conflit, puisqu'elle ne remplace pas un fichier entier.
fn combined_status(origin: Option<&str>, after: &str) -> Status {
    if origin == Some(after) {
        Status::DejaFait
    } else {
        Status::AFaire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchors;
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    const ROUTER: &str = "pub fn router() -> Router {\n    Router::new()\n        // <rbs:routes>\n        // </rbs:routes>\n}\n";

    fn with_router(project: &TempDir, source: &str) {
        fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        fs::write(project.path().join("src/router.rs"), source).expect("l'écriture aboutit");
    }

    #[test]
    fn creating_a_missing_file_is_todo() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent, rien ne s'y oppose");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.files()[0].before, None);
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn creating_an_already_identical_file_is_done() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM rust\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].before.as_deref(), Some("FROM rust\n"));
    }

    #[test]
    fn creating_over_different_content_is_a_conflict() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::Conflit);
        assert_eq!(plan.files()[0].before.as_deref(), Some("FROM alpine\n"));
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn planning_a_creation_does_not_write_the_file() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");
        builder.finir();

        assert!(!project.path().join("Dockerfile").exists());
    }

    #[test]
    fn inserting_into_an_empty_anchor_is_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "src/router.rs");
        assert!(
            plan.files()[0]
                .after
                .contains(".merge(crate::users::routes())")
        );
    }

    #[test]
    fn inserting_an_already_present_line_is_done() {
        let project = project();
        with_router(
            &project,
            &ROUTER.replace(
                "        // </rbs:routes>",
                "        .merge(crate::users::routes())\n        // </rbs:routes>",
            ),
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(
            plan.files()[0].before.as_deref(),
            Some(plan.files()[0].after.as_str())
        );
    }

    #[test]
    fn two_insertions_in_one_file_chain_onto_a_single_file() {
        let project = project();
        let lib = "// <rbs:migration_modules>\n// </rbs:migration_modules>\nvec![\n    // <rbs:migrations>\n    // </rbs:migrations>\n]\n";
        fs::create_dir_all(project.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(project.path().join("migration/src/lib.rs"), lib).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::MIGRATION_MODULES,
                &["mod m20260826_creer_users;".to_string()],
            )
            .expect("l'ancre est présente");
        builder
            .insert(
                anchors::MIGRATIONS,
                &["Box::new(m20260826_creer_users::Migration),".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions().len(), 2);
        assert_eq!(plan.files().len(), 1);
        assert!(plan.files()[0].after.contains("mod m20260826_creer_users;"));
        assert!(
            plan.files()[0]
                .after
                .contains("Box::new(m20260826_creer_users::Migration),")
        );
        assert_eq!(plan.files()[0].before.as_deref(), Some(lib));
    }

    #[test]
    fn a_missing_anchor_stops_the_planning() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");

        assert!(matches!(error, Error::Anchor(_)));
    }

    #[test]
    fn inserting_into_a_missing_file_names_the_file_not_the_anchor() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("le fichier n'existe pas");

        assert!(matches!(error, Error::FichierAbsent { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("src/router.rs"), "{message}");
        assert!(
            !message.contains("<rbs:routes>"),
            "le message parle d'une balise alors que le fichier entier manque : {message}"
        );
    }

    /// Un fichier illisible n'est pas un fichier absent : seul `NotFound` vaut absence.
    #[test]
    fn inserting_into_an_unreadable_file_stays_an_access_error() {
        let project = project();
        fs::create_dir_all(project.path().join("src/router.rs")).expect("le répertoire se crée");
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(anchors::ROUTES, &["peu importe".to_string()])
            .expect_err("le fichier ne se lit pas");

        assert!(matches!(error, Error::Acces { .. }), "{error:?}");
    }

    #[test]
    fn inserting_into_a_present_file_without_the_anchor_stays_an_anchor_error() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");

        assert!(matches!(error, Error::Anchor(_)), "{error:?}");
        assert!(error.to_string().contains("<rbs:routes>"), "{error}");
    }

    const CARGO: &str = "[package]\nname = \"demo\"\n\n[package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = [\"health\"]\n";

    #[test]
    fn patching_a_missing_feature_is_todo() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "Cargo.toml");
        assert!(plan.files()[0].after.contains("\"docker\""));
    }

    #[test]
    fn patching_an_already_recorded_feature_is_done() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("health".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, CARGO);
    }

    #[test]
    fn patching_a_missing_manifest_is_reported() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect_err("le manifeste manque");

        assert!(matches!(error, Error::ManifesteAbsent { .. }));
        let message = error.to_string();
        assert!(
            message.starts_with("Cargo.toml"),
            "le message ne nomme pas le manifeste comme `Action::path` : {message}"
        );
        assert!(
            !message.contains(&project.path().display().to_string()),
            "le message porte un chemin absolu : {message}"
        );
        assert!(
            message.contains("introuvable"),
            "le message ne dit pas que le fichier manque : {message}"
        );
    }

    const CARGO_DEPS: &str = "[package]\nname = \"demo\"\n\n[dependencies]\naxum = \"0.9\"       # le serveur\n\n[package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = [\"health\"]\n";

    fn redis() -> PatchToml {
        PatchToml::AjouterDependance(crate::metadata::Dependency {
            name: "redis".to_string(),
            version: "0.32".to_string(),
            features: vec!["tokio-comp".to_string()],
            default_features: true,
        })
    }

    /// Écrit `manifest`, applique `patch`, et rend le plan obtenu.
    fn patched_plan(project: &TempDir, manifest: &str, patch: PatchToml) -> Plan {
        fs::write(project.path().join("Cargo.toml"), manifest).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder.patch(patch).expect("le manifeste est valide");

        builder.finir()
    }

    #[test]
    fn patching_a_missing_dependency_is_todo() {
        let project = project();

        let plan = patched_plan(&project, CARGO_DEPS, redis());

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "Cargo.toml");
        assert!(
            plan.files()[0]
                .after
                .contains(r#"redis = { version = "0.32", features = ["tokio-comp"] }"#),
            "{}",
            plan.files()[0].after
        );
    }

    #[test]
    fn patching_an_already_declared_dependency_is_done() {
        let project = project();
        let after = patched_plan(&project, CARGO_DEPS, redis()).files()[0]
            .after
            .clone();

        let plan = patched_plan(&project, &after, redis());

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, after);
    }

    #[test]
    fn patching_a_missing_dependency_feature_is_todo() {
        let project = project();
        let patch = PatchToml::AjouterFeatureADependance {
            dependency: "axum".to_string(),
            feature: "macros".to_string(),
        };

        let plan = patched_plan(&project, CARGO_DEPS, patch);

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert!(
            plan.files()[0].after.contains(
                r#"axum = { version = "0.9", features = ["macros"] }       # le serveur"#
            ),
            "{}",
            plan.files()[0].after
        );
    }

    #[test]
    fn patching_an_already_enabled_dependency_feature_is_done() {
        let project = project();
        let patch = || PatchToml::AjouterFeatureADependance {
            dependency: "axum".to_string(),
            feature: "macros".to_string(),
        };
        let after = patched_plan(&project, CARGO_DEPS, patch()).files()[0]
            .after
            .clone();

        let plan = patched_plan(&project, &after, patch());

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, after);
    }

    #[test]
    fn patching_a_feature_on_a_missing_dependency_is_reported() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO_DEPS).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .patch(PatchToml::AjouterFeatureADependance {
                dependency: "sea-orm".to_string(),
                feature: "with-uuid".to_string(),
            })
            .expect_err("la dépendance manque");

        assert!(matches!(error, Error::Metadata(_)), "{error}");
    }

    #[test]
    fn a_second_patch_of_the_same_feature_stays_todo() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(
            plan.actions()[1].statut,
            Status::AFaire,
            "le manifeste trouvé ne porte pas `docker` : l'action a bien un effet"
        );
    }

    #[test]
    fn a_second_insertion_of_the_same_line_stays_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());
        let lines = [".merge(crate::users::routes())".to_string()];

        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(
            plan.actions()[1].statut,
            Status::AFaire,
            "le routeur trouvé ne porte pas la ligne : l'action a bien un effet"
        );
        assert_eq!(
            plan.files()[0].after.matches("users::routes").count(),
            1,
            "la ligne a été insérée deux fois"
        );
    }

    #[test]
    fn creating_on_an_already_projected_path_is_rejected() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let error = builder
            .create("src/router.rs", ROUTER)
            .expect_err("une action a déjà projeté ce fichier");
        let plan = builder.finir();

        assert!(matches!(error, Error::DejaProjete { .. }));

        assert_eq!(plan.actions().len(), 1);
        assert!(
            plan.files()[0].after.contains("users::routes"),
            "la projection de l'insertion a été écrasée : {}",
            plan.files()[0].after
        );
    }

    #[test]
    fn creating_the_same_file_twice_is_rejected() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");
        let error = builder
            .create("Dockerfile", "FROM alpine\n")
            .expect_err("le fichier est déjà projeté");
        let plan = builder.finir();

        assert!(matches!(error, Error::DejaProjete { .. }));
        assert_eq!(plan.actions().len(), 1);
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn a_conflicting_file_says_so_without_reading_its_actions() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.files()[0].statut, Status::Conflit);
    }

    #[test]
    fn a_file_whose_every_action_is_a_no_op_is_a_no_op() {
        let project = project();
        let peuple = ROUTER.replace(
            "        // </rbs:routes>",
            "        .merge(crate::users::routes())\n        // </rbs:routes>",
        );
        with_router(&project, &peuple);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("src/router.rs", &peuple)
            .expect("le fichier se lit");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.actions()[1].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].statut, Status::DejaFait);
    }

    #[test]
    fn a_file_mixing_a_no_op_and_a_todo_action_is_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("src/router.rs", ROUTER)
            .expect("le fichier se lit");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.actions()[1].statut, Status::AFaire);
        assert_eq!(plan.files()[0].statut, Status::AFaire);
    }

    #[test]
    fn a_failed_action_leaves_neither_action_nor_file_in_the_plan() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");

        builder
            .insert(anchors::ROUTES, &["peu importe".to_string()])
            .expect_err("l'ancre manque");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect_err("le manifeste manque");
        builder
            .create("Dockerfile", "FROM alpine\n")
            .expect_err("le fichier est déjà projeté");

        let plan = builder.finir();

        assert_eq!(plan.actions().len(), 1, "une action en échec a été retenue");
        assert_eq!(plan.files().len(), 1, "un fichier en échec a été projeté");
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    /// Chemin et contenu de chaque fichier du répertoire, trié : deux empreintes égales
    /// valent répertoires identiques.
    fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut vus = Vec::new();
        let mut a_parcourir = vec![root.to_path_buf()];

        while let Some(directory) = a_parcourir.pop() {
            for input in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = input.expect("l'entrée se lit").path();
                if path.is_dir() {
                    a_parcourir.push(path);
                } else {
                    let relatif = path
                        .strip_prefix(root)
                        .expect("le chemin est sous la racine")
                        .display()
                        .to_string();
                    vus.push((relatif, fs::read(&path).expect("le fichier se lit")));
                }
            }
        }

        vus.sort();
        vus
    }

    /// Le critère du lot : ancre absente, rien d'écrit, et le bloc à recoller sous la main.
    #[test]
    fn a_missing_anchor_leaves_the_project_intact_and_gives_the_block_to_paste() {
        let project = project();
        with_router(
            &project,
            &ROUTER
                .replace("        // <rbs:routes>\n", "")
                .replace("        // </rbs:routes>\n", ""),
        );
        let before = fingerprint(project.path());

        let mut builder = Builder::new(project.path().to_path_buf());
        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");
        let plan = builder.finir();

        let Error::Anchor(absente) = error else {
            panic!("le fichier est là, seule l'ancre manque : {error:?}");
        };

        assert_eq!(
            fingerprint(project.path()),
            before,
            "l'échec de planification a touché au disque"
        );
        assert!(plan.files().is_empty(), "un fichier a été projeté");

        let block = absente.anchor.block();
        assert!(block.contains("// <rbs:routes>"), "{block}");
        assert!(block.contains("// </rbs:routes>"), "{block}");
    }

    /// Le critère du lot : une ligne déjà montée ne se réécrit pas au plan suivant.
    #[test]
    fn inserting_the_same_line_twice_changes_nothing_the_second_time() {
        let project = project();
        with_router(&project, ROUTER);
        let lines = [".merge(crate::users::routes())".to_string()];

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let premier = builder.finir();
        assert_eq!(premier.files()[0].statut, Status::AFaire);

        fs::write(
            project.path().join("src/router.rs"),
            &premier.files()[0].after,
        )
        .expect("l'écriture aboutit");

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let second = builder.finir();

        assert_eq!(second.files()[0].statut, Status::DejaFait);
        assert_eq!(
            second.files()[0].before.as_deref(),
            Some(second.files()[0].after.as_str()),
            "la seconde planification réécrirait le fichier"
        );
    }

    #[test]
    fn planning_does_not_modify_the_project_directory() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        with_router(&project, ROUTER);
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");

        let before = fingerprint(project.path());

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        builder
            .create("docker-compose.yml", "services:\n")
            .expect("le fichier est absent");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(
            fingerprint(project.path()),
            before,
            "la planification a touché au disque"
        );
        assert_eq!(plan.actions().len(), 4);
        assert_eq!(plan.files().len(), 4);
    }
}
