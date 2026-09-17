//! Traduit ce qu'un manifeste de fragment déclare en actions de retrait.
//!
//! Le miroir d'`add::installation` : la même lecture de manifeste, parcourue à l'envers.
//! Chaque section qu'`add` sait poser, celle-ci sait la défaire — sauf ce qu'aucun retrait
//! ne doit toucher : la variable d'environnement, qu'un `git checkout` ne réparerait pas
//! puisqu'elle vit dans un `.env` gitignoré, et la dépendance ou la feature Cargo qu'un
//! autre fragment installé réclame encore.
//!
//! Cette dernière règle — l'union des réclamations — est le piège de ce module : lire le
//! seul manifeste du fragment qui part ne dit rien de ce que les autres exigent encore de
//! `async-trait`, de `thiserror` ou de `tokio`. [`reclamees_ailleurs`] répond à cette
//! question en relisant le manifeste de chaque fragment installé, sauf le partant.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

use crate::add::installation;
use crate::anchors::{self, Anchor};
use crate::generate::mount;
use crate::manifest::{self, Manifest};
use crate::plan;
use crate::template::Renderer;
use crate::templates;

/// Ce que `add` apporte au squelette, et n'en retire donc jamais.
///
/// Un fragment leur ajoute des flags de feature par `[cargo.<crate>]` ; aucun n'en fait
/// une `[[dependencies]]` propre, ce qu'aucun fragment embarqué ne déclare aujourd'hui.
/// La garde reste explicite : un `--template-dir` pourrait écrire un manifeste qui s'y
/// risque, et le squelette ne doit pas dépendre de ce que les fragments choisissent de
/// ne pas faire.
// Sans appelant avant que `rbs remove` ne soit câblée à cette commande : `-D warnings`
// la dirait morte, alors que les tests en prouvent déjà le contrat.
#[allow(dead_code)]
const SQUELETTE: [&str; 3] = ["tokio", "sea-orm", "rbs-core"];

/// Le fragment tel que le retrait le voit.
// Idem : construit par les seuls tests avant que la commande n'existe.
#[allow(dead_code)]
pub(crate) struct Fragment<'a> {
    /// Nom de la feature, pour les messages d'erreur.
    pub name: &'a str,
    /// Ce que son manifeste déclare.
    pub manifest: &'a Manifest,
    /// Ses templates, telles que la source les a lues — pour rendre le contenu attendu
    /// des fichiers qu'il a déposés, et le comparer à ce que le disque en garde.
    pub templates: &'a [templates::File],
    /// Contexte de rendu, déduit du projet visé.
    pub context: minijinja::Value,
    /// Ce que les autres fragments installés réclament encore.
    pub reclamees: &'a Reclamees,
}

/// Ce que les fragments installés, le partant excepté, réclament encore.
///
/// Calculé une fois par [`reclamees_ailleurs`] et prêté au retrait : lui seul sait quels
/// autres fragments tournent, la question n'ayant de sens que pour l'appelant qui les
/// connaît tous.
// Idem : construit par les seuls tests avant que la commande n'existe.
#[allow(dead_code)]
pub(crate) struct Reclamees {
    /// Noms des crates tierces qu'au moins un autre fragment déclare en `[[dependencies]]`.
    pub dependencies: BTreeSet<String>,
    /// Pour chaque crate visée par un `[cargo.<crate>]`, les features qu'au moins un
    /// autre fragment y active encore.
    pub cargo: BTreeMap<String, BTreeSet<String>>,
}

/// Ce que le retrait a fait, pour que l'appelant l'affiche.
// Idem : construit par les seuls tests avant que la commande n'existe.
#[allow(dead_code)]
pub(crate) struct Retires {
    /// Chemins des fichiers déclarés par le fragment, dans l'ordre où ils sont planifiés.
    pub fichiers: Vec<String>,
    /// Chemin de la migration retirée, si le fragment en déclarait une et qu'elle a été
    /// retrouvée.
    pub migration: Option<String>,
    /// Ce qui a été sciemment laissé : variables d'environnement, dépendances et features
    /// que d'autres fragments réclament encore.
    pub laissees: Vec<String>,
}

