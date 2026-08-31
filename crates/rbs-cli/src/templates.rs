//! Provenance et lecture des templates : le squelette de projet, et les fragments de
//! feature qu'`rbs add` y dépose.
//!
//! Le binaire porte les deux arborescences en lui, pour qu'une installation depuis
//! crates.io n'ait besoin d'aucun fichier externe ; `--template-dir` leur substitue un
//! répertoire du disque, ce dont le développement de rbs a besoin à chaque retouche d'une
//! template.

use std::io;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

/// Suffixe que porte toute template, et que ne porte aucune destination.
const SUFFIXE: &str = "jinja";

/// Nom du manifeste d'un fragment, à la racine de son répertoire.
const MANIFESTE: &str = "feature.toml";

/// Le squelette de projet, embarqué au moment de la compilation du binaire.
static PROJET: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/project");

/// Les fragments de feature, un sous-répertoire par feature installable.
static FEATURES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/features");

/// Les guides `AGENTS.md`, une template par langue.
static AGENTS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/agents");

/// Provenance des templates.
#[derive(Debug)]
pub enum Source {
    /// Une arborescence embarquée dans le binaire.
    Embarquees(&'static Dir<'static>),
    /// Un répertoire du disque, donné par `--template-dir`.
    Repertoire(PathBuf),
}

/// Une feature dont aucun fragment n'existe, ni embarqué ni sur le disque.
#[derive(Debug, thiserror::Error)]
#[error("`{feature}` n'est pas une feature installable : {known}")]
pub struct Unknown {
    /// Nom demandé.
    pub feature: String,
    /// Les features que la source propose, énumérées.
    pub known: String,
}

/// Une template et le chemin auquel son rendu sera écrit.
#[derive(Debug)]
pub struct File {
    /// Chemin de sortie relatif à la racine du projet, suffixe `.jinja` retiré.
    pub destination: PathBuf,
    /// Chemin de la template dans sa source, suffixe compris.
    ///
    /// C'est par lui qu'un manifeste de fragment désigne une template : la destination,
    /// elle, y est déclarée séparément.
    pub origin: PathBuf,
    /// Source de la template, telle quelle : le rendu est l'affaire de l'appelant.
    pub source: String,
}

impl Source {
    /// Retient le répertoire donné par `--template-dir`, ou l'embarqué à défaut.
    pub fn fresh(directory: Option<&Path>) -> Self {
        match directory {
            Some(path) => Self::Repertoire(path.to_path_buf()),
            None => Self::Embarquees(&PROJET),
        }
    }

    /// S'ouvre sur les guides `AGENTS.md`, toujours dans l'embarqué.
    ///
    /// Les guides ne font pas partie du squelette substituable par `--template-dir` : ils
    /// ne sont pas rendus dans l'arborescence que `Source::fresh` produit, ils composent un
    /// fichier à part. Un utilisateur qui fournit son propre squelette n'a aucune raison
    /// d'y ajouter des guides d'agent, et les lire dans son répertoire ferait échouer
    /// `rbs new --template-dir` sur un squelette par ailleurs valide.
    pub fn agents() -> Self {
        Self::Embarquees(&AGENTS)
    }

    /// S'ouvre sur le fragment d'une feature, sous le répertoire donné ou dans l'embarqué.
    ///
    /// Une feature sans fragment est refusée ici plutôt qu'au rendu : un catalogue vide
    /// produirait un plan vide, donc une commande qui réussit sans rien faire.
    pub fn feature(directory: Option<&Path>, feature: &str) -> Result<Self, Unknown> {
        match directory {
            Some(path) => {
                let fragment = path.join(feature);

                if fragment.is_dir() {
                    Ok(Self::Repertoire(fragment))
                } else {
                    Err(Unknown {
                        feature: feature.to_owned(),
                        known: enumerate(names_on_disk(path)),
                    })
                }
            }
            None => FEATURES
                .get_dir(feature)
                .map(Self::Embarquees)
                .ok_or_else(|| Unknown {
                    feature: feature.to_owned(),
                    known: enumerate(embedded_names()),
                }),
        }
    }

    /// Lit toutes les templates, triées par destination.
    ///
    /// Le tri n'est pas cosmétique : `include_dir` et `fs::read_dir` ne rendent pas leurs
    /// entrées dans le même ordre, et le second n'en garantit aucun.
    ///
    /// Le manifeste d'un fragment est écarté : il déclare ce que l'installation fait au
    /// projet, il n'est pas un des fichiers qu'elle y dépose.
    pub fn files(&self) -> io::Result<Vec<File>> {
        let mut files = self.all()?;

        files.retain(|file| file.destination != Path::new(MANIFESTE));

        Ok(files)
    }

    /// Source du manifeste du fragment, ou `None` s'il n'en porte pas.
    pub fn manifest(&self) -> io::Result<Option<String>> {
        Ok(self
            .all()?
            .into_iter()
            .find(|file| file.destination == Path::new(MANIFESTE))
            .map(|file| file.source))
    }

