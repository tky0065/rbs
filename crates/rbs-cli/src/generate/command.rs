//! `rbs generate crud` et `rbs generate feature` : la feature écrite dans un projet.
//!
//! La séquence est celle du §4.4 de la spec, dans l'ordre où les échecs restent
//! inoffensifs : le nom, les champs et les ancres sont vérifiés, le rendu aboutit
//! entièrement, et le premier fichier n'est écrit qu'ensuite. Un nom refusé, une ancre
//! disparue ou une feature déjà présente laissent le disque tel qu'ils l'ont trouvé.

use std::io;
use std::path::{Path, PathBuf};

use crate::git;
use crate::metadata;
use crate::plan;

use super::feature::Feature;
use super::{
    controller, dto, entity, fields, format, migration, mount, name, repository, seed, service,
    tests_http,
};

/// Ce qu'il faut savoir pour générer une feature.
pub(crate) struct Options {
    /// Nom de la feature, au pluriel.
    pub name: String,
    /// Champs de l'entité, tels que `--fields` les donne.
    pub fields: Option<String>,
    /// `crud` génère l'entité, la migration et les tests ; `feature` s'arrête au squelette.
    pub complete: bool,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Génère même si le projet porte des modifications non commitées.
    pub force: bool,
}

/// Un fichier à écrire : son chemin, relatif à la racine du projet, et son contenu.
type File = (String, String);

/// Ce qu'une génération fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Chemins des fichiers de la feature, relatifs à la racine du projet.
    pub files: Vec<String>,
    /// Module de la migration générée, s'il y en a une.
    pub migration: Option<String>,
    /// Ce que rustfmt n'a pas pu faire sur le rendu, s'il y a lieu.
    pub avertissement: Option<format::Avertissement>,
}

/// Ce qui peut empêcher de générer une feature.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le nom de la feature est inutilisable.
    #[error("{0}")]
    Nom(name::NameError),

    /// Les champs ne s'analysent pas.
    #[error("{0}")]
    Champs(fields::FieldsError),

    /// La feature occupe déjà son répertoire.
    #[error("{path} existe déjà : la feature `{feature}` est déjà là")]
    DejaPresente {
        /// Chemin occupé, relatif à la racine.
        path: String,
        /// Feature demandée.
        feature: String,
    },

    /// Une template ne s'est pas rendue.
    #[error("{file} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        file: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// Un fichier du projet n'a pu être lu ou écrit.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// Le projet porte des modifications non commitées, qu'une génération rendrait
    /// indiscernables des siennes.
    #[error("le working tree n'est pas propre : {files} — commitez, ou relancez avec --force")]
    WorkingTreeSale {
        /// Fichiers suivis modifiés, énumérés.
        files: String,
    },

    /// Le plan de la génération n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

impl Error {
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Seule une ancre disparue a un remède tenant en un bloc de texte : les autres pannes
    /// se règlent par une décision — commiter, choisir un autre nom, corriger un champ.
    pub(crate) fn remedy(&self) -> Option<String> {
        match self {
            Error::Plan(plan::Error::Anchor(absente)) => Some(format!(
                "dans {} :\n{}",
                absente.anchor.file,
                absente.anchor.block()
            )),
            _ => None,
        }
    }
}

