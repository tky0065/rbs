//! `rbs generate crud` et `rbs generate feature` : la feature écrite dans un projet.
//!
//! La séquence est celle du §4.4 de la spec, dans l'ordre où les échecs restent
//! inoffensifs : le nom, les champs et les ancres sont vérifiés, le rendu aboutit
//! entièrement, et le premier fichier n'est écrit qu'ensuite. Un nom refusé, une ancre
//! disparue ou une feature déjà présente laissent le disque tel qu'ils l'ont trouvé.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::anchors;
use crate::git;
use crate::metadata;
use crate::plan;

use super::feature::Feature;
use super::fields::to_pascal_case;
use super::{
    controller, dto, entities, entity, fields, format, migration, mount, name, relations,
    repository, seed, service, tests_http,
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
    /// Entités enfant dont ce modèle doit recevoir la variante inverse, sans rien générer
    /// d'autre : la réparation d'une relation posée avant que ce côté n'existe.
    pub has_many: Vec<String>,
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
    /// Nom de la relation qui a écarté le seed, si l'entité en porte une requise.
    pub seed_skipped: Option<String>,
    /// La zone de l'`AGENTS.md` que le projet ne porte pas, s'il en manque une.
    pub zone_manquante: Option<crate::agents::MissingZone>,
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
    Fields(fields::FieldsError),

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

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Une ou plusieurs références n'ont pu être résolues : cible introuvable, ou deux
    /// relations réclamant la même variante.
    #[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Relations(Vec<relations::ResolveError>),

    /// Une ou plusieurs cibles résolues n'ont pas de migration dans le projet.
    #[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    MigrationsAbsentes(Vec<relations::TargetWithoutMigration>),

    /// Le modèle cible porte déjà, sous ce nom, une variante visant une autre entité.
    #[error(
        "{file} porte déjà une variante `{variant}` visant une autre cible : retirez-la, ou \
         renommez la relation, avant de régénérer"
    )]
    Homonyme {
        /// Fichier fautif, relatif à la racine.
        file: String,
        /// Nom de la variante en conflit.
        variant: String,
    },

    /// `--has-many` répare une feature déjà générée : elle doit d'abord exister.
    #[error(
        "{path} n'existe pas : `--has-many` répare une feature déjà générée, qui doit d'abord exister"
    )]
    Absente {
        /// Chemin attendu, relatif à la racine.
        path: String,
        /// Feature demandée.
        feature: String,
    },

    /// `--has-many` nomme une entité enfant qui ne porte pas la clé attendue.
    #[error(
        "{child} ne porte aucune colonne référençant `{table}` : ajoutez-la avant de relancer `--has-many {child}`"
    )]
    EnfantSansCle {
        /// Entité enfant nommée par le flag.
        child: String,
        /// Table de la feature réparée.
        table: String,
    },
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

    // Une seule lecture pour toute la fonction : son erreur se propage par `?` plutôt que
    // d'être ré-tentée, et `agents::refresh` reçoit ces métadonnées au lieu de les relire
    // elle-même.
    let metadonnees = metadata::read(&root.join("Cargo.toml"))?;

    if !options.force {
        let modifies = git::modified_files(&root);
        if !modifies.is_empty() {
            return Err(Error::WorkingTreeSale {
                files: git::enumerate(&modifies),
            });
        }
    }

    name::validate(&options.name).map_err(Error::Nom)?;

    // `--has-many` ne génère rien : il répare le côté inverse d'une feature déjà
    // présente, et suit donc un chemin entièrement séparé du reste de la fonction.
    if !options.has_many.is_empty() {
        return plan_repair(options, &root, &metadonnees);
    }

    let mut fields =
        fields::parse(options.fields.as_deref().unwrap_or_default()).map_err(Error::Fields)?;

    // Lu une fois, avant que le nom ne soit tranché : la même inventaire sert à
    // résoudre les cibles des références et à écrire leur côté inverse.
    let entities = entities::scan(&root);
    relations::resolve(&mut fields, &entities, &options.name).map_err(Error::Relations)?;
    relations::ensure_migrations_exist(&fields, &root).map_err(Error::MigrationsAbsentes)?;

    let feature = Feature::fresh(&options.name, fields);
    let module = feature.module().to_string();

    if root.join("src").join(&module).exists() {
        return Err(Error::DejaPresente {
            path: format!("src/{module}"),
            feature: module,
        });
    }

    // Le seed rejoint l'entité par la bibliothèque du projet, nommée d'après son paquet :
    // Cargo remplace les tirets par des soulignés pour en faire un identifiant Rust. Un
    // projet antérieur à ce jalon n'en a pas, et le seed y reprend la forme `#[path]`.
    let crate_name = root
        .join("src/lib.rs")
        .exists()
        .then(|| {
            metadata::package_name(&root.join("Cargo.toml")).map(|name| name.replace('-', "_"))
        })
        .transpose()?;

    // Une référence requise rend l'entité non semable : un seed engendré échouerait à
    // chaque lancement sur la contrainte de clé étrangère, faute de ligne cible connue.
    let seedable = seed::is_seedable(&feature);
    let seed_skipped = (options.complete && !seedable).then(|| unseedable_reference(&feature));

    let (mut files, migration) =
        render(&feature, options.complete, seedable, crate_name.as_deref())?;

    // Après le rendu et avant le plan : le plan porte le contenu exact qui sera écrit,
    // et c'est lui que `--dry-run` montre.
    let avertissement = format::format_batch(files.iter_mut().map(|(_, content)| content));

    // Calculé avant le builder, qui prend `root` par valeur : le contenu actuel du
    // fichier cible sert à détecter une variante homonyme visant une autre entité.
    let inverses = relations::inverses(&feature.fields, &feature, &entities);
    for inverse in &inverses {
        let existing = fs::read_to_string(root.join(&inverse.file)).unwrap_or_default();
        if relations::homonymous_conflict(&existing, inverse) {
            return Err(Error::Homonyme {
                file: inverse.file.clone(),
                variant: inverse.variant.last().cloned().unwrap_or_default(),
            });
        }
    }

    // Résolue avant le builder, qui prend `root` par valeur : sur un projet antérieur à
    // ce jalon, dépourvu de bibliothèque, l'ancre reste dans `src/main.rs`.
    let features_anchor = anchors::resolve_features(&root);

    let mut builder = plan::Builder::new(root.clone());
    for (path, content) in &files {
        builder.create(path, content)?;
    }

    let mut montages = mount::pour(&module, features_anchor);
    if let Some(migration) = &migration {
        montages.extend(mount::for_migration(migration));
    }
    if options.complete && seedable {
        montages.extend(mount::for_seed(&module));
    }
    for inverse in &inverses {
        montages.extend(mount::for_inverse(inverse));
    }
    for mount in montages {
        builder.insert(mount.anchor, &mount.lines)?;
    }

    builder.patch(plan::PatchToml::InscrireFeature(module.clone()))?;

    // L'inventaire décrit le projet tel que ce plan le laissera : c'est le `Some(&module)`
    // qui l'y fait nommer la feature, le manifeste du disque l'ignorant encore.
    let zone_manquante = crate::agents::refresh(&mut builder, &root, &metadonnees, Some(&module))?;

    Ok(Planned {
        plan: builder.finir(),
        files: files.into_iter().map(|(path, _)| path).collect(),
        migration,
        avertissement,
        seed_skipped,
        zone_manquante,
    })
}