    /// Toutes les entrées du répertoire, manifeste compris.
    fn all(&self) -> io::Result<Vec<File>> {
        let mut files = Vec::new();

        match self {
            Self::Embarquees(root) => read_embedded(root, root.path(), &mut files)?,
            Self::Repertoire(root) => read_directory(root, root, &mut files)?,
        }

        files.sort_by(|gauche, droite| gauche.destination.cmp(&droite.destination));

        Ok(files)
    }
}

/// Les features dont le binaire porte un fragment, triées.
pub(crate) fn embedded_names() -> Vec<String> {
    let mut names: Vec<String> = FEATURES
        .dirs()
        .filter_map(|dir| dir.path().file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();

    names.sort();
    names
}

/// Les features qu'un `--template-dir` propose, triées, ou rien s'il est illisible.
fn names_on_disk(directory: &Path) -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entrees
        .flatten()
        .filter(|input| input.path().is_dir())
        .map(|input| input.file_name().to_string_lossy().into_owned())
        .collect();

    names.sort();
    names
}

/// Les features installables, celles du `--template-dir` s'il en désigne un.
///
/// Une seule liste pour la question de `rbs new`, la validation de `--with` et le message
/// qui énumère les features connues : trois listes écrites à la main avaient divergé, et
/// `jobs` manquait à celle qui décidait.
pub(crate) fn feature_names(directory: Option<&Path>) -> Vec<String> {
    match directory {
        Some(directory) => names_on_disk(directory),
        None => embedded_names(),
    }
}

/// Rend une liste de features lisible dans un message d'erreur.
fn enumerate(names: Vec<String>) -> String {
    if names.is_empty() {
        "aucune n'est disponible".to_string()
    } else {
        names.join(", ")
    }
}

fn read_embedded(directory: &Dir<'static>, base: &Path, files: &mut Vec<File>) -> io::Result<()> {
    for sous_repertoire in directory.dirs() {
        read_embedded(sous_repertoire, base, files)?;
    }

    for file in directory.files() {
        // Une template non-UTF-8 est une template qu'aucun rendu ne traversera : la
        // laisser passer déplacerait l'échec dans l'écriture du projet.
        let source = file.contents_utf8().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} n'est pas de l'UTF-8", file.path().display()),
            )
        })?;

        // Le chemin d'un fichier embarqué est relatif à la racine de l'`include_dir!`, et
        // non au fragment ouvert : sans ce retrait, `add docker` viserait `docker/Dockerfile`.
        let relatif = file.path().strip_prefix(base).unwrap_or(file.path());

        files.push(File {
            destination: destination(relatif),
            origin: relatif.to_path_buf(),
            source: source.to_owned(),
        });
    }

    Ok(())
}

fn read_directory(root: &Path, directory: &Path, files: &mut Vec<File>) -> io::Result<()> {
    let entrees = std::fs::read_dir(directory).map_err(|error| name_of(directory, error))?;

    for input in entrees {
        let path = input.map_err(|error| name_of(directory, error))?.path();

        if path.is_dir() {
            read_directory(root, &path, files)?;
            continue;
        }

        let source = std::fs::read_to_string(&path).map_err(|error| name_of(&path, error))?;
        let relatif = path.strip_prefix(root).unwrap_or(&path);

        files.push(File {
            destination: destination(relatif),
            origin: relatif.to_path_buf(),
            source,
        });
    }

    Ok(())
}

/// Retire le suffixe `.jinja` du chemin d'une template.
///
/// C'est l'unique endroit du CLI où la convention du §1 du design du squelette
/// s'applique : tout le reste du code ne voit que des destinations. Un chemin sans
/// suffixe traverse intact — le refuser transformerait la faute de frappe d'un
/// `--template-dir` en erreur incompréhensible.
fn destination(template: &Path) -> PathBuf {
    if template
        .extension()
        .is_some_and(|suffixe| suffixe == SUFFIXE)
    {
        template.with_extension("")
    } else {
        template.to_path_buf()
    }
}

/// Rejoue une erreur d'entrée-sortie en nommant le chemin en cause.
///
/// Un `--template-dir` mal saisi est l'erreur la plus probable de ce flag, et
/// « No such file or directory » seul ne la corrige pas.
fn name_of(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{} : {error}", path.display()))
}

#[cfg(test)]
mod tests {
    //! Ces templates sont une interface : les commandes de génération écrivent dans leurs
    //! ancres, et un projet déjà déroulé ne bénéficie d'aucune correction faite après
    //! coup. On vérifie donc en permanence ce qui ne dépend pas d'un rendu complet — la
    //! convention de nommage, les ancres, et l'absence de variable non déclarée.

    use std::fs;
    use std::path::{Path, PathBuf};

    use minijinja::{Value, context};

    use super::*;
    use crate::database::Database;
    use crate::template::Renderer;

    /// Racine des templates du squelette, résolue depuis la crate plutôt que depuis le
    /// répertoire courant, que `cargo test` ne garantit pas.
    const RACINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/project");

