//! Traduit ce qu'un manifeste de fragment déclare en actions de plan.
//!
//! C'est ici, et nulle part ailleurs, qu'`add` cesse d'être générique : le manifeste est
//! lu, ses sections sont interprétées, et le CLI ne connaît toujours aucune feature par
//! son nom.

use std::path::Path;

use minijinja::Value;

use crate::anchors::{self, Anchor};
use crate::generate::mount;
use crate::manifest::Manifest;
use crate::metadata;
use crate::plan;
use crate::template::Renderer;
use crate::templates;

/// Où les variables d'environnement d'un fragment sont déclarées.
///
/// C'est `.env.example` et non `.env` : le second porte les secrets réels du développeur,
/// que rbs n'a pas à toucher.
const FICHIER_ENV: &str = ".env.example";

/// Le fragment tel que l'installation le voit.
pub(crate) struct Fragment<'a> {
    /// Nom de la feature, pour les messages d'erreur.
    pub name: &'a str,
    /// Ce que son manifeste déclare.
    pub manifest: &'a Manifest,
    /// Ses templates, telles que la source les a lues.
    pub templates: &'a [templates::File],
    /// Contexte de rendu, déduit du projet visé.
    pub context: Value,
    /// Horodatage que portera la migration du fragment.
    ///
    /// Il est reçu et non lu de l'horloge : une planification doit être reproductible.
    pub timestamp: &'a str,
}

/// Ce qui peut empêcher d'interpréter un manifeste.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Le manifeste désigne une template que le fragment ne porte pas.
    #[error("{feature}/feature.toml déclare `{template}`, absente du fragment")]
    TemplateAbsente {
        /// Feature en cours d'installation.
        feature: String,
        /// Template introuvable, telle que le manifeste la nomme.
        template: String,
    },

    /// Le manifeste vise une ancre que le squelette ne porte pas.
    #[error("{feature}/feature.toml vise l'ancre `{anchor}`, qui n'existe pas : {known}")]
    AncreInconnue {
        /// Feature en cours d'installation.
        feature: String,
        /// Nom d'ancre refusé.
        anchor: String,
        /// Les ancres du squelette, énumérées.
        known: String,
    },

    /// Une template ne s'est pas rendue.
    #[error("{file} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        file: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// L'action n'a pas pu être planifiée.
    #[error("{0}")]
    Plan(#[from] plan::Error),
}

/// Ajoute au plan ce que le manifeste déclare, et rend les chemins déposés.
pub(crate) fn actions(
    fragment: &Fragment,
    builder: &mut plan::Builder,
) -> Result<Vec<String>, Error> {
    let renderer = Renderer::new();
    let mut deposes = Vec::new();

    for (destination, source) in a_deposer(fragment)? {
        let content = render(&renderer, fragment, source, &destination)?;

        builder.create(&destination, &content)?;
        deposes.push(destination);
    }

    if let Some(declared) = &fragment.manifest.migration {
        // Le format est celui de `generate crud` et de `migrate new` : deux formats
        // d'horodatage dans un même projet, ce sont deux ordres de migration possibles.
        let module = format!("m{}_{}", fragment.timestamp, declared.name);
        let path = format!("migration/src/{module}.rs");
        let content = render(
            &renderer,
            fragment,
            template(fragment, &declared.source)?,
            &path,
        )?;

        builder.create(&path, &content)?;
        deposes.push(path);

        for mount in mount::for_migration(&module) {
            builder.insert(mount.anchor, &mount.lines)?;
        }
    }

    for insertion in &fragment.manifest.anchors {
        let anchor = anchor(fragment, &insertion.anchor)?;
        builder.insert(anchor, &lines(&insertion.content))?;
    }

    // Avant les features de `[cargo.<crate>]` : activer une feature suppose la dépendance
    // déclarée, et un fragment peut fort bien viser une crate qu'il apporte lui-même.
    for declared in &fragment.manifest.dependencies {
        builder.patch(plan::PatchToml::AjouterDependance(metadata::Dependency {
            name: declared.name.clone(),
            version: declared.version.clone(),
            features: declared.features.clone(),
            default_features: declared.default_features,
        }))?;
    }

    for (crate_, patch) in &fragment.manifest.cargo {
        for feature in &patch.features {
            builder.patch(plan::PatchToml::AjouterFeatureADependance {
                dependency: crate_.clone(),
                feature: feature.clone(),
            })?;
        }
    }

    for section in &fragment.manifest.config {
        builder.add_section(&section.file, &section.section, &section.content)?;
    }

    for variable in &fragment.manifest.env {
        builder.add_variable(
            FICHIER_ENV,
            &variable.key,
            &variable.value,
            variable.comment.as_deref(),
        )?;
    }

    Ok(deposes)
}