/// Calcule ce que la génération de `options` ferait au projet, sans rien écrire.
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    let start = options
        .directory
        .canonicalize()
        .map_err(|source| access(&options.directory, source))?;
    let root = metadata::project_root(&start).ok_or(Error::PasUnProjet)?;

    if !options.force {
        let modifies = git::modified_files(&root);
        if !modifies.is_empty() {
            return Err(Error::WorkingTreeSale {
                files: git::enumerate(&modifies),
            });
        }
    }

    name::validate(&options.name).map_err(Error::Nom)?;
    let fields =
        fields::parse(options.fields.as_deref().unwrap_or_default()).map_err(Error::Champs)?;

    let feature = Feature::fresh(&options.name, fields);
    let module = feature.module().to_string();

    if root.join("src").join(&module).exists() {
        return Err(Error::DejaPresente {
            path: format!("src/{module}"),
            feature: module,
        });
    }

    let (mut files, migration) = render(&feature, options.complete)?;

    // Après le rendu et avant le plan : le plan porte le contenu exact qui sera écrit,
    // et c'est lui que `--dry-run` montre.
    let avertissement = format::format_batch(files.iter_mut().map(|(_, content)| content));

    let mut builder = plan::Builder::new(root);
    for (path, content) in &files {
        builder.create(path, content)?;
    }

    let mut montages = mount::pour(&module);
    if let Some(migration) = &migration {
        montages.extend(mount::for_migration(migration));
    }
    if options.complete {
        montages.extend(mount::for_seed(&module));
    }
    for mount in montages {
        builder.insert(mount.anchor, &mount.lines)?;
    }

    builder.patch(plan::PatchToml::InscrireFeature(module))?;

    Ok(Planned {
        plan: builder.finir(),
        files: files.into_iter().map(|(path, _)| path).collect(),
        migration,
        avertissement,
    })
}

/// Rend les fichiers de la feature, et sa migration si elle est complète.
///
/// Rien n'est écrit ici : une template fautive doit échouer avant la première écriture.
fn render(feature: &Feature, complete: bool) -> Result<(Vec<File>, Option<String>), Error> {
    let module = feature.module();
    let dans = |name: &str| format!("src/{module}/{name}");

    let mut files = vec![
        (dans("mod.rs"), controller::render_mod(feature, complete)),
        (dans("model.rs"), entity::render(feature)),
        (dans("dto.rs"), dto::render(feature)),
        (dans("repository.rs"), repository::render(feature)),
        (dans("service.rs"), service::render(feature)),
        (dans("controller.rs"), controller::render(feature)),
    ];

    if complete {
        files.push((dans("tests.rs"), tests_http::render(feature)));
        // Hors du répertoire de la feature : le seed appartient au binaire qui l'applique,
        // et non au module que le routeur monte.
        files.push((format!("src/seeds/{module}.rs"), seed::render(feature)));
    }

    let mut rendus = Vec::with_capacity(files.len() + 1);
    for (path, rendered) in files {
        let content = rendered.map_err(|source| Error::Rendu {
            file: path.clone(),
            source,
        })?;
        rendus.push((path, content));
    }

    if !complete {
        return Ok((rendus, None));
    }

    let rendue = migration::render(feature, &migration::current_timestamp()).map_err(|source| {
        Error::Rendu {
            file: format!("migration de {module}"),
            source,
        }
    })?;

    rendus.push((
        format!("migration/src/{}.rs", rendue.module),
        rendue.content,
    ));

    Ok((rendus, Some(rendue.module)))
}