    /// Les chemins de sortie attendus du squelette, tels que `rbs new` les écrira.
    ///
    /// `docker-compose.yml` en fait partie : la source les rend tous, et c'est `rbs new`
    /// qui l'écarte pour un projet qui n'a rien à monter.
    const DESTINATIONS: [&str; 19] = [
        ".env",
        ".env.example",
        ".gitignore",
        "Cargo.toml",
        "config/default.toml",
        "config/development.toml",
        "config/production.toml",
        "docker-compose.yml",
        "migration/Cargo.toml",
        "migration/src/lib.rs",
        "migration/src/main.rs",
        "src/health/controller.rs",
        "src/health/mod.rs",
        "src/lib.rs",
        "src/main.rs",
        "src/openapi.rs",
        "src/router.rs",
        "src/seeds/main.rs",
        "src/state.rs",
    ];

    /// Contexte de rendu minimal : les variables que `rbs new` fournira.
    /// Le contexte que `new::render` construit, recopié ici.
    ///
    /// La divergence se voit : une variable ajoutée là-bas et oubliée ici fait tomber
    /// `each_template_renders_with_the_context_of_a_creation` sur la template qui
    /// l'emploie.
    fn context() -> Value {
        let database = Database::default();

        context! {
            project_name => "mon-api",
            crate_name => "mon_api",
            rbs_core_dep => "\"0.1\"",
            rbs_version => "0.1.0",
            database_url => "postgres://postgres:postgres@localhost:5432/mon_api",
            database => database.name(),
            sea_orm_feature => database.sea_orm_feature(),
            database_url_par_defaut => database.default_url("mon_api"),
            database_user => "postgres",
            database_password => "postgres",
            database_name => "mon_api",
            database_port => 5432,
            lang => "fr",
        }
    }

    /// Toutes les templates du squelette, répertoires imbriqués compris.
    fn templates() -> Vec<PathBuf> {
        let mut trouvees = Vec::new();
        walk(Path::new(RACINE), &mut trouvees);

        assert!(
            !trouvees.is_empty(),
            "aucune template trouvée sous {RACINE}"
        );

        trouvees
    }