/// L'ancre du squelette que le manifeste désigne par `name`.
///
/// Un nom inconnu est une faute du manifeste : l'ignorer installerait une feature dont
/// le montage manquerait, ce que seule la compilation du projet révélerait.
fn anchor(fragment: &Fragment, name: &str) -> Result<Anchor, Error> {
    anchors::ANCRES
        .into_iter()
        .find(|anchor| anchor.name == name)
        .ok_or_else(|| Error::AncreInconnue {
            feature: fragment.name.to_string(),
            anchor: name.to_string(),
            known: anchors::ANCRES
                .iter()
                .map(|anchor| anchor.name)
                .collect::<Vec<_>>()
                .join(", "),
        })
}

/// Découpe le contenu déclaré en lignes à insérer.
///
/// Une ancre en reçoit souvent plusieurs — les cinq chemins OpenAPI d'une feature — et
/// une chaîne TOML multiligne est la façon naturelle de les écrire.
fn lines(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim_end().to_string())
        .collect()
}

/// Rend `source` dans le contexte du fragment, en nommant `destination` si elle échoue.
fn render(
    renderer: &Renderer,
    fragment: &Fragment,
    source: &str,
    destination: &str,
) -> Result<String, Error> {
    renderer
        .render(source, fragment.context.clone())
        .map_err(|source| Error::Rendu {
            file: destination.to_string(),
            source,
        })
}

/// Les templates à déposer, avec leur chemin dans le projet.
///
/// Sans `[[files]]`, le fragment est copié tel quel : un fragment qui n'apporte pas de
/// code Rust n'a rien à déclarer pour que ses fichiers arrivent où leur arborescence les
/// place déjà.
fn a_deposer<'a>(fragment: &'a Fragment) -> Result<Vec<(String, &'a str)>, Error> {
    if fragment.manifest.files.is_empty() {
        return Ok(fragment
            .templates
            .iter()
            .map(|template| {
                (
                    template.destination.to_string_lossy().into_owned(),
                    template.source.as_str(),
                )
            })
            .collect());
    }

    fragment
        .manifest
        .files
        .iter()
        .map(|declare| {
            let source = template(fragment, &declare.source)?;
            Ok((declare.destination.clone(), source))
        })
        .collect()
}