/// Calcule ce que `--has-many` écrirait dans le modèle de la feature déjà présente.
///
/// Une réparation, non une génération : la table nommée doit déjà exister, et chaque
/// entité enfant nommée doit déjà porter, dans son propre modèle, la clé qui la rattache
/// à elle — sans quoi la variante posée décrirait une relation que SeaORM refuserait à la
/// compilation.
///
/// La feature réparée se cherche dans l'inventaire et non dans l'arborescence : une entité
/// peut vivre ailleurs que sous le répertoire de son nom — `users` est nichée dans
/// `src/auth/model.rs` — et la réclamer sous `src/users/` la déclarerait absente.
fn plan_repair(
    options: &Options,
    root: &Path,
    metadonnees: &metadata::Metadata,
) -> Result<Planned, Error> {
    let module = options.name.clone();
    let entities = entities::scan(root);

    let Some(parent) = entities::find(&entities, &module).cloned() else {
        return Err(Error::Absente {
            path: format!("src/{module}"),
            feature: module,
        });
    };

    let own_file = parent.file.clone();

    let mut inverses = Vec::with_capacity(options.has_many.len());
    for child_name in &options.has_many {
        let Some(child) = entities::find(&entities, child_name) else {
            return Err(Error::Relations(vec![
                relations::ResolveError::UnknownTarget(relations::UnknownTarget {
                    relation: "has-many".to_string(),
                    target: child_name.clone(),
                    known: entities::tables(&entities),
                }),
            ]));
        };

        if !relations::child_references(child, &parent, root) {
            return Err(Error::EnfantSansCle {
                child: child_name.clone(),
                table: module.clone(),
            });
        }

        let variant = to_pascal_case(&child.table);
        let target_entity = format!("{}::Entity", child.module_path);
        // Sans indentation propre sur `variant` : voir le commentaire de
        // `relations::inverses`, qui vaut ici à l'identique.
        inverses.push(relations::Inverse {
            file: own_file.clone(),
            entity: parent.table.clone(),
            variant: vec![
                format!(r#"#[sea_orm(has_many = "{target_entity}")]"#),
                format!("{variant},"),
            ],
            related: relations::related_impl(&target_entity, &variant),
        });
    }

    // Toutes les cibles avant la première écriture : un enfant fautif au milieu de la
    // liste ne doit pas laisser les précédents à moitié montés.
    let existing = fs::read_to_string(root.join(&own_file)).unwrap_or_default();
    for inverse in &inverses {
        if relations::homonymous_conflict(&existing, inverse) {
            return Err(Error::Homonyme {
                file: inverse.file.clone(),
                variant: inverse.variant.last().cloned().unwrap_or_default(),
            });
        }
    }

    let mut builder = plan::Builder::new(root);
    for inverse in &inverses {
        for mount in mount::for_inverse(inverse) {
            builder.insert(mount.anchor, &mount.lines)?;
        }
    }

    // La réparation n'inscrit aucune feature nouvelle : l'inventaire est régénéré à
    // l'identique, et son action prend le statut « déjà fait » plutôt que de réécrire un
    // fichier conforme.
    let zone_manquante = crate::agents::refresh(&mut builder, root, metadonnees, None)?;

    Ok(Planned {
        plan: builder.finir(),
        files: Vec::new(),
        migration: None,
        avertissement: None,
        seed_skipped: None,
        zone_manquante,
    })
}

/// Nom de la relation dont la référence requise rend `feature` non semable.
///
/// N'est appelée que quand `is_seedable` a déjà répondu non : une référence bloquante
/// existe forcément.
fn unseedable_reference(feature: &Feature) -> String {
    feature
        .fields
        .iter()
        .find(|field| field.reference().is_some() && !field.optional)
        .expect("is_seedable a déjà établi qu'une référence requise existe")
        .relation_name()
        .to_string()
}

/// Rend les fichiers de la feature, et sa migration si elle est complète.
///
/// Rien n'est écrit ici : une template fautive doit échouer avant la première écriture.
fn render(
    feature: &Feature,
    complete: bool,
    seedable: bool,
    crate_name: Option<&str>,
) -> Result<(Vec<File>, Option<String>), Error> {
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
    }

    if complete && seedable {
        // Hors du répertoire de la feature : le seed appartient au binaire qui l'applique,
        // et non au module que le routeur monte.
        files.push((
            format!("src/seeds/{module}.rs"),
            seed::render(feature, crate_name),
        ));
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
                lang: crate::lang::Lang::Fr,
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
            has_many: Vec::new(),
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

    /// L'inventaire est ce que l'agent lit pour savoir ce que le projet porte : une
    /// entité engendrée qui n'y figure pas le renvoie explorer le disque.
    #[test]
    fn generating_a_crud_names_the_entity_in_the_agents_inventory() {
        let (_parent, root) = project();

        run(&options(&root, "articles", Some("title:string"), true))
            .expect("articles doit se générer");

        let agents = read(&root.join("AGENTS.md"));

        assert!(agents.contains("articles"), "{agents}");
        assert!(
            agents.contains("## Notes du projet"),
            "l'écriture a débordé de la zone : {agents}"
        );
    }

    /// Un fichier de documentation supprimé ne doit pas empêcher d'engendrer une feature.
    #[test]
    fn a_missing_agents_file_does_not_stop_the_generation() {
        let (_parent, root) = project();
        fs::remove_file(root.join("AGENTS.md")).expect("le fichier existe");

        let genere = run(&options(&root, "articles", Some("title:string"), true));

        assert!(genere.is_ok(), "{:?}", genere.err());
    }

    /// La zone est régénérée, non complétée : deux générations successives laissent une
    /// seule mention de chaque entité.
    #[test]
    fn the_inventory_names_each_entity_once() {
        let (_parent, root) = project();

        run(&options(&root, "articles", Some("title:string"), true))
            .expect("articles doit se générer");
        run(&options(&root, "comments", Some("body:string"), true))
            .expect("comments doit se générer");

        let agents = read(&root.join("AGENTS.md"));
        let entites = agents
            .lines()
            .find(|line| line.contains("Entités"))
            .expect("la ligne des entités est rendue");

        assert_eq!(entites.matches("articles").count(), 1, "{entites}");
        assert!(entites.contains("comments"), "{entites}");
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

        assert!(read(&root.join("src/lib.rs")).contains("pub mod articles;"));
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

    /// Une référence requise rend l'entité non semable : ni le fichier ni son montage ne
    /// sont produits, et le plan nomme la relation en cause.
    #[test]
    fn a_required_reference_writes_no_seed_and_names_the_relation() {
        let (_parent, root) = project();
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");

        let planned = run(&options(
            &root,
            "posts",
            Some("title:string,author:references:users"),
            true,
        ))
        .expect("la génération doit aboutir malgré la référence requise");

        assert!(
            !root.join("src/seeds/posts.rs").exists(),
            "un seed a été écrit malgré la référence requise"
        );

        let binaire = read(&root.join("src/seeds/main.rs"));
        let ancre = crate::anchors::body(&binaire, crate::anchors::SEEDS)
            .expect("l'ancre des seeds est présente");
        assert!(
            !ancre.contains("posts,"),
            "le seed écarté ne doit pas être monté :\n{binaire}"
        );

        assert_eq!(planned.seed_skipped.as_deref(), Some("author"));
    }

    /// Une référence optionnelle, elle, ne bloque rien : le seed se sème à `None`.
    #[test]
    fn an_optional_reference_still_writes_its_seed() {
        let (_parent, root) = project();
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");

        let planned = run(&options(
            &root,
            "posts",
            Some("title:string,author:references:users:optional"),
            true,
        ))
        .expect("la génération doit aboutir");

        assert!(root.join("src/seeds/posts.rs").exists(), "le seed manque");
        assert_eq!(planned.seed_skipped, None);
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
        assert!(!read(&root.join("src/lib.rs")).contains("mod articles;"));
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

    /// La tâche du lot : une référence écrit le côté inverse dans le modèle de sa cible.
    #[test]
    fn a_reference_writes_the_inverse_side_into_the_target_model() {
        let (_parent, root) = project();
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");

        run(&options(
            &root,
            "posts",
            Some("title:string,author:references:users"),
            true,
        ))
        .expect("posts doit se générer");

        let cible = read(&root.join("src/users/model.rs"));
        assert!(
            cible.contains(r#"has_many = "crate::posts::model::Entity""#),
            "{cible}"
        );
        assert!(cible.contains("    Posts,"), "{cible}");
        assert!(
            cible.contains("impl Related<crate::posts::model::Entity> for Entity {"),
            "{cible}"
        );
    }

    /// Garde-fou contre la rechute : le binaire des seeds rejoignait autrefois l'entité
    /// par `#[path = "../<feature>/model.rs"]`, une racine de crate distincte qui ne
    /// voyait pas le côté inverse qu'une relation écrit dans le modèle d'une autre
    /// feature — la cause exacte du défaut que la bibliothèque du projet corrige. Une
    /// relation entre deux entités semables est le cas qui le déclenchait.
    ///
    /// L'interdit ne vaut que pour un projet **portant** une bibliothèque : sans elle, le
    /// chemin de crate ne mène nulle part, et `#[path]` redevient la seule façon pour le
    /// binaire des seeds d'atteindre l'entité. Un tel projet n'a de toute façon pas de
    /// relation à voir — c'est précisément pourquoi il n'a pas de bibliothèque.
    #[test]
    fn a_project_with_a_library_reaches_no_module_through_a_path_attribute() {
        let (_parent, root) = project();
        assert!(
            root.join("src/lib.rs").exists(),
            "le garde-fou ne vaut que sur un projet qui porte une bibliothèque"
        );
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");
        run(&options(
            &root,
            "posts",
            Some("title:string,author:references:users"),
            true,
        ))
        .expect("posts doit se générer");

        for (path, content) in fingerprint(&root) {
            assert!(
                !content.contains("#[path"),
                "{} rejoint un module par `#[path]` :\n{content}",
                path.display()
            );
        }
    }

    /// Le trou trouvé en relecture d'une tâche antérieure : `generate feature` écrit un
    /// modèle sans migration, et une relation qui le viserait poserait une clé étrangère
    /// vers une table qu'aucune migration ne crée.
    #[test]
    fn a_reference_to_a_model_without_a_migration_is_refused() {
        let (_parent, root) = project();
        run(&options(&root, "users", None, false)).expect("la feature vide doit se générer");

        let error = run(&options(
            &root,
            "posts",
            Some("author:references:users"),
            true,
        ))
        .expect_err("users n'a pas de migration");

        assert!(error.to_string().contains("users"), "{error}");
        assert!(
            !root.join("src/posts").is_dir(),
            "des fichiers ont été écrits malgré la migration absente"
        );
    }

    /// Deux relations du même plan qui se singularisent pareil : la génération s'arrête
    /// avant d'écrire un `enum Relation` que rustc refuserait.
    #[test]
    fn two_relations_singularising_alike_are_refused_before_any_write() {
        let (_parent, root) = project();
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");
        let avant = fingerprint(&root);

        let error = run(&options(
            &root,
            "posts",
            Some("author:references:users,authors:references:users"),
            true,
        ))
        .expect_err("les deux relations réclament la variante Author");

        assert!(error.to_string().contains("Author"), "{error}");
        assert!(error.to_string().contains("authors"), "{error}");
        assert_eq!(
            fingerprint(&root),
            avant,
            "rien ne doit être écrit quand deux relations se heurtent"
        );
    }

    /// Le point de vigilance de la tâche : une variante déjà présente sous ce nom, mais
    /// visant une autre cible, refuse plutôt que de laisser deux relations homonymes dans
    /// la même énumération.
    #[test]
    fn a_homonymous_variant_towards_another_target_is_refused() {
        let (_parent, root) = project();
        run(&options(&root, "users", Some("email:string:unique"), true))
            .expect("users doit se générer");

        let modele = root.join("src/users/model.rs");
        let source = read(&modele);
        let pollue = source.replace(
            "    // <rbs:relations:users>\n",
            "    // <rbs:relations:users>\n    \
             #[sea_orm(has_many = \"crate::somewhere::model::Entity\")]\n    Posts,\n",
        );
        fs::write(&modele, pollue).expect("l'écriture aboutit");
        let avant = fingerprint(&root);

        let error = run(&options(
            &root,
            "posts",
            Some("title:string,author:references:users"),
            true,
        ))
        .expect_err("la variante Posts est déjà prise par une autre cible");

        assert!(error.to_string().contains("users"), "{error}");
        assert!(error.to_string().contains("Posts"), "{error}");
        assert_eq!(
            fingerprint(&root),
            avant,
            "rien ne doit être écrit quand l'inverse est en conflit"
        );
    }

    /// `--has-many` répare une feature déjà là : rien à créer, seul le modèle de la
    /// feature réparée reçoit le côté inverse.
    #[test]
    fn has_many_writes_the_inverse_into_an_already_generated_feature() {
        let (_parent, root) = project();
        run(&options(&root, "posts", Some("title:string"), true)).expect("posts doit se générer");
        run(&options(
            &root,
            "comments",
            Some("body:string,post:references:posts"),
            true,
        ))
        .expect("comments doit se générer");

        // Retire le côté inverse que la génération de `comments` venait d'écrire, pour
        // rejouer précisément ce que `--has-many` doit réparer.
        let modele = root.join("src/posts/model.rs");
        let variant_block = "    #[sea_orm(has_many = \"crate::comments::model::Entity\")]\n    \
             Comments,\n";
        let related_block = "impl Related<crate::comments::model::Entity> for Entity {\n    \
             fn to() -> RelationDef {\n        Relation::Comments.def()\n    }\n}\n";
        let sans_inverse = read(&modele)
            .replace(variant_block, "")
            .replace(related_block, "");
        fs::write(&modele, &sans_inverse).expect("l'écriture aboutit");
        assert!(
            !sans_inverse.contains("Comments"),
            "l'inverse doit être retiré avant le test : {sans_inverse}"
        );

        let avant = fingerprint(&root);

        run(&Options {
            has_many: vec!["comments".to_string()],
            ..options(&root, "posts", None, false)
        })
        .expect("la réparation doit aboutir");

        let repare = read(&modele);
        assert!(
            repare.contains(r#"has_many = "crate::comments::model::Entity""#),
            "{repare}"
        );
        assert!(repare.contains("    Comments,"), "{repare}");
        assert!(
            repare.contains("impl Related<crate::comments::model::Entity> for Entity {"),
            "{repare}"
        );

        let apres = fingerprint(&root);
        assert_eq!(
            apres.keys().collect::<Vec<_>>(),
            avant.keys().collect::<Vec<_>>(),
            "la réparation ne doit créer ni supprimer aucun fichier"
        );
        let touches: Vec<&PathBuf> = avant
            .iter()
            .filter(|(chemin, contenu)| apres.get(*chemin) != Some(contenu))
            .map(|(chemin, _)| chemin)
            .collect();
        assert_eq!(
            touches,
            vec![&PathBuf::from("src/posts/model.rs")],
            "la réparation ne doit toucher que le modèle réparé"
        );
    }

    /// `--has-many` répare une feature déjà là : elle refuse quand cette feature n'existe
    /// pas encore.
    #[test]
    fn has_many_on_a_feature_that_does_not_exist_is_refused() {
        let (_parent, root) = project();

        let error = run(&Options {
            has_many: vec!["comments".to_string()],
            ..options(&root, "posts", None, false)
        })
        .expect_err("posts n'existe pas encore");

        assert!(error.to_string().contains("posts"), "{error}");
    }

    /// `--has-many` refuse une entité enfant qui ne porte pas la clé attendue : sans ce
    /// contrôle, SeaORM refuserait la variante posée, mais à la compilation seulement.
    #[test]
    fn has_many_on_a_child_without_the_expected_key_is_refused() {
        let (_parent, root) = project();
        run(&options(&root, "posts", Some("title:string"), true)).expect("posts doit se générer");
        run(&options(&root, "comments", Some("body:string"), true))
            .expect("comments doit se générer");

        let error = run(&Options {
            has_many: vec!["comments".to_string()],
            ..options(&root, "posts", None, false)
        })
        .expect_err("comments ne référence pas posts");

        assert!(error.to_string().contains("comments"), "{error}");
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

    /// Le critère qui manquait au lot précédent : un projet portant une relation compile
    /// réellement. Le binaire des seeds rejoignait autrefois l'entité par `#[path]`, une
    /// racine de crate distincte qui ne voit pas le côté inverse qu'une relation écrit
    /// dans le modèle de sa cible — `cargo build` échouait sur `bin "seed"` dès qu'une
    /// relation existait. La bibliothèque du projet supprime cette racine séparée.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn a_project_with_a_relation_compiles_its_two_binaries() {
        let project = bench::Project::fresh();

        for arguments in [
            vec![
                "generate",
                "crud",
                "users",
                "--fields",
                "email:string:unique",
            ],
            vec![
                "generate",
                "crud",
                "posts",
                "--fields",
                "title:string,author:references:users",
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

    /// Une relation vers les deux entités du fragment `auth`, qui vivent nichées dans un
    /// même `src/auth/model.rs` sous une migration nommée `create_auth_tables`.
    ///
    /// Deux défauts qu'aucun test n'attrapait, faute d'un projet qui les porte : la
    /// migration de `users` était cherchée par le nom du fichier, et toute relation vers
    /// une table du fragment se voyait refusée ; et le côté inverse allait dans la
    /// première paire d'ancres du fichier, quelle que soit l'entité visée — `refresh_tokens`
    /// recevait sa variante dans le module `user`, où `rustc` ne la relie à rien.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn a_relation_towards_each_auth_entity_lands_in_its_own_module_and_compiles() {
        let project = bench::Project::fresh();
        project.rbs_ok(&["add", "auth", "--yes"]);
        project.rbs_ok(&[
            "generate",
            "crud",
            "tickets",
            "--fields",
            "label:string,reporter:references:users:optional:nullify,\
             token:references:refresh_tokens:optional:nullify",
        ]);

        let model = read(&project.root().join("src/auth/model.rs"));
        let (users, refresh_tokens) = model
            .split_once("pub mod refresh_token {")
            .expect("le fragment auth déclare ses deux entités");

        assert!(
            users.contains(r#"has_many = "crate::tickets::model::Entity""#),
            "l'entité `users` doit recevoir son côté inverse :\n{users}"
        );
        assert!(
            refresh_tokens.contains(r#"has_many = "crate::tickets::model::Entity""#),
            "l'entité `refresh_tokens` doit recevoir le sien :\n{refresh_tokens}"
        );

        project.compile();
    }

    /// Deux features distinctes visant une même cible : la seconde y pose un second `impl
    /// Related`, dont l'accolade fermante passait pour déjà écrite — le modèle sortait
    /// avec un délimiteur non refermé, et le projet ne compilait plus.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn two_features_pointing_at_one_target_leave_it_a_model_that_compiles() {
        let project = bench::Project::fresh();
        project.rbs_ok(&[
            "generate",
            "crud",
            "owners",
            "--fields",
            "email:string:unique",
        ]);
        project.rbs_ok(&[
            "generate",
            "crud",
            "badges",
            "--fields",
            "holder:references:owners:unique",
        ]);
        project.rbs_ok(&[
            "generate",
            "crud",
            "stamps",
            "--fields",
            "issuer:references:owners:optional:nullify",
        ]);

        let model = read(&project.root().join("src/owners/model.rs"));
        assert_eq!(
            model.matches("impl Related<").count(),
            2,
            "les deux côtés inverses doivent être écrits :\n{model}"
        );
        assert_eq!(
            model.matches('{').count(),
            model.matches('}').count(),
            "les délimiteurs du modèle ne s'équilibrent plus :\n{model}"
        );

        project.compile();
    }

    /// Deux références d'une même feature vers une même cible : le côté portant renonce à
    /// son `impl Related`, faute de pouvoir arbitrer, et la cible doit renoncer à sa
    /// variante `has_many` — que `EntityTrait::has_many<R>` ne compile qu'avec un
    /// `R: Related<Self>`. Écrire l'une sans l'autre donnait un `E0277`.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn two_references_of_one_feature_towards_one_target_compile() {
        let project = bench::Project::fresh();
        project.rbs_ok(&[
            "generate",
            "crud",
            "editors",
            "--fields",
            "email:string:unique",
        ]);
        project.rbs_ok(&[
            "generate",
            "crud",
            "drafts",
            "--fields",
            "title:string,author:references:editors,reviewer:references:editors",
        ]);

        let target = read(&project.root().join("src/editors/model.rs"));
        assert!(
            !target.contains("#[sea_orm(has_many"),
            "la cible ne doit pas recevoir de variante `has_many` :\n{target}"
        );
        assert!(
            target.contains("`Author`, `Reviewer`"),
            "un commentaire doit dire pourquoi elle n'en reçoit pas :\n{target}"
        );

        project.compile();
    }

    /// Le parc engendré avant que le squelette ne porte une bibliothèque : une commande
    /// lancée aujourd'hui dans un tel projet doit continuer à produire du code qui
    /// compile. Le seed y visait `<crate>::<feature>::model`, un chemin qui ne mène nulle
    /// part, et le fragment `jobs` détachait son worker par le même chemin.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn a_project_without_a_library_still_gets_code_that_compiles() {
        let project = bench::Project::fresh();
        drop_the_library(project.root());

        project.rbs_ok(&["add", "jobs", "--yes"]);
        project.rbs_ok(&["generate", "crud", "labels", "--fields", "caption:string"]);

        let seed = read(&project.root().join("src/seeds/labels.rs"));
        assert!(
            seed.contains(r#"#[path = "../labels/model.rs"]"#),
            "sans bibliothèque, le seed rejoint l'entité par son chemin :\n{seed}"
        );

        let main = read(&project.root().join("src/main.rs"));
        assert!(
            main.contains("crate::jobs::worker::spawn(state.clone());"),
            "le worker se détache par `crate::`, faute de bibliothèque :\n{main}"
        );

        project.compile();
    }

    /// Ramène le projet à la forme qu'il avait avant que le squelette ne porte une
    /// bibliothèque : les modules de feature vivaient dans le binaire, déclarés sous
    /// l'ancre `<rbs:features>` de `src/main.rs`.
    fn drop_the_library(root: &Path) {
        fs::remove_file(root.join("src/lib.rs")).expect("la bibliothèque s'efface");

        let crate_name = crate::metadata::package_name(&root.join("Cargo.toml"))
            .expect("le manifeste se lit")
            .replace('-', "_");
        let main = root.join("src/main.rs");
        let source = read(&main);
        let rewritten = source.replace(
            &format!("use {crate_name}::{{router, state}};\n"),
            "mod health;\nmod openapi;\nmod router;\nmod state;\n\
             // <rbs:features>\n// </rbs:features>\n",
        );

        assert_ne!(
            rewritten, source,
            "le préambule du binaire doit avoir été réécrit"
        );
        fs::write(&main, rewritten).expect("l'écriture aboutit");
    }
}