fn access(path: &Path, source: io::Error) -> Error {
    Error::Acces {
        path: path.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use assert_cmd::Command;
    use tempfile::TempDir;

    use super::*;
    use crate::generate::bench;

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    fn fingerprint(root: &Path) -> std::collections::BTreeMap<PathBuf, String> {
        let mut vue = std::collections::BTreeMap::new();
        let mut a_visiter = vec![root.to_path_buf()];

        while let Some(directory) = a_visiter.pop() {
            for input in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = input.expect("l'entrée se lit").path();
                let relatif = path
                    .strip_prefix(root)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();

                if path.is_dir() {
                    vue.insert(relatif, String::new());
                    a_visiter.push(path);
                } else {
                    vue.insert(relatif, fs::read_to_string(&path).unwrap_or_default());
                }
            }
        }

        vue
    }

    /// Planifie puis applique, comme la commande le fait.
    ///
    /// Les tests portent sur ce que la génération laisse sur le disque : les deux temps
    /// n'ont d'intérêt qu'à l'affichage, qui appartient à l'appelant.
    fn run(options: &Options) -> Result<Planned, Error> {
        let planned = plan_for(options)?;
        crate::plan::application::apply(&planned.plan, options.force)?;

        Ok(planned)
    }

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
    fn project() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    fn options(root: &Path, name: &str, fields: Option<&str>, complete: bool) -> Options {
        Options {
            name: name.to_string(),
            fields: fields.map(str::to_string),
            complete,
            directory: root.to_path_buf(),
            force: false,
        }
    }

    /// Fait du projet un dépôt dont tout est commité.
    fn commit(root: &Path) {
        for arguments in [
            vec!["config", "user.email", "rbs@example.test"],
            vec!["config", "user.name", "rbs"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "projet neuf"],
        ] {
            let output = std::process::Command::new("git")
                .args(&arguments)
                .current_dir(root)
                .output()
                .expect("git doit être lançable");

            assert!(
                output.status.success(),
                "git {arguments:?} a échoué :\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Modifie un fichier suivi du projet, sans abîmer ce dont la génération a besoin.
    fn dirty(root: &Path) {
        let main = root.join("src/main.rs");
        let source = read(&main);

        fs::write(&main, format!("{source}\n// une modification en cours\n"))
            .expect("main.rs réécrivable");
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} illisible : {error}", path.display()))
    }

    #[test]
    fn a_crud_writes_the_seven_files_of_the_feature_and_its_migration() {
        let (_parent, root) = project();

        let generated = run(&options(&root, "articles", Some("title:string"), true))
            .expect("la génération doit aboutir");

        for file in [
            "mod.rs",
            "model.rs",
            "dto.rs",
            "repository.rs",
            "service.rs",
            "controller.rs",
            "tests.rs",
        ] {
            assert!(
                root.join("src/articles").join(file).exists(),
                "src/articles/{file} manquant"
            );
        }

        let module = generated.migration.expect("un crud porte une migration");
        assert!(
            root.join("migration/src")
                .join(format!("{module}.rs"))
                .exists(),
            "migration {module} manquante"
        );
    }

    /// Les premières lignes qui séparent le rendu de ce que rustfmt en ferait.
    ///
    /// Un `assert_eq!` sur deux fichiers entiers noie la divergence dans deux pavés
    /// échappés ; ce qui se lit, c'est la ligne fautive et son numéro.
    fn divergence(rendered: &str, formatted: &str) -> String {
        rendered
            .lines()
            .zip(formatted.lines())
            .enumerate()
            .filter(|(_, (before, after))| before != after)
            .take(3)
            .map(|(rang, (before, after))| {
                format!(
                    "  ligne {} :\n    rendu   : {before}\n    rustfmt : {after}",
                    rang + 1
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// La longueur d'un nom de feature est un continuum, et rustfmt bascule à 100
    /// colonnes : une forme écrite en dur dans un template n'est juste que pour les noms
    /// qui la font tomber du bon côté. L'éventail balaye les deux bascules — `tag` tient
    /// sur une ligne là où `articles` déborde, `administrative_documents` déborde là où
    /// `articles` tient — et la feature sans champ, dont le rendu perd des blocs entiers.
    #[test]
    fn the_render_goes_through_rustfmt_without_a_diff_whatever_the_name_length() {
        let cas: &[(&str, Option<&str>, bool)] = &[
            ("tag", Some("title:string,views:int"), true),
            ("post", Some("title:string,views:int"), true),
            ("billet", Some("title:string,views:int"), true),
            (
                "articles",
                Some("title:string,summary:text:optional,views:int"),
                true,
            ),
            (
                "administrative_documents",
                Some("title:string,views:int"),
                true,
            ),
            ("notes", None, false),
        ];

        for (name, fields, complete) in cas {
            let (_parent, root) = project();

            let generated =
                run(&options(&root, name, *fields, *complete)).expect("la génération doit aboutir");

            let mut ecrits: Vec<PathBuf> = generated
                .files
                .iter()
                .map(|relatif| root.join(relatif))
                .collect();
            if let Some(module) = &generated.migration {
                ecrits.push(root.join("migration/src").join(format!("{module}.rs")));
            }

            assert!(!ecrits.is_empty(), "{name} n'a rien écrit");

            for path in ecrits {
                let ecrit = read(&path);
                let formatted = bench::formatted(&ecrit);
                let file = path
                    .file_name()
                    .expect("le chemin nomme un fichier")
                    .to_string_lossy();

                assert!(
                    formatted == ecrit,
                    "un `cargo fmt` chez l'utilisateur reformaterait {name}/{file}, \
                     qu'il n'a pas touché :\n{}",
                    divergence(&ecrit, &formatted)
                );
            }
        }
    }

    #[test]
    fn a_crud_mounts_the_feature_into_the_five_anchors() {
        let (_parent, root) = project();

        run(&options(&root, "articles", Some("title:string"), true))
            .expect("la génération doit aboutir");

        assert!(read(&root.join("src/main.rs")).contains("mod articles;"));
        assert!(read(&root.join("src/router.rs")).contains(".merge(crate::articles::routes())"));
        assert!(read(&root.join("src/openapi.rs")).contains("crate::articles::controller::list,"));

        let lib = read(&root.join("migration/src/lib.rs"));
        assert!(lib.contains("_create_articles;"), "{lib}");
        assert!(lib.contains("::Migration),"), "{lib}");
    }

    /// Le premier critère du lot : le seed est écrit, et l'ancre porte son appel.
    #[test]
    fn a_crud_drops_its_seed_and_declares_it_in_the_anchor() {
        let (_parent, root) = project();

        run(&options(&root, "articles", Some("title:string"), true))
            .expect("la génération doit aboutir");

        assert!(
            root.join("src/seeds/articles.rs").exists(),
            "le seed de la feature manque"
        );

        let binaire = read(&root.join("src/seeds/main.rs"));
        let ancre = crate::anchors::body(&binaire, crate::anchors::SEEDS)
            .expect("l'ancre des seeds est présente");
        assert_eq!(ancre.trim(), "articles,", "{binaire}");
    }

    /// Le troisième critère : deux générations, deux seeds, une ancre en ordre.
    #[test]
    fn two_generations_leave_two_seeds_and_an_orderly_anchor() {
        let (_parent, root) = project();

        for feature in ["articles", "notes"] {
            run(&options(&root, feature, Some("title:string"), true))
                .expect("la génération doit aboutir");
        }

        assert!(root.join("src/seeds/articles.rs").exists());
        assert!(root.join("src/seeds/notes.rs").exists());

        let binaire = read(&root.join("src/seeds/main.rs"));
        let ancre = crate::anchors::body(&binaire, crate::anchors::SEEDS)
            .expect("l'ancre des seeds est présente");
        let declarations: Vec<&str> = ancre
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        assert_eq!(declarations, ["articles,", "notes,"], "{binaire}");
    }

    /// Une feature écrite à la main n'a pas d'entité : rien à semer.
    #[test]
    fn an_empty_feature_writes_neither_migration_nor_tests() {
        let (_parent, root) = project();

        let generated =
            run(&options(&root, "notes", None, false)).expect("la génération doit aboutir");

        assert!(root.join("src/notes/controller.rs").exists());
        assert!(
            !root.join("src/notes/tests.rs").exists(),
            "une feature écrite à la main ne porte pas de tests générés"
        );
        assert_eq!(generated.migration, None);
        assert!(
            !read(&root.join("migration/src/lib.rs")).contains("notes"),
            "la crate migration ne doit pas être touchée"
        );
        assert!(
            !root.join("src/seeds/notes.rs").exists(),
            "une feature sans entité n'a rien à semer"
        );
        assert!(
            !read(&root.join("src/seeds/main.rs")).contains("notes"),
            "le binaire des seeds ne doit pas être touché"
        );
        assert!(
            !read(&root.join("src/notes/mod.rs")).contains("mod tests;"),
            "le module de tests ne doit pas être déclaré"
        );
    }

    /// Le troisième critère de D7b, que la commande rend enfin vérifiable.
    #[test]
    fn a_name_clashing_with_the_skeleton_is_rejected_naming_the_conflict() {
        let (_parent, root) = project();
        let before = read(&root.join("src/main.rs"));

        let error = run(&options(&root, "state", Some("title:string"), true))
            .expect_err("`state` entre en conflit avec le squelette");

        let message = error.to_string();
        assert!(message.contains("state"), "{message}");
        assert!(
            message.contains("module"),
            "le message doit nommer le conflit : {message}"
        );
        assert!(!root.join("src/state").is_dir(), "un répertoire a été créé");
        assert_eq!(read(&root.join("src/main.rs")), before, "main.rs a bougé");
    }

    /// Le troisième critère de D7b, second cas.
    #[test]
    fn a_rust_keyword_is_rejected_naming_the_conflict() {
        let (_parent, root) = project();

        let error = run(&options(&root, "match", Some("title:string"), true))
            .expect_err("`match` est un mot-clé");

        let message = error.to_string();
        assert!(message.contains("match"), "{message}");
        assert!(!root.join("src/match").is_dir(), "un répertoire a été créé");
    }

    #[test]
    fn faulty_fields_are_rejected_before_any_write() {
        let (_parent, root) = project();

        let error = run(&options(&root, "articles", Some("title:chaine"), true))
            .expect_err("`chaine` n'est pas un type");

        assert!(error.to_string().contains("chaine"), "{error}");
        assert!(
            !root.join("src/articles").is_dir(),
            "un répertoire a été créé"
        );
    }

    #[test]
    fn an_already_present_feature_is_rejected() {
        let (_parent, root) = project();
        run(&options(&root, "articles", Some("title:string"), true))
            .expect("la première génération doit aboutir");

        let error = run(&options(&root, "articles", Some("title:string"), true))
            .expect_err("la feature est déjà là");

        assert!(error.to_string().contains("articles"), "{error}");
    }

    #[test]
    fn the_feature_is_recorded_in_the_project_metadata() {
        let (_parent, root) = project();

        run(&options(&root, "articles", Some("title:string"), true))
            .expect("la génération doit aboutir");

        let metadonnees = metadata::read(&root.join("Cargo.toml")).expect("métadonnées lisibles");
        assert!(
            metadonnees.features.contains(&"articles".to_string()),
            "{metadonnees:?}"
        );
    }

    #[test]
    fn a_vanished_anchor_gives_the_block_to_paste() {
        let error = Error::Plan(crate::plan::Error::Anchor(crate::anchors::Missing {
            anchor: crate::anchors::ROUTES,
        }));

        let remedy = error.remedy().expect("une ancre disparue se recolle");

        assert!(remedy.contains("src/router.rs"), "{remedy}");
        assert!(remedy.contains("// <rbs:routes>"), "{remedy}");
        assert!(remedy.contains("// </rbs:routes>"), "{remedy}");
    }

    #[test]
    fn an_error_without_a_known_remedy_does_not_invent_one() {
        assert_eq!(Error::PasUnProjet.remedy(), None);
    }

    /// Le plan montré est celui qui sera exécuté : c'est ce qui rend `--dry-run` digne de
    /// foi. Un affichage qui décrirait une intention plutôt qu'un résultat mentirait dès
    /// que le projet s'écarterait de ce que la commande suppose.
    ///
    /// Le plan est planifié une seule fois, et non deux : le nom d'une migration porte
    /// l'heure à la seconde, et deux planifications successives peuvent légitimement
    /// différer. C'est le même plan qui doit être affiché puis appliqué, non deux plans
    /// que l'on comparerait.
    #[test]
    fn the_displayed_plan_is_the_one_the_run_carries_out() {
        let (_parent, root) = project();
        let options = options(&root, "articles", Some("title:string"), true);

        let before = fingerprint(&root);
        let planned = plan_for(&options).expect("la planification aboutit");
        let announcement = crate::plan::render::plan(&planned.plan);

        assert_eq!(
            fingerprint(&root),
            before,
            "planifier et afficher ne doivent rien écrire"
        );

        crate::plan::application::apply(&planned.plan, false).expect("l'écriture aboutit");

        for file in planned.plan.files() {
            assert_eq!(
                read(&root.join(&file.path)),
                file.after,
                "`{}` ne porte pas le contenu que le plan annonçait",
                file.path
            );
        }
        assert_eq!(
            crate::plan::render::plan(&planned.plan),
            announcement,
            "le plan a changé entre son affichage et son application"
        );
    }

    #[test]
    fn outside_an_rbs_project_nothing_is_written() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let error = run(&options(ailleurs.path(), "articles", None, true))
            .expect_err("il n'y a pas de projet ici");

        assert!(matches!(error, Error::PasUnProjet), "{error}");
        assert!(!ailleurs.path().join("src").exists());
    }

    #[test]
    fn the_command_runs_from_a_subdirectory_of_the_project() {
        let (_parent, root) = project();

        run(&options(
            &root.join("src"),
            "articles",
            Some("title:string"),
            true,
        ))
        .expect("la racine se retrouve en remontant");

        assert!(root.join("src/articles/mod.rs").exists());
    }

    /// L'ancre disparue arrête la commande : le CLI ne réécrit jamais ce qu'il ne
    /// reconnaît pas, et n'écrit rien du reste non plus.
    #[test]
    fn a_missing_anchor_stops_the_command_without_writing_anything() {
        let (_parent, root) = project();
        let router = root.join("src/router.rs");
        let ampute = read(&router).replace("// <rbs:routes>", "");
        fs::write(&router, ampute).expect("routeur écrivable");

        let error = run(&options(&root, "articles", Some("title:string"), true))
            .expect_err("l'ancre des routes a disparu");

        assert!(
            matches!(error, Error::Plan(crate::plan::Error::Anchor(_))),
            "{error}"
        );
        assert!(
            !root.join("src/articles").is_dir(),
            "des fichiers ont été écrits malgré l'ancre absente"
        );
        assert!(!read(&root.join("src/main.rs")).contains("mod articles;"));
    }

    /// Ce que rbs écrit doit rester défaisable par un `git checkout` : il ne peut donc pas
    /// mêler ses fichiers à des modifications que le développeur n'a pas commitées.
    #[test]
    fn a_dirty_project_refuses_generation_and_writes_nothing() {
        let (_parent, root) = project();
        commit(&root);
        dirty(&root);

        let error = run(&options(&root, "notes", None, false))
            .expect_err("le working tree n'est pas propre");

        assert!(matches!(error, Error::WorkingTreeSale { .. }), "{error}");
        assert!(
            error.to_string().contains("src/main.rs"),
            "le fichier en cause doit être nommé : {error}"
        );
        assert!(
            !root.join("src/notes").is_dir(),
            "des fichiers ont été écrits malgré le working tree sale"
        );
    }

    #[test]
    fn a_dirty_project_accepts_generation_with_force() {
        let (_parent, root) = project();
        commit(&root);
        dirty(&root);

        run(&Options {
            force: true,
            ..options(&root, "notes", None, false)
        })
        .expect("`--force` passe outre");

        assert!(root.join("src/notes/controller.rs").exists());
    }

    #[test]
    fn a_project_outside_a_git_repository_generates_without_force() {
        let (_parent, root) = project();
        fs::remove_dir_all(root.join(".git")).expect("dépôt supprimable");

        run(&options(&root, "notes", None, false)).expect("hors dépôt, il n'y a rien à protéger");

        assert!(root.join("src/notes/controller.rs").exists());
    }

    /// Le critère du lot : le projet compile après génération d'une feature vide.
    ///
    /// Un CRUD est généré avec elle : c'est la même commande, et rien ne prouverait
    /// autrement que ce qu'elle écrit dans un vrai projet forme du Rust valide. Le CRUD
    /// exercé contre une base est l'affaire du test d'intégration du lot.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn the_project_compiles_after_generating_an_empty_feature() {
        let project = bench::Project::fresh();

        for arguments in [
            vec!["generate", "feature", "notes"],
            vec![
                "generate",
                "crud",
                "carnets",
                "--fields",
                "title:string,email:string:unique,views:int,published:bool,published_at:datetime",
            ],
        ] {
            Command::cargo_bin("rbs")
                .expect("le binaire rbs doit être compilé")
                .current_dir(project.root())
                .args(&arguments)
                .assert()
                .success();
        }

        project.compile();
    }
}