/// Ce qui peut empêcher d'interpréter un manifeste à l'envers.
// Idem : rendu par les seules fonctions de ce module, sans appelant avant que la
// commande `rbs remove` ne soit câblée.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Le manifeste vise une ancre que le squelette ne porte pas.
    #[error("{feature}/feature.toml vise l'ancre `{anchor}`, qui n'existe pas : {known}")]
    AncreInconnue {
        /// Feature en cours de retrait.
        feature: String,
        /// Nom d'ancre refusé.
        anchor: String,
        /// Les ancres du squelette, énumérées.
        known: String,
    },

    /// Une template déclarée par le fragment est absente de son arborescence.
    ///
    /// Ne peut venir que d'[`installation::a_deposer`] ou d'[`installation::template`],
    /// que ce module réutilise pour retrouver les mêmes destinations qu'`add` a posées :
    /// la même faute mérite le même message, qu'on pose ou qu'on retire.
    #[error("{0}")]
    Installation(#[from] installation::Error),

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

    /// Un fichier du projet, ou le manifeste d'un fragment installé ailleurs, n'a pas pu
    /// être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le manifeste d'un autre fragment installé ne se lit pas.
    #[error("{0}")]
    Manifest(#[from] manifest::Error),

    /// Deux fichiers de migration portent le même suffixe.
    ///
    /// `add` étant idempotent, une seconde migration du même nom signale une réécriture
    /// manuelle — un renommage, une copie — que le retrait ne doit pas trancher à la
    /// place de l'utilisateur en en choisissant une au hasard.
    #[error("plusieurs migrations correspondent à `{name}`, retrait refusé : {files}")]
    MigrationAmbigue {
        /// Nom déclaré par le manifeste, horodatage excepté.
        name: String,
        /// Les fichiers trouvés, énumérés.
        files: String,
    },
}

/// Retire du plan ce que le manifeste déclare, dans l'ordre inverse de son installation.
///
/// Chaque section qu'`installation::actions` sait poser trouve ici son inverse, à trois
/// exceptions près, documentées section par section : les variables d'environnement ne
/// sont jamais retirées, une dépendance ou une feature encore réclamée reste en place, et
/// le point de montage de `src/modules/` ne se supprime jamais — seule la ligne que le
/// fragment y a inscrite s'en va, portée comme n'importe quelle autre ancre par la
/// section 3.
// Sans appelant avant que `rbs remove` ne soit câblée à cette commande : `-D warnings`
// la dirait morte, alors que les tests en prouvent déjà le contrat.
#[allow(dead_code)]
pub(crate) fn actions(fragment: &Fragment, builder: &mut plan::Builder) -> Result<Retires, Error> {
    let renderer = Renderer::new();
    let mut fichiers = Vec::new();
    let mut laissees = Vec::new();

    // 1. Les fichiers : chaque destination déclarée est rendue puis planifiée en retrait,
    // qu'elle ait ou non été posée sous `if_absent` — elle l'a été, ou ne l'a pas été, et
    // `Builder::supprimer` rend `DejaFait` dans le second cas.
    for (destination, source, _if_absent) in
        installation::a_deposer(fragment.name, fragment.manifest, fragment.templates)?
    {
        let content = render(&renderer, fragment, source, &destination)?;
        builder.supprimer(&destination, &content)?;
        fichiers.push(destination);
    }

    // 2. La migration : retrouvée par son suffixe, jamais par un horodatage qu'aucun
    // manifeste ne garde.
    let mut migration = None;
    if let Some(declared) = &fragment.manifest.migration
        && let Some((module, path)) = migration_de(builder, declared)?
    {
        let source = installation::template(fragment.name, fragment.templates, &declared.source)?;
        let content = render(&renderer, fragment, source, &path)?;
        builder.supprimer(&path, &content)?;
        fichiers.push(path.clone());
        migration = Some(path);

        for mount in mount::for_migration(&module) {
            builder.retirer_lignes(mount.anchor, &mount.lines)?;
        }
    }

    // 3. Les ancres : `before` n'est pas vérifié ici, à la différence de l'installation —
    // il garde une insertion à sa place, et n'a rien à dire d'un retrait.
    //
    // C'est ici, et non par une suppression de fichier séparée, que le point de montage de
    // `src/modules/` perd la ligne que le fragment y avait inscrite : sa propre entrée
    // dans `fragment.manifest.anchors` suffit, comme pour n'importe quelle autre ancre.
    for insertion in &fragment.manifest.anchors {
        let anchor = anchor(fragment, &insertion.anchor, builder)?;
        let content = render(
            &renderer,
            fragment,
            &insertion.content,
            anchor.file.as_ref(),
        )?;
        builder.retirer_lignes(anchor, &installation::lines(&content))?;
    }

    // 4. Les dépendances : retirées seulement si aucun autre fragment installé ne les
    // réclame encore, et jamais si elles appartiennent au squelette.
    for declared in &fragment.manifest.dependencies {
        if SQUELETTE.contains(&declared.name.as_str()) {
            laissees.push(format!(
                "{} appartient au squelette, jamais retirée",
                declared.name
            ));
        } else if fragment.reclamees.dependencies.contains(&declared.name) {
            laissees.push(format!(
                "{} reste réclamée par un autre fragment installé",
                declared.name
            ));
        } else {
            builder.patch(plan::PatchToml::RetirerDependance(declared.name.clone()))?;
        }
    }

    // 5. Les features cargo : la crate elle-même n'est jamais retirée par ce chemin, que
    // ses features restent ou non — seule la section 4 retire une dépendance entière.
    for (dependency, patch) in &fragment.manifest.cargo {
        let reclamees_du_crate = fragment.reclamees.cargo.get(dependency);
        for feature in &patch.features {
            if reclamees_du_crate.is_some_and(|features| features.contains(feature)) {
                laissees.push(format!(
                    "{dependency}/{feature} reste réclamée par un autre fragment installé"
                ));
                continue;
            }

            builder.patch(plan::PatchToml::RetirerFeatureADependance {
                dependency: dependency.clone(),
                feature: feature.clone(),
            })?;
        }
    }

    // 6. Les sections de configuration.
    for section in &fragment.manifest.config {
        builder.retirer_section(&section.file, &section.section)?;
    }

    // 7. Les variables d'environnement : jamais retirées, seulement nommées. `.env` est
    // gitignoré — c'est la seule écriture qu'aucun `git checkout` ne réparerait, et la
    // décision de la retirer appartient au développeur, pas à cette commande.
    for variable in &fragment.manifest.env {
        laissees.push(format!(
            "{} n'est pas retirée de .env, à faire à la main si elle ne sert plus",
            variable.key
        ));
    }

    // 8. La métadonnée : la feature quitte l'inventaire du projet en dernier, une fois
    // tout ce qu'elle avait posé démonté.
    builder.patch(plan::PatchToml::RetirerFeature(fragment.name.to_string()))?;

    Ok(Retires {
        fichiers,
        migration,
        laissees,
    })
}

/// L'ancre du squelette que le manifeste désigne par `name`, résolue comme à l'installation.
// Sans appelant hors de `actions`, elle-même sans appelant avant `rbs remove`.
#[allow(dead_code)]
fn anchor(fragment: &Fragment, name: &str, builder: &plan::Builder) -> Result<Anchor, Error> {
    let anchor = anchors::ANCRES
        .into_iter()
        .find(|anchor| anchor.name == name)
        .ok_or_else(|| Error::AncreInconnue {
            feature: fragment.name.to_string(),
            anchor: name.to_string(),
            known: anchors::ANCRES
                .iter()
                .map(|anchor| anchor.name.as_ref())
                .collect::<Vec<_>>()
                .join(", "),
        })?;

    Ok(anchors::resolve(anchor, builder.exists("src/lib.rs")?))
}

/// Ce que les fragments installés réclament encore, `partant` excepté.
///
/// Un nom de `[package.metadata.rbs] features` sans fragment embarqué correspondant est
/// un CRUD engendré par `rbs generate crud`, pas une feature : cette liste mêle les deux,
/// et l'absence de fragment est ici son seul signe distinctif. Ce n'est pas une faute —
/// c'est le régime ordinaire d'un projet qui a généré des ressources — et le nom est donc
/// ignoré en silence plutôt que de faire échouer le retrait.
// Sans appelant avant que `rbs remove` ne soit câblée à cette commande : `-D warnings`
// la dirait morte, alors que les tests en prouvent déjà le contrat.
#[allow(dead_code)]
pub(crate) fn reclamees_ailleurs(
    template_dir: Option<&Path>,
    partant: &str,
    installees: &[String],
) -> Result<Reclamees, Error> {
    let mut dependencies = BTreeSet::new();
    let mut cargo: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for nom in installees {
        if nom == partant {
            continue;
        }

        let Ok(source) = templates::Source::feature(template_dir, nom) else {
            continue;
        };

        let (manifeste, _) = source
            .manifest_and_files()
            .map_err(|source| crate::errors::Acces::new(Path::new(nom.as_str()), source))?;
        let Some(texte) = manifeste else {
            continue;
        };

        let manifest = manifest::read(&texte, &format!("{nom}/feature.toml"))?;

        for declared in &manifest.dependencies {
            dependencies.insert(declared.name.clone());
        }
        for (dependency, patch) in &manifest.cargo {
            cargo
                .entry(dependency.clone())
                .or_default()
                .extend(patch.features.iter().cloned());
        }
    }

    Ok(Reclamees {
        dependencies,
        cargo,
    })
}

/// Retrouve la migration que `declared` désigne, par son suffixe.
///
/// L'horodatage n'est gardé nulle part : ni dans le manifeste, dont il ne dépend pas, ni
/// dans `[package.metadata.rbs]`, qui ne connaît que le nom de la feature. Le suffixe
/// `_{name}.rs` est donc la seule clé de recherche, et une deuxième correspondance est
/// refusée plutôt que tranchée : `add` étant idempotent, elle ne peut venir que d'une
/// réécriture manuelle — un renommage, une copie — que cette commande ne doit pas juger
/// à la place de l'utilisateur.
// Sans appelant hors de `actions`, elle-même sans appelant avant `rbs remove` ; les
// tests de ce module l'appellent directement, ce que la seule compilation `--lib` ignore.
#[allow(dead_code)]
pub(crate) fn migration_de(
    builder: &plan::Builder,
    declared: &manifest::DeclaredMigration,
) -> Result<Option<(String, String)>, Error> {
    let repertoire = builder.root().join("migration/src");
    let suffixe = format!("_{}.rs", declared.name);

    let entrees = match std::fs::read_dir(&repertoire) {
        Ok(entrees) => entrees,
        // Un projet sans crate `migration`, ou dont ce répertoire a disparu, n'a
        // simplement rien à retirer : ce n'est pas une faute d'accès.
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(crate::errors::Acces::new(&repertoire, source).into()),
    };

    let mut trouvees: Vec<String> = entrees
        .filter_map(Result::ok)
        .filter_map(|entree| entree.file_name().into_string().ok())
        .filter(|nom| nom.starts_with('m') && nom.ends_with(&suffixe))
        .collect();
    trouvees.sort();

    match trouvees.as_slice() {
        [] => Ok(None),
        [seule] => {
            let module = seule
                .strip_suffix(".rs")
                .expect("le filtre ne garde que des `.rs`")
                .to_string();
            Ok(Some((module, format!("migration/src/{seule}"))))
        }
        plusieurs => Err(Error::MigrationAmbigue {
            name: declared.name.clone(),
            files: plusieurs
                .iter()
                .map(|nom| format!("migration/src/{nom}"))
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

/// Rend `source` dans le contexte du fragment, en nommant `destination` si elle échoue.
// Sans appelant hors de `actions`, elle-même sans appelant avant `rbs remove`.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::metadata;

    /// Un projet, et le répertoire temporaire qui le porte : `rbs new --features` crée le
    /// second comme parent du premier, et les deux doivent survivre ensemble.
    struct Projet {
        _parent: TempDir,
        root: PathBuf,
    }

    impl Projet {
        fn path(&self) -> &Path {
            &self.root
        }
    }

    /// Le contexte de rendu du projet par défaut de [`crate::fixtures::Project`], qui
    /// crée toujours en français : `rate-limit` choisit son message d'erreur sur `lang`
    /// et son compteur sur la présence de `redis` dans `features` — absente ici, aucun
    /// des fragments posés par les tests de ce module ne l'installant avec lui.
    fn context() -> minijinja::Value {
        minijinja::context! {
            project_name => "demo-api",
            crate_name => "demo_api",
            crate_path => "demo_api",
            lang => "fr",
            features => Vec::<String>::new(),
        }
    }

    /// Pose `name`, et les fragments d'`autres` avec lui, sur un projet neuf, par le
    /// pipeline réel de `rbs new --features` : la seule façon d'obtenir un disque qui
    /// porte exactement ce qu'une installation aurait écrit, migrations et patchs de
    /// `Cargo.toml` compris.
    fn fragment_pose_avec(name: &str, autres: &[&str]) -> (Projet, Fragment<'static>) {
        let mut toutes: Vec<String> = vec![name.to_string()];
        toutes.extend(autres.iter().map(|nom| nom.to_string()));

        let (parent, root) = crate::fixtures::Project::new()
            .features(&toutes.iter().map(String::as_str).collect::<Vec<_>>())
            .create();

        let metadonnees = metadata::read(&root.join("Cargo.toml"))
            .expect("les métadonnées du projet posé se lisent");

        let source =
            templates::Source::feature(None, name).expect("le fragment retiré est embarqué");
        let (manifeste, templates) = source
            .manifest_and_files()
            .expect("le fragment retiré se lit");
        let manifest = manifest::read(
            &manifeste.expect("le fragment retiré porte un manifeste"),
            &format!("{name}/feature.toml"),
        )
        .expect("le manifeste retiré est valide");

        let reclamees = reclamees_ailleurs(None, name, &metadonnees.features)
            .expect("les réclamations des autres fragments se calculent");

        // Fuite délibérée : le fragment emprunte son nom, son manifeste, ses templates et
        // ses réclamations, et un test n'a pas de portée plus longue où les loger.
        let fragment = Fragment {
            name: Box::leak(name.to_string().into_boxed_str()),
            manifest: Box::leak(Box::new(manifest)),
            templates: Box::leak(templates.into_boxed_slice()),
            context: context(),
            reclamees: Box::leak(Box::new(reclamees)),
        };

        (
            Projet {
                _parent: parent,
                root,
            },
            fragment,
        )
    }

    fn fragment_pose(name: &str) -> (Projet, Fragment<'static>) {
        fragment_pose_avec(name, &[])
    }

    fn declared(name: &str) -> manifest::DeclaredMigration {
        manifest::DeclaredMigration {
            source: "migration.rs.jinja".to_string(),
            name: name.to_string(),
        }
    }

    /// Un projet temporaire portant déjà les fichiers donnés, pour les tests de
    /// [`migration_de`] qui n'ont pas besoin d'un projet installé pour de bon.
    fn projet_avec(fichiers: &[(&str, &str)]) -> TempDir {
        let projet = TempDir::new().expect("le répertoire temporaire se crée");
        for (chemin, contenu) in fichiers {
            let cible = projet.path().join(chemin);
            if let Some(parent) = cible.parent() {
                std::fs::create_dir_all(parent).expect("le répertoire du fichier se crée");
            }
            std::fs::write(cible, contenu).expect("l'écriture aboutit");
        }
        projet
    }

    /// Chaque fichier déclaré est planifié en suppression, comparé à son rendu.
    ///
    /// `cors` déclare aussi l'ancre `modules` : sa ligne quitte `src/modules/mod.rs`,
    /// mais le fichier lui-même reste — R8 l'interdit — d'où la quatrième exception,
    /// à côté du manifeste, de la configuration et du routeur que le fragment patche
    /// sans les supprimer.
    #[test]
    fn every_declared_file_is_planned_for_removal() {
        let (projet, fragment) = fragment_pose("cors");
        let mut builder = plan::Builder::new(projet.path());

        let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

        assert_eq!(retires.fichiers.len(), 3);
        assert!(
            builder
                .finir()
                .files()
                .iter()
                .all(|file| file.after.is_none()
                    || file.path.ends_with("Cargo.toml")
                    || file.path.ends_with("default.toml")
                    || file.path.ends_with("router.rs")
                    || file.path.ends_with("modules/mod.rs")),
            "un fichier touché n'est ni supprimé ni parmi les patchs attendus"
        );
    }

    /// Une dépendance qu'un autre fragment installé déclare encore reste en place.
    #[test]
    fn a_dependency_another_installed_fragment_still_declares_stays() {
        let (projet, fragment) = fragment_pose_avec("storage", &["jobs"]); // async-trait partagée
        let mut builder = plan::Builder::new(projet.path());

        let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

        assert!(
            retires
                .laissees
                .iter()
                .any(|laissee| laissee.contains("async-trait")),
            "la dépendance partagée doit être nommée comme laissée : {:?}",
            retires.laissees
        );
        let plan = builder.finir();
        let manifeste = plan
            .files()
            .iter()
            .find(|file| file.path == "Cargo.toml")
            .expect("le manifeste est visé");
        assert!(
            manifeste
                .after
                .as_deref()
                .expect("il reste")
                .contains("async-trait")
        );
    }

    /// Une feature cargo qu'un autre fragment installé demande encore reste active.
    #[test]
    fn a_cargo_feature_another_installed_fragment_still_asks_for_stays() {
        // `scheduler` et `jobs` demandent tous deux `tokio/time`.
        let (projet, fragment) = fragment_pose_avec("scheduler", &["jobs"]);
        let mut builder = plan::Builder::new(projet.path());

        actions(&fragment, &mut builder).expect("le retrait se planifie");

        let plan = builder.finir();
        let cargo = plan
            .files()
            .iter()
            .find(|file| file.path == "Cargo.toml")
            .expect("le manifeste est visé");
        let apres = cargo.after.as_deref().expect("le manifeste reste");
        assert!(
            apres.contains("\"time\""),
            "la feature partagée devait rester : {apres}"
        );
    }

    /// Une dépendance du squelette n'est jamais retirée, fût-elle vidée de ses features.
    #[test]
    fn a_skeleton_dependency_is_never_removed() {
        let (projet, fragment) = fragment_pose("rate-limit");
        let mut builder = plan::Builder::new(projet.path());

        actions(&fragment, &mut builder).expect("le retrait se planifie");

        let plan = builder.finir();
        let cargo = plan
            .files()
            .iter()
            .find(|file| file.path == "Cargo.toml")
            .expect("le manifeste est visé");
        assert!(
            cargo
                .after
                .as_deref()
                .expect("le manifeste reste")
                .contains("tokio"),
            "tokio appartient au squelette : un fragment lui ajoute des flags, il ne l'apporte pas"
        );
    }

    /// Même un fragment qui déclarerait `tokio` en `[[dependencies]]` propre — ce
    /// qu'aucun fragment embarqué ne fait aujourd'hui, `[cargo.tokio]` suffisant à tous —
    /// ne la retire jamais : la garde du squelette porte sur le nom, pas sur la façon
    /// dont un manifeste choisit de le réclamer.
    #[test]
    fn a_dependency_declared_on_a_skeleton_crate_is_left_in_place_and_named() {
        let projet = projet_avec(&[(
            "Cargo.toml",
            "[package]\nname = \"demo-api\"\n\n\
             [package.metadata.rbs]\nfeatures = [\"essai\"]\n\n\
             [dependencies]\ntokio = { version = \"1\", features = [\"rt-multi-thread\"] }\n",
        )]);
        let manifest = manifest::read(
            "[feature]\ndescription = \"essai\"\n\n\
             [[dependencies]]\nname = \"tokio\"\nversion = \"1\"\n",
            "essai/feature.toml",
        )
        .expect("le manifeste de test est valide");
        let reclamees = Reclamees {
            dependencies: BTreeSet::new(),
            cargo: BTreeMap::new(),
        };
        let fragment = Fragment {
            name: "essai",
            manifest: &manifest,
            templates: &[],
            context: context(),
            reclamees: &reclamees,
        };
        let mut builder = plan::Builder::new(projet.path());

        let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

        assert!(
            retires
                .laissees
                .iter()
                .any(|laissee| laissee.contains("tokio")),
            "la dépendance du squelette doit être nommée comme laissée : {:?}",
            retires.laissees
        );
        let plan = builder.finir();
        let cargo = plan
            .files()
            .iter()
            .find(|file| file.path == "Cargo.toml")
            .expect("le manifeste est visé");
        assert!(
            cargo
                .after
                .as_deref()
                .expect("le manifeste reste")
                .contains("tokio"),
            "{:?}",
            cargo.after
        );
    }

    /// La variable d'environnement reste, et le retrait la nomme.
    #[test]
    fn the_environment_variable_stays_and_is_named() {
        let (projet, fragment) = fragment_pose("mail");
        let mut builder = plan::Builder::new(projet.path());

        let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

        assert!(
            retires
                .laissees
                .iter()
                .any(|laissee| laissee.contains("RBS_MAIL__SMTP_PASSWORD"))
        );
    }

    /// La migration est retrouvée par son suffixe, son horodatage n'étant nulle part gardé.
    #[test]
    fn the_migration_is_found_by_its_suffix() {
        let projet = projet_avec(&[(
            "migration/src/m20260101_000000_create_auth_tables.rs",
            "// la migration\n",
        )]);
        let builder = plan::Builder::new(projet.path());

        let trouvee =
            migration_de(&builder, &declared("create_auth_tables")).expect("la recherche aboutit");

        assert_eq!(
            trouvee,
            Some((
                "m20260101_000000_create_auth_tables".to_string(),
                "migration/src/m20260101_000000_create_auth_tables.rs".to_string()
            ))
        );
    }

    /// Zéro fichier de migration : rien à retirer, et ce n'est pas une faute.
    #[test]
    fn no_matching_migration_file_is_not_an_error() {
        let projet = projet_avec(&[("migration/src/lib.rs", "// vide\n")]);
        let builder = plan::Builder::new(projet.path());

        let trouvee = migration_de(&builder, &declared("create_auth_tables"))
            .expect("l'absence n'est pas une faute");

        assert_eq!(trouvee, None);
    }

    /// Deux migrations de même suffixe arrêtent la commande plutôt que d'en choisir une.
    #[test]
    fn two_migrations_of_the_same_suffix_stop_the_command() {
        let projet = projet_avec(&[
            (
                "migration/src/m20260101_000000_create_auth_tables.rs",
                "// une\n",
            ),
            (
                "migration/src/m20260202_000000_create_auth_tables.rs",
                "// deux\n",
            ),
        ]);
        let builder = plan::Builder::new(projet.path());

        let erreur = migration_de(&builder, &declared("create_auth_tables"))
            .expect_err("l'ambiguïté doit être refusée");

        assert!(matches!(erreur, Error::MigrationAmbigue { .. }), "{erreur}");
        assert!(
            erreur
                .to_string()
                .contains("m20260101_000000_create_auth_tables.rs")
                && erreur
                    .to_string()
                    .contains("m20260202_000000_create_auth_tables.rs"),
            "les deux fichiers doivent être nommés : {erreur}"
        );
    }

    /// Un nom de `[package.metadata.rbs] features` sans fragment embarqué — un CRUD
    /// engendré — est ignoré plutôt que refusé.
    #[test]
    fn a_generated_crud_without_an_embedded_fragment_is_ignored() {
        let reclamees =
            reclamees_ailleurs(None, "cors", &["cors".to_string(), "posts".to_string()])
                .expect("un nom sans fragment ne fait pas échouer le calcul");

        assert!(reclamees.dependencies.is_empty());
        assert!(reclamees.cargo.is_empty());
    }

    /// Le partant lui-même n'entre jamais dans ses propres réclamations.
    #[test]
    fn the_departing_fragment_never_reclaims_from_itself() {
        let reclamees =
            reclamees_ailleurs(None, "cors", &["cors".to_string()]).expect("le calcul aboutit");

        assert!(reclamees.dependencies.is_empty());
    }
}