    fn walk(directory: &Path, trouvees: &mut Vec<PathBuf>) {
        let entrees = fs::read_dir(directory).unwrap_or_else(|error| {
            panic!("{} illisible : {error}", directory.display());
        });

        for input in entrees {
            let path = input.expect("entrée de répertoire lisible").path();
            if path.is_dir() {
                walk(&path, trouvees);
            } else {
                trouvees.push(path);
            }
        }
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|error| {
            panic!("{} illisible : {error}", path.display());
        })
    }

    #[test]
    fn each_template_carries_the_jinja_suffix() {
        for path in templates() {
            assert_eq!(
                path.extension().and_then(|suffixe| suffixe.to_str()),
                Some("jinja"),
                "{} ne porte pas le suffixe `.jinja`",
                path.display()
            );
        }
    }

    #[test]
    fn the_features_anchor_follows_the_skeleton_modules_in_the_library() {
        let source = read(&Path::new(RACINE).join("src/lib.rs.jinja"));

        let modules = source
            .find("pub mod state;")
            .expect("les modules du squelette doivent être déclarés");
        let anchor = source
            .find("// <rbs:features>")
            .expect("lib.rs doit porter l'ancre des features");

        assert!(
            modules < anchor,
            "l'ancre doit suivre les modules du squelette :\n{source}"
        );
    }

    #[test]
    fn each_anchor_is_opened_then_closed_in_its_file() {
        for anchor in crate::anchors::ANCRES {
            // L'ancre des features vise `src/lib.rs` depuis ce jalon : c'est là que le
            // squelette la rend, `src/main.rs` n'étant qu'un repli pour un projet plus
            // ancien, sans bibliothèque.
            let anchor = if anchor.name == crate::anchors::FEATURES.name {
                anchor.in_file("src/lib.rs")
            } else {
                anchor
            };
            let relatif = format!("{}.jinja", anchor.file);
            let chemin = Path::new(RACINE).join(&relatif);

            // Une ancre optionnelle peut vivre dans un fichier qu'un fragment dépose et
            // que le squelette ne rend pas : il n'y a alors aucune template à contrôler.
            if anchor.optional && !chemin.exists() {
                continue;
            }

            let source = read(&chemin);

            let opening = anchor.opening();
            let closing = anchor.closing();

            assert_eq!(
                source.matches(&opening).count(),
                1,
                "{relatif} doit porter une fois `{opening}`"
            );
            assert_eq!(
                source.matches(&closing).count(),
                1,
                "{relatif} doit porter une fois `{closing}`"
            );
            assert!(
                source.find(&opening) < source.find(&closing),
                "{relatif} referme `{}` avant de l'ouvrir",
                anchor.name
            );
        }
    }

    #[test]
    fn each_template_renders_with_the_context_of_a_creation() {
        let renderer = Renderer::new();

        for path in templates() {
            let source = read(&path);
            renderer.render(&source, context()).unwrap_or_else(|error| {
                panic!("{} ne se rend pas : {error}", path.display());
            });
        }
    }

    #[test]
    fn each_rust_template_of_the_skeleton_conforms_to_rustfmt() {
        // Le workflow d'`rbs add ci` lance `cargo fmt --check` sur le projet généré : un
        // squelette non conforme le fait échouer au premier pas, sur du code que le
        // développeur n'a pas écrit.
        //
        // Le squelette est déroulé en entier plutôt que fichier par fichier : rustfmt suit
        // les déclarations de modules, et un `main.rs` seul ne résout pas ses `mod`.
        let renderer = Renderer::new();
        let temp = tempfile::tempdir().expect("répertoire temporaire créable");
        let root = temp.path();

        let files = Source::fresh(None)
            .files()
            .expect("les templates embarquées doivent se lire");

        let mut sources = Vec::new();
        for file in &files {
            let destination = root.join(&file.destination);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("le répertoire est créable");
            }

            let rendered = renderer
                .render(&file.source, context())
                .unwrap_or_else(|error| {
                    panic!("{} ne se rend pas : {error}", file.destination.display())
                });
            fs::write(&destination, rendered).expect("le rendu est écrivable");

            if destination
                .extension()
                .is_some_and(|suffixe| suffixe == "rs")
            {
                sources.push((file.destination.clone(), destination));
            }
        }

        assert!(
            !sources.is_empty(),
            "le squelette ne porte aucun fichier Rust"
        );

        for (relatif, path) in sources {
            let output = std::process::Command::new("rustfmt")
                .args(["--edition", "2024", "--check"])
                .arg(&path)
                .output()
                .expect("rustfmt doit être lançable");

            assert!(
                output.status.success(),
                "{} n'est pas conforme à rustfmt :\n{}{}",
                relatif.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn the_rendered_manifest_carries_the_project_name_and_the_core_dependency() {
        let source = read(&Path::new(RACINE).join("Cargo.toml.jinja"));

        let rendered = Renderer::new()
            .render(&source, context())
            .expect("le manifeste doit se rendre");

        assert!(
            rendered.contains("name = \"mon-api\""),
            "nom du paquet absent du manifeste rendu :\n{rendered}"
        );
        assert!(
            rendered.contains("rbs-core = \"0.1\""),
            "dépendance au noyau absente du manifeste rendu :\n{rendered}"
        );
    }

    #[test]
    fn the_embedded_source_yields_the_skeleton_with_its_output_paths() {
        let files = Source::fresh(None)
            .files()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = files
            .iter()
            .map(|file| file.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, DESTINATIONS);

        for file in &files {
            assert!(
                !file.source.is_empty(),
                "{} est embarquée vide",
                file.destination.display()
            );
        }
    }

    #[test]
    fn no_destination_carries_the_jinja_suffix() {
        let files = Source::fresh(None)
            .files()
            .expect("les templates embarquées doivent se lire");

        for file in files {
            assert_ne!(
                file.destination.extension(),
                Some("jinja".as_ref()),
                "{} garde le suffixe `.jinja`",
                file.destination.display()
            );
        }
    }

    #[test]
    fn a_templates_directory_takes_precedence_over_the_embedded_one() {
        let directory = tempfile::tempdir().expect("répertoire temporaire créable");
        fs::create_dir(directory.path().join("config")).expect("sous-répertoire créable");
        fs::write(
            directory.path().join("Cargo.toml.jinja"),
            "name = \"surcharge\"",
        )
        .expect("template écrivable");
        fs::write(
            directory.path().join("config/default.toml.jinja"),
            "port = 1",
        )
        .expect("template écrivable");

        let files = Source::fresh(Some(directory.path()))
            .files()
            .expect("le répertoire doit se lire");

        let destinations: Vec<&Path> = files
            .iter()
            .map(|file| file.destination.as_path())
            .collect();

        // Comparer des `Path` et non leur rendu : sous Windows, `config/default.toml`
        // s'affiche `config\default.toml`, et l'assertion parlerait du séparateur au
        // lieu de parler de l'ordre des fichiers.
        assert_eq!(
            destinations,
            [
                Path::new("Cargo.toml"),
                &Path::new("config").join("default.toml")
            ]
        );
        assert_eq!(files[0].source, "name = \"surcharge\"");
    }

    #[test]
    fn a_nonexistent_templates_directory_fails_naming_the_path() {
        let absent = Path::new("/introuvable/templates/rbs");

        let error = Source::fresh(Some(absent))
            .files()
            .expect_err("un répertoire absent ne doit pas rendre une liste vide");

        assert!(
            error.to_string().contains("/introuvable/templates/rbs"),
            "le message ne nomme pas le chemin : {error}"
        );
    }

    /// Racine des fragments de feature, résolue comme celle du squelette.
    const RACINE_FEATURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features");

    /// Les chemins de sortie attendus de `docker`, tels que `rbs add docker` les écrira.
    const DESTINATIONS_DOCKER: [&str; 4] = [
        ".dockerignore",
        "Dockerfile",
        "config/production.toml",
        "docker-compose.yml",
    ];

    /// Contexte de rendu d'un fragment : les deux variables qu'un projet existant fournit.
    /// Le contexte que `add::plan_for` construit, recopié ici.
    fn feature_context() -> Value {
        let database = Database::default();

        context! {
            project_name => "mon-api",
            crate_name => "mon_api",
            database => database.name(),
            database_a_un_serveur => database.a_un_serveur(),
            database_url_compose => database.compose_url("mon_api"),
            database_url_par_defaut => database.default_url("mon_api"),
            database_user => "rbs",
            database_password => "rbs",
            database_name => "mon_api",
            database_port => 5432u16,
        }
    }

    /// Toutes les templates de tous les fragments de feature.
    ///
    /// Le manifeste d'un fragment n'en est pas une : il décrit l'installation, il n'y
    /// est pas déposé.
    fn feature_templates() -> Vec<PathBuf> {
        let mut trouvees = Vec::new();
        walk(Path::new(RACINE_FEATURES), &mut trouvees);
        trouvees.retain(|path| path.file_name() != Some(MANIFESTE.as_ref()));

        assert!(
            !trouvees.is_empty(),
            "aucun fragment trouvé sous {RACINE_FEATURES}"
        );

        trouvees
    }

    /// Le manifeste décrit l'installation ; il n'a rien à faire dans le projet installé.
    #[test]
    fn the_fragment_manifest_is_not_copied_into_the_project() {
        for feature in ["docker", "ci"] {
            let files = Source::feature(None, feature)
                .expect("le fragment doit exister")
                .files()
                .expect("les templates embarquées doivent se lire");

            assert!(
                !files
                    .iter()
                    .any(|file| file.destination == Path::new(MANIFESTE)),
                "`{MANIFESTE}` serait déposé par `add {feature}` : {:?}",
                files
                    .iter()
                    .map(|file| file.destination.display().to_string())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn a_feature_source_yields_its_embedded_files() {
        let files = Source::feature(None, "docker")
            .expect("`docker` doit être une feature connue")
            .files()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = files
            .iter()
            .map(|file| file.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, DESTINATIONS_DOCKER);

        for file in &files {
            assert!(
                !file.source.is_empty(),
                "{} est embarquée vide",
                file.destination.display()
            );
        }
    }

    #[test]
    fn an_unknown_feature_is_reported_by_its_name() {
        let error = Source::feature(None, "_aucune_feature_de_ce_nom_")
            .expect_err("aucun fragment ne porte ce nom : la source ne doit pas être vide");

        assert!(
            error.to_string().contains("_aucune_feature_de_ce_nom_"),
            "le message ne nomme pas la feature : {error}"
        );
        // Énumérées une à une plutôt qu'en un bloc : la liste s'allonge à chaque fragment
        // livré, et l'ordre alphabétique intercale les nouveaux venus.
        for installable in ["auth", "ci", "docker", "mail", "redis", "storage"] {
            assert!(
                error.to_string().contains(installable),
                "le message n'énumère pas `{installable}` : {error}"
            );
        }
    }

    #[test]
    fn a_templates_directory_takes_precedence_for_a_feature() {
        let directory = tempfile::tempdir().expect("répertoire temporaire créable");
        fs::create_dir(directory.path().join("docker")).expect("sous-répertoire créable");
        fs::write(
            directory.path().join("docker/Dockerfile.jinja"),
            "FROM surcharge",
        )
        .expect("template écrivable");

        let files = Source::feature(Some(directory.path()), "docker")
            .expect("le répertoire doit fournir la feature")
            .files()
            .expect("le répertoire doit se lire");

        let destinations: Vec<String> = files
            .iter()
            .map(|file| file.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, ["Dockerfile"]);
        assert_eq!(files[0].source, "FROM surcharge");
    }

    #[test]
    fn each_feature_template_carries_the_jinja_suffix() {
        for path in feature_templates() {
            assert_eq!(
                path.extension().and_then(|suffixe| suffixe.to_str()),
                Some("jinja"),
                "{} ne porte pas le suffixe `.jinja`",
                path.display()
            );
        }
    }

    #[test]
    fn each_feature_template_renders_with_its_context() {
        let renderer = Renderer::new();

        for path in feature_templates() {
            let source = read(&path);
            renderer
                .render(&source, feature_context())
                .unwrap_or_else(|error| {
                    panic!("{} ne se rend pas : {error}", path.display());
                });
        }
    }

    // `users` est la cible la plus probable de toute relation d'un projet rbs : sans les
    // ancres, `rbs add auth` suivi d'une relation vers `users` dans un projet neuf ne
    // pourrait jamais écrire son côté inverse — la règle qui protège un modèle antérieur
    // au jalon viderait alors la fonctionnalité de sa substance sur son cas le plus
    // courant. Les deux entités du fragment sont nichées dans un module, d'où
    // l'indentation à huit espaces pour l'ancre interne à l'énumération.
    #[test]
    fn the_auth_fragment_carries_both_anchors_on_each_of_its_two_entities() {
        let source = read(&Path::new(RACINE_FEATURES).join("auth/model.rs.jinja"));

        for table in ["users", "refresh_tokens"] {
            assert!(
                source.contains(&format!(
                    "        // <rbs:relations:{table}>\n        // </rbs:relations:{table}>"
                )),
                "`{table}` doit porter l'ancre des variantes à son nom :\n{source}"
            );
            assert!(
                source.contains(&format!(
                    "    // <rbs:related:{table}>\n    // </rbs:related:{table}>"
                )),
                "`{table}` doit porter l'ancre des `impl Related` à son nom :\n{source}"
            );
        }
    }

    /// L'appel qui commence à `debut`, refermé sur sa parenthèse ouvrante.
    ///
    /// Une fenêtre d'un nombre fixe de caractères déborderait sur le code d'après, où
    /// l'adresse a toute sa place.
    fn call_at(source: &str, debut: usize) -> String {
        let mut profondeur = 0_usize;
        let mut appel = String::new();

        for caractere in source[debut..].chars() {
            appel.push(caractere);

            match caractere {
                '(' => profondeur += 1,
                ')' if profondeur <= 1 => break,
                ')' => profondeur -= 1,
                _ => {}
            }
        }

        appel
    }

    /// Un 409 qui cite l'adresse la confirme à qui l'a soumise, dans la réponse comme
    /// dans le journal : l'inscription devient l'oracle d'énumération que le hash témoin
    /// de `login` écarte de l'autre côté.
    #[test]
    fn no_conflict_of_the_auth_fragment_echoes_the_address_it_refuses() {
        for fichier in ["service.rs.jinja", "repository.rs.jinja"] {
            let source = read(&Path::new(RACINE_FEATURES).join("auth").join(fichier));

            for (debut, _) in source.match_indices("Error::Conflict") {
                let construction = call_at(&source, debut);

                assert!(
                    !construction.contains("email"),
                    "{fichier} répète l'adresse refusée dans son 409 :\n{construction}"
                );
            }
        }
    }

    /// Un jeton rejoué a fuité : révoquer la seule ligne présentée laisse celui qui a
    /// devancé la rotation légitime avec une paire valide, renouvelée indéfiniment.
    #[test]
    fn a_replayed_refresh_closes_every_session_of_the_account() {
        let repository = read(&Path::new(RACINE_FEATURES).join("auth/repository.rs.jinja"));
        let service = read(&Path::new(RACINE_FEATURES).join("auth/service.rs.jinja"));

        assert!(
            repository.contains("pub async fn revoke_sessions_of("),
            "le repository n'offre aucun moyen de fermer les sessions d'un compte :\n{repository}"
        );
        assert!(
            service.contains("repository::revoke_sessions_of("),
            "le service laisse les sessions sœurs ouvertes après un rejeu :\n{service}"
        );
    }

    /// La réutilisation détectée est un signal de sécurité, pas un incident muet — et le
    /// journal ne porte pas ce que la réponse tait.
    #[test]
    fn the_replay_is_logged_without_the_address_nor_the_token() {
        let service = read(&Path::new(RACINE_FEATURES).join("auth/service.rs.jinja"));

        let debut = service
            .find("tracing::warn!")
            .expect("un jeton rejoué doit laisser une trace");
        let alerte = call_at(&service, debut);

        for interdit in ["email", "refresh_token"] {
            assert!(
                !alerte.contains(interdit),
                "l'alerte de rejeu porte `{interdit}` :\n{alerte}"
            );
        }
    }

    #[test]
    fn the_jobs_fragment_carries_both_anchors() {
        let source = read(&Path::new(RACINE_FEATURES).join("jobs/model.rs.jinja"));

        assert!(
            source.contains(
                "pub enum Relation {\n    // <rbs:relations:jobs>\n    // </rbs:relations:jobs>\n}"
            ),
            "{source}"
        );
        assert!(
            source.contains("\n// <rbs:related:jobs>\n// </rbs:related:jobs>\n"),
            "{source}"
        );
    }

    /// Renversement assumé de la décision inverse : le compose ne publiait pas 5432
    /// parce que l'API l'atteignait par le réseau du compose. Le compose du squelette
    /// sert `cargo run` sur l'hôte, qui ne l'atteint que par un port publié.
    /// Les docs sont exposées par défaut, ce qu'un projet en cours d'écriture veut ; un
    /// déploiement, non. Sans ce profil, la décision n'a nulle part où s'écrire, et tout
    /// `docker compose --profile app up` publie `/docs` et le document.
    #[test]
    fn the_production_profile_closes_the_docs() {
        let source = read(&Path::new(RACINE).join("config/production.toml.jinja"));

        assert!(
            source.contains("[docs]"),
            "le profil de production ne dit rien des docs :\n{source}"
        );
        for reglage in ["swagger_ui = false", "openapi_json = false"] {
            assert!(
                source.contains(reglage),
                "`{reglage}` manque au profil de production :\n{source}"
            );
        }
    }

    /// Le profil existe en deux exemplaires qui doivent rester identiques : celui du
    /// squelette, et celui que le fragment `docker` dépose sur un projet créé avant lui —
    /// sans quoi le `RBS_ENV=production` du compose désignerait un fichier absent, et
    /// l'API publierait la documentation que ce profil coupe.
    #[test]
    fn the_production_profile_of_the_docker_fragment_matches_the_skeleton() {
        let squelette = read(&Path::new(RACINE).join("config/production.toml.jinja"));
        let repli = read(&Path::new(RACINE_FEATURES).join("docker/config/production.toml.jinja"));

        assert_eq!(
            squelette, repli,
            "le profil du fragment docker diverge de celui du squelette"
        );
    }

    #[test]
    fn the_project_compose_publishes_the_database_port() {
        let source = read(&Path::new(RACINE).join("docker-compose.yml.jinja"));

        assert!(
            source.contains("{@ database_port @}:5432"),
            "le compose doit publier le port du .env :\n{source}"
        );
    }

    #[test]
    fn the_project_compose_targets_the_latest_stable_postgres() {
        // Le code généré ne réclame plus la 18 depuis que le modèle pose lui-même son
        // identifiant : c'est un choix de défaut pour un projet neuf, non une exigence.
        // Le test l'épingle pour que l'image ne vieillisse pas en silence.
        let source = read(&Path::new(RACINE).join("docker-compose.yml.jinja"));

        assert!(
            source.contains("postgres:18"),
            "le compose ne vise pas PostgreSQL 18 :\n{source}"
        );
    }

    #[test]
    fn the_ci_source_yields_its_workflow() {
        let files = Source::feature(None, "ci")
            .expect("`ci` doit être une feature connue")
            .files()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = files
            .iter()
            .map(|file| file.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, [".github/workflows/ci.yml"]);
    }

    #[test]
    fn the_ci_workflow_brings_a_migrated_database_before_the_tests() {
        // Les tests d'une feature générée montent l'application sur une vraie base et
        // supposent les migrations appliquées : sans elles, la CI échoue sur un schéma
        // absent, loin de sa cause.
        let source = read(&Path::new(RACINE_FEATURES).join("ci/.github/workflows/ci.yml.jinja"));

        assert!(
            source.contains("postgres:18"),
            "le workflow n'amène pas PostgreSQL 18 :\n{source}"
        );

        let migrations = source
            .find("-p migration")
            .expect("le workflow doit appliquer les migrations");
        let tests = source
            .find("cargo test")
            .expect("le workflow doit lancer les tests");

        assert!(
            migrations < tests,
            "les migrations doivent précéder les tests :\n{source}"
        );
    }

    #[test]
    fn the_docker_builder_installs_what_the_build_needs() {
        // `utoipa-swagger-ui` télécharge son archive pendant la compilation, avec `curl`,
        // que l'image `rust:slim` ne porte pas : sans lui le build casse à la toute fin,
        // après plusieurs minutes de compilation.
        let source = read(&Path::new(RACINE_FEATURES).join("docker/Dockerfile.jinja"));
        let builder = source
            .split("AS runtime")
            .next()
            .expect("le Dockerfile doit avoir une étape de build");

        assert!(
            builder.contains("curl"),
            "l'étape de build n'installe pas curl :\n{builder}"
        );
    }

    /// Le dépilage porte ses trois moteurs, et lui seul.
    ///
    /// `SKIP LOCKED` est du PostgreSQL et du MySQL 8 ; SQLite ne le connaît pas et n'en a
    /// pas besoin, ne laissant écrire qu'un processus à la fois. Le tri se fait à
    /// l'exécution : le fragment livre les trois branches, le projet n'en emprunte qu'une.
    #[test]
    fn the_dequeue_carries_its_three_engines_and_nothing_else_does() {
        let racine = Path::new(RACINE_FEATURES).join("jobs");
        let queue = read(&racine.join("queue.rs.jinja"));

        for moteur in [
            "DatabaseBackend::Postgres",
            "DatabaseBackend::MySql",
            "DatabaseBackend::Sqlite",
        ] {
            assert!(
                queue.contains(moteur),
                "le dépilage ne traite pas {moteur} :\n{queue}"
            );
        }

        // Deux moteurs la portent, et la clause n'a pas à paraître ailleurs — surtout pas
        // dans la branche SQLite, qui la refuserait à l'exécution.
        assert_eq!(
            queue.matches("FOR UPDATE SKIP LOCKED").count(),
            3,
            "la clause doit paraître dans les deux requêtes qui l'admettent et dans son \
             commentaire, et nulle part ailleurs :\n{queue}"
        );
    }

    /// Le dépilage de la file n'apparaît qu'à un seul endroit du fragment.
    ///
    /// C'est ce qui rend le portage vers un autre moteur tenable : il n'y a qu'un corps de
    /// fonction à récrire tant qu'il ne s'est pas dispersé dans le worker.
    #[test]
    fn the_dequeue_appears_in_a_single_place_of_the_jobs_fragment() {
        let racine = Path::new(RACINE_FEATURES).join("jobs");
        let mut fichiers = Vec::new();
        walk(&racine, &mut fichiers);

        let porteurs: Vec<String> = fichiers
            .iter()
            .filter(|path| read(path).contains("FOR UPDATE SKIP LOCKED"))
            .map(|path| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        assert_eq!(porteurs, ["queue.rs.jinja"], "porteurs : {porteurs:?}");

        let queue = read(&racine.join("queue.rs.jinja"));
        assert_eq!(
            queue.matches("pub async fn reserver_prochain_job").count(),
            1,
            "le dépilage doit tenir dans une fonction unique :\n{queue}"
        );
    }

    /// Une balise Jinja de contrôle (`{%- if … %}`, `{%- endif %}`) s'écrit au ras de la
    /// marge par convention du dépôt, quelle que soit la profondeur YAML environnante :
    /// elle ne porte donc aucune indentation à retirer, dans aucun des deux fichiers.
    fn est_balise_jinja(line: &str) -> bool {
        line.trim_start().starts_with("{%")
    }

    /// Retire l'indentation commune à toutes les lignes non vides d'un bloc, pour comparer
    /// le contenu d'une ancre au niveau racine d'un manifeste à son équivalent imbriqué
    /// sous `services:` dans un fichier YAML — modulo l'indentation, sans quoi les deux
    /// diffèreraient sur chaque ligne plutôt que sur celle qui a vraiment divergé. Les
    /// balises Jinja sont exclues du calcul et laissées à la marge : les compter ferait
    /// tomber l'indentation mesurée à zéro, celle qu'elles portent réellement.
    fn dedent(text: &str) -> String {
        let indentation = text
            .lines()
            .filter(|line| !line.trim().is_empty() && !est_balise_jinja(line))
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        text.lines()
            .map(|line| {
                if line.trim().is_empty() || est_balise_jinja(line) {
                    line.trim_start()
                } else {
                    &line[indentation.min(line.len())..]
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    }

    /// Le bloc `migrate`/`api` existe en deux exemplaires qui doivent rester identiques au
    /// caractère près : le `content` de l'ancre `services` du manifeste du fragment
    /// `docker`, inséré dans un compose qui existe déjà, et le corps de la même ancre dans
    /// le compose de repli qu'`if_absent` écrit pour un projet qui n'en a pas encore.
    ///
    /// Une seule ligne désynchronisée (`restart: "no"` devenu `restart: always` d'un seul
    /// côté, mesuré en relecture) fait que la ligne survivante s'attache au mauvais
    /// service dans le fichier fraîchement écrit, sans qu'aucune commande n'échoue et sans
    /// qu'aucun autre test ne le remarque : c'est ce test qui doit mordre à sa place.
    #[test]
    fn the_services_anchor_content_matches_the_fallback_compose_body() {
        let manifeste = read(&Path::new(RACINE_FEATURES).join("docker/feature.toml"));
        let manifest = crate::manifest::read(&manifeste, "docker/feature.toml")
            .expect("le manifeste du fragment docker doit se lire");

        let content = &manifest
            .anchors
            .iter()
            .find(|insertion| insertion.anchor == "services")
            .expect("le fragment docker déclare l'ancre services")
            .content;

        let compose = read(&Path::new(RACINE_FEATURES).join("docker/docker-compose.yml.jinja"));
        let corps = crate::anchors::body(&compose, crate::anchors::SERVICES)
            .expect("le compose de repli doit porter l'ancre services");

        assert_eq!(
            dedent(content),
            dedent(corps),
            "le contenu de l'ancre du manifeste diverge du corps du compose de repli"
        );
    }

    /// Début et fin du service `db`, identiques dans le corps de la fonction de test elle
    /// aussi : c'est la même chaîne dont la présence est vérifiée dans les deux fichiers.
    const DEBUT_SERVICE_DB: &str = "\n  db:\n";
    const FIN_SERVICE_DB: &str = "      retries: 30\n";

    /// Isole le service `db`, de sa déclaration à la fin de son healthcheck.
    fn service_db(source: &str) -> &str {
        let debut = source
            .find(DEBUT_SERVICE_DB)
            .expect("le service db doit être présent")
            + 1;
        let fin_relative = source[debut..]
            .find(FIN_SERVICE_DB)
            .expect("le healthcheck du service db doit être présent");

        &source[debut..debut + fin_relative + FIN_SERVICE_DB.len()]
    }

    /// Le service `db` est dupliqué entre le compose du squelette — écrit quand un projet
    /// a de quoi le rendre utile — et le compose de repli du fragment `docker` — écrit pour
    /// un projet qui n'a pas encore de compose. Une divergence entre les deux serait plus
    /// douce que celle de `migrate`/`api` (les deux composent des fichiers valides), mais
    /// romprait la même promesse : que les deux chemins d'écriture rendent le même service.
    #[test]
    fn the_db_service_matches_between_the_skeleton_and_the_fallback_compose() {
        let squelette = read(&Path::new(RACINE).join("docker-compose.yml.jinja"));
        let repli = read(&Path::new(RACINE_FEATURES).join("docker/docker-compose.yml.jinja"));

        assert_eq!(
            service_db(&squelette),
            service_db(&repli),
            "le service db diverge entre le compose du squelette et celui du fragment docker"
        );
    }
}