/// La source de la template que le manifeste désigne par `name`.
fn template<'a>(fragment: &'a Fragment, name: &str) -> Result<&'a str, Error> {
    fragment
        .templates
        .iter()
        .find(|template| template.origin == Path::new(name))
        .map(|template| template.source.as_str())
        .ok_or_else(|| Error::TemplateAbsente {
            feature: fragment.name.to_string(),
            template: name.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use minijinja::context;
    use tempfile::TempDir;

    use super::*;
    use crate::manifest;

    /// Une template du fragment, telle que la source la restituerait.
    fn template_of(origin: &str, source: &str) -> templates::File {
        let origin = PathBuf::from(origin);
        let destination = if origin.extension().is_some_and(|suffixe| suffixe == "jinja") {
            origin.with_extension("")
        } else {
            origin.clone()
        };

        templates::File {
            destination,
            origin,
            source: source.to_string(),
        }
    }

    /// Planifie l'installation d'un fragment décrit par son manifeste et ses templates.
    fn plan_for(
        root: &Path,
        manifest: &str,
        templates: &[templates::File],
    ) -> Result<(Vec<String>, plan::Plan), Error> {
        let manifest = manifest::read(manifest, "essai/feature.toml")
            .expect("le manifeste du test doit être valide");
        let mut builder = plan::Builder::new(root.to_path_buf());

        let deposes = actions(
            &Fragment {
                name: "essai",
                manifest: &manifest,
                templates,
                context: context! { nom_projet => "demo-api", crate_name => "demo_api" },
                timestamp: "20260827_120000",
            },
            &mut builder,
        )?;

        Ok((deposes, builder.finir()))
    }

    #[test]
    fn without_a_files_section_the_fragment_is_copied_as_is() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        let templates = [
            template_of("Dockerfile.jinja", "FROM rust\n"),
            template_of(".dockerignore", "target\n"),
        ];

        let (deposes, plan) = plan_for(
            project.path(),
            "[feature]\ndescription = \"docker\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["Dockerfile", ".dockerignore"]);
        assert_eq!(plan.files().len(), 2);
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn a_declared_template_is_written_to_the_destination_beside_it() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        let templates = [template_of(
            "model.rs.jinja",
            "// {@ crate_name @}\npub struct User;\n",
        )];

        let (deposes, plan) = plan_for(
            project.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[files]]\nsource = \"model.rs.jinja\"\ndestination = \"src/auth/model.rs\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["src/auth/model.rs"]);
        assert_eq!(plan.files()[0].path, "src/auth/model.rs");
        assert_eq!(plan.files()[0].after, "// demo_api\npub struct User;\n");
    }

    /// Un manifeste de projet réaliste : une dépendance nue, une commentée en fin de
    /// ligne, et un commentaire de bloc.
    const CARGO: &str = "\
[package]
name = \"demo-api\"

[dependencies]
# le noyau, épinglé par `rbs new`
rbs-core = \"0.1.0\"   # ne pas remonter sans relire le CHANGELOG
axum = \"0.8\"
";

    /// Un fragment qui n'exerce que les patchs : manifeste, configuration, environnement.
    const PATCHS: &str = "[feature]\ndescription = \"auth\"\n\n\
         [cargo.rbs-core]\nfeatures = [\"auth\"]\n\n\
         [[config]]\nfile = \"config/default.toml\"\nsection = \"auth\"\n\
         content = \"\"\"\naccess_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n\"\"\"\n\n\
         [[env]]\nkey = \"RBS_AUTH__SECRET\"\nvalue = \"changez-moi\"\n\
         comment = \"Secret de signature HS256, au moins 32 octets\"\n";

    /// Pose les fichiers du projet que les patchs viseront.
    fn avec(root: &Path, files: &[(&str, &str)]) {
        for (path, content) in files {
            let destination = root.join(path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).expect("le répertoire se crée");
            }
            std::fs::write(destination, content).expect("le fichier s'écrit");
        }
    }

    /// Le contenu que le plan projette pour `path`.
    fn projected<'plan>(plan: &'plan plan::Plan, path: &str) -> &'plan str {
        &plan
            .files()
            .iter()
            .find(|file| file.path == path)
            .unwrap_or_else(|| panic!("{path} absent du plan"))
            .after
    }

    /// Le critère de la tâche : le patch touche une ligne et laisse les autres intactes.
    #[test]
    fn rbs_core_gains_the_feature_without_the_rest_being_reformatted() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        avec(
            project.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = plan_for(project.path(), PATCHS, &[]).expect("le plan doit se calculer");
        let after = projected(&plan, "Cargo.toml");

        let attendues = CARGO.lines().count();
        assert_eq!(after.lines().count(), attendues, "{after}");

        for (rang, (before, after)) in CARGO.lines().zip(after.lines()).enumerate() {
            if before.starts_with("rbs-core") {
                continue;
            }
            assert_eq!(before, after, "la ligne {} a été reformatée", rang + 1);
        }
    }

    /// Le critère de la tâche : ce que le développeur a annoté lui appartient.
    #[test]
    fn the_developers_comments_survive_the_patch() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        avec(
            project.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = plan_for(project.path(), PATCHS, &[]).expect("le plan doit se calculer");
        let after = projected(&plan, "Cargo.toml");

        assert!(
            after.contains("# le noyau, épinglé par `rbs new`"),
            "le commentaire de bloc a disparu :\n{after}"
        );
        assert!(
            after.contains(
                "rbs-core = { version = \"0.1.0\", features = [\"auth\"] }   \
                 # ne pas remonter sans relire le CHANGELOG"
            ),
            "le commentaire de fin de ligne a disparu :\n{after}"
        );
    }

    /// Le critère de la tâche : la configuration et l'environnement du fragment arrivent.
    #[test]
    fn the_configuration_section_and_the_environment_variable_are_added() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        avec(
            project.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = plan_for(project.path(), PATCHS, &[]).expect("le plan doit se calculer");

        let config = projected(&plan, "config/default.toml");
        assert!(config.starts_with("[server]\nport = 8080\n"), "{config}");
        assert!(config.contains("[auth]"), "{config}");
        assert!(config.contains("access_ttl_secs = 900"), "{config}");
        assert!(config.contains("refresh_ttl_secs = 2592000"), "{config}");

        let env = projected(&plan, ".env.example");
        assert!(env.starts_with("RBS_DATABASE__URL=postgres://\n"), "{env}");
        assert!(
            env.contains(
                "# Secret de signature HS256, au moins 32 octets\nRBS_AUTH__SECRET=changez-moi"
            ),
            "{env}"
        );
    }

    /// Le critère de la tâche : un patch déjà posé ne se repose pas.
    #[test]
    fn the_three_patches_are_no_ops_the_second_time() {
        let project = TempDir::new().expect("répertoire temporaire créable");
        avec(
            project.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, premier) = plan_for(project.path(), PATCHS, &[]).expect("le plan doit se calculer");
        for file in premier.files() {
            avec(project.path(), &[(&file.path, &file.after)]);
        }

        let (_, second) = plan_for(project.path(), PATCHS, &[]).expect("le plan se recalcule");

        for file in second.files() {
            assert_eq!(
                file.statut,
                plan::Status::DejaFait,
                "{} n'est pas sans effet :\n{}",
                file.path,
                file.after
            );
        }
    }

    /// Un fragment qui n'apporte que des crates tierces, versions figées comme le veut le
    /// moule. `axum` est déjà déclarée par le projet, `lettre` non.
    const DEPENDANCES: &str = "[feature]\ndescription = \"mail\"\n\n\
         [[dependencies]]\nname = \"lettre\"\nversion = \"0.11\"\n\
         default_features = false\nfeatures = [\"smtp-transport\", \"builder\"]\n\n\
         [[dependencies]]\nname = \"axum\"\nversion = \"0.8\"\n";

    /// Le manifeste que l'installation de `DEPENDANCES` projette sur `CARGO`.
    fn cargo_after_dependencies(project: &TempDir) -> String {
        avec(project.path(), &[("Cargo.toml", CARGO)]);

        let (_, plan) =
            plan_for(project.path(), DEPENDANCES, &[]).expect("le plan doit se calculer");

        projected(&plan, "Cargo.toml").to_string()
    }

    /// Le critère de la tâche : la version, les features et le `default-features` du
    /// fragment arrivent tels quels dans `[dependencies]`.
    #[test]
    fn the_declared_dependency_arrives_with_its_version_its_features_and_its_default_features() {
        let project = TempDir::new().expect("répertoire temporaire créable");

        let after = cargo_after_dependencies(&project);

        assert!(
            after.contains(
                "lettre = { version = \"0.11\", default-features = false, \
                 features = [\"smtp-transport\", \"builder\"] }"
            ),
            "{after}"
        );
    }

    /// Le critère de la tâche : ce que le développeur a écrit et annoté lui appartient.
    #[test]
    fn comments_and_formatting_survive_adding_a_dependency() {
        let project = TempDir::new().expect("répertoire temporaire créable");

        let after = cargo_after_dependencies(&project);

        for line in CARGO.lines() {
            assert!(
                after.lines().any(|rendue| rendue == line),
                "la ligne « {line} » a été reformatée :\n{after}"
            );
        }
        assert_eq!(
            after.lines().count(),
            CARGO.lines().count() + 1,
            "le patch a débordé de la ligne qu'il ajoute :\n{after}"
        );
    }

    /// Le critère de la tâche : une crate que le projet déclare déjà reste déclarée une
    /// fois. Sans quoi cargo refuserait le manifeste que le fragment vient d'écrire.
    #[test]
    fn a_dependency_already_declared_in_the_project_is_not_duplicated() {
        let project = TempDir::new().expect("répertoire temporaire créable");

        let after = cargo_after_dependencies(&project);

        assert_eq!(
            after
                .lines()
                .filter(|line| line.starts_with("axum"))
                .count(),
            1,
            "{after}"
        );
        assert!(after.contains("axum = \"0.8\"\n"), "{after}");
    }

    /// Une template déclarée mais absente est une faute du manifeste, pas un silence.
    #[test]
    fn a_declared_but_missing_template_is_reported_by_its_name() {
        let project = TempDir::new().expect("répertoire temporaire créable");

        let error = plan_for(
            project.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[files]]\nsource = \"absente.rs.jinja\"\ndestination = \"src/auth/model.rs\"\n",
            &[],
        )
        .expect_err("la template n'existe pas");

        assert!(matches!(error, Error::TemplateAbsente { .. }), "{error}");
        assert!(error.to_string().contains("absente.rs.jinja"), "{error}");
    }
}
