//! `rbs upgrade` : le projet aligné sur la version du CLI, et rien d'autre.
//!
//! La commande n'écrit que dans `Cargo.toml`. Le reste du projet — contrôleurs,
//! configuration, migrations — appartient au développeur dès que `rbs new` l'a posé : le
//! re-rendre sur une version plus récente effacerait son travail sans qu'il l'ait demandé
//! nommément.
//!
//! La séquence est celle du projet — lire → planifier → vérifier → afficher → appliquer.
//! La garde Git vient après le plan, et non avant : un projet déjà à jour n'a rien à
//! protéger, et doit pouvoir répondre « rien à faire » depuis un working tree sale.

use std::io;
use std::path::PathBuf;

use crate::git;
use crate::metadata;
use crate::plan;

/// La dépendance dont la version suit celle du projet.
const NOYAU: &str = "rbs-core";

/// Version du CLI qui met à niveau.
const CLI: &str = env!("CARGO_PKG_VERSION");

/// Ce qu'il faut savoir pour mettre un projet à niveau.
pub(crate) struct Options {
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Met à niveau même si le projet porte des modifications non commitées.
    pub force: bool,
}

/// Ce qu'une mise à niveau fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Version de rbs qui a généré le projet.
    pub depuis: String,
    /// Version sur laquelle le projet sera aligné.
    pub vers: String,
    /// Le projet porte déjà cette version : le plan n'écrit rien.
    pub deja_a_jour: bool,
}

/// Ce qui peut empêcher de mettre un projet à niveau.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs upgrade` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le répertoire visé n'a pas pu être résolu.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif.
        path: String,
        /// Cause système.
        source: io::Error,
    },

    /// Le projet a été généré par un rbs postérieur au CLI qui le lit.
    ///
    /// Le cas courant est celui de deux CLI installés : nommer les deux numéros est la
    /// seule façon pour le développeur de savoir lequel des deux il vient de lancer.
    #[error(
        "le projet est en rbs {projet}, le CLI en {cli} : `rbs upgrade` ne redescend pas \
         un projet — relancez-le avec un CLI en {projet} ou plus récent"
    )]
    CliAnterieur {
        /// Version qui a généré le projet.
        projet: String,
        /// Version du CLI qui vient de la lire.
        cli: String,
    },

    /// Le projet porte des modifications non commitées, qu'une mise à niveau rendrait
    /// indiscernables des siennes.
    #[error("le working tree n'est pas propre : {files} — commitez, ou relancez avec --force")]
    WorkingTreeSale {
        /// Fichiers suivis modifiés, énumérés.
        files: String,
    },

    /// Le manifeste du projet n'a pu être lu ou patché.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Le plan de la mise à niveau n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

/// Calcule ce que la mise à niveau ferait au projet, sans rien écrire.
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    plan_for_with(options, CLI)
}

/// Le même, la version visée étant donnée en paramètre.
///
/// C'est ce qui rend les deux chemins — mise à niveau et refus — exerçables de part et
/// d'autre d'une publication, sans attendre qu'elle ait eu lieu.
pub(crate) fn plan_for_with(options: &Options, cli: &str) -> Result<Planned, Error> {
    let start = options
        .directory
        .canonicalize()
        .map_err(|source| Error::Acces {
            path: options.directory.display().to_string(),
            source,
        })?;
    let root = metadata::project_root(&start).ok_or(Error::PasUnProjet)?;

    let depuis = metadata::read(&root.join("Cargo.toml"))?.version;

    if posterieure(&depuis, cli) {
        return Err(Error::CliAnterieur {
            projet: depuis,
            cli: cli.to_string(),
        });
    }

    let mut builder = plan::Builder::new(root.clone());
    builder.patch(plan::PatchToml::AlignerSurVersion {
        dependency: NOYAU.to_string(),
        version: cli.to_string(),
    })?;
    let plan = builder.finir();

    let deja_a_jour = plan
        .files()
        .iter()
        .all(|file| file.statut == plan::Status::DejaFait);

    if !deja_a_jour && !options.force {
        let modifies = git::modified_files(&root);
        if !modifies.is_empty() {
            return Err(Error::WorkingTreeSale {
                files: git::enumerate(&modifies),
            });
        }
    }

    Ok(Planned {
        plan,
        depuis,
        vers: cli.to_string(),
        deja_a_jour,
    })
}

/// `projet` est-elle postérieure à `cli` ?
///
/// Une comparaison de versions ne vaut pas une dépendance de plus pour une seule
/// question. Ce qui ne se réduit pas à trois nombres n'est pas tenu pour postérieur :
/// mieux vaut mettre à niveau un manifeste douteux que refuser à tort.
pub(crate) fn posterieure(projet: &str, cli: &str) -> bool {
    matches!(
        nombres(projet).zip(nombres(cli)),
        Some((projet, cli)) if projet > cli
    )
}

/// Les trois nombres d'une version, la pré-publication et les métadonnées de compilation
/// laissées de côté.
pub(crate) fn nombres(version: &str) -> Option<[u64; 3]> {
    let mut parts = version
        .split(['-', '+'])
        .next()
        .expect("un `split` rend toujours au moins un fragment")
        .split('.');

    let mut nombres = [0; 3];
    for nombre in &mut nombres {
        *nombre = parts.next()?.parse().ok()?;
    }

    parts.next().is_none().then_some(nombres)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::SystemTime;

    use tempfile::TempDir;

    use super::*;

    /// Une version que le dépôt n'atteindra pas de sitôt : les tests qui la visent
    /// exercent la mise à niveau sans dépendre du numéro courant du workspace.
    const FUTUR: &str = "1.0.0";

    /// Un projet neuf, tel que `rbs new` l'écrit, dans son dépôt Git.
    fn project(core_path: Option<PathBuf>) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    fn options(root: &Path) -> Options {
        Options {
            directory: root.to_path_buf(),
            force: false,
        }
    }

    /// Le manifeste du projet, tel qu'il est sur le disque.
    fn manifeste(root: &Path) -> String {
        fs::read_to_string(root.join("Cargo.toml")).expect("le manifeste est lisible")
    }

    /// Applique la mise à niveau vers `cli` et rend ce qui en a été planifié.
    fn upgrade(root: &Path, cli: &str) -> Planned {
        let planned =
            plan_for_with(&options(root), cli).expect("la mise à niveau doit se planifier");
        plan::application::apply(&planned.plan, false).expect("le plan doit s'appliquer");

        planned
    }

    fn git(root: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("git doit être lançable");

        assert!(
            output.status.success(),
            "git {arguments:?} a échoué :\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Commite le projet entier : `rbs new` initialise le dépôt sans rien y déposer, et
    /// un `git diff` n'a rien à montrer tant que rien n'est suivi.
    fn commit(root: &Path) {
        git(root, &["config", "user.email", "rbs@example.test"]);
        git(root, &["config", "user.name", "rbs"]);
        git(root, &["add", "-A"]);
        git(root, &["commit", "--quiet", "-m", "initial"]);
    }

    /// Le contenu et la date de chaque fichier du projet, `.git` excepté.
    ///
    /// La date seule ne suffirait pas — deux écritures dans la même milliseconde se
    /// confondraient — et le contenu seul laisserait passer une réécriture à l'identique,
    /// qui reste des octets écrits.
    fn empreinte(root: &Path) -> BTreeMap<PathBuf, (SystemTime, Vec<u8>)> {
        let mut empreinte = BTreeMap::new();
        let mut a_visiter = vec![root.to_path_buf()];

        while let Some(directory) = a_visiter.pop() {
            for entry in fs::read_dir(&directory).expect("le répertoire est lisible") {
                let path = entry.expect("l'entrée est lisible").path();

                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }

                if path.is_dir() {
                    a_visiter.push(path);
                    continue;
                }

                let modifie = fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .expect("la date de modification est lisible");

                empreinte.insert(
                    path.clone(),
                    (modifie, fs::read(&path).expect("fichier lisible")),
                );
            }
        }

        empreinte
    }

    #[test]
    fn a_project_behind_the_cli_is_aligned_on_both_numbers() {
        let (_parent, root) = project(None);

        let planned = plan_for_with(&options(&root), FUTUR).expect("la mise à niveau se planifie");

        assert!(
            !planned.deja_a_jour,
            "un projet en retard a quelque chose à faire"
        );
        assert_eq!(planned.depuis, CLI);
        assert_eq!(planned.vers, FUTUR);

        let rendu = plan::render::plan(&planned.plan);
        assert!(rendu.contains("Cargo.toml"), "{rendu}");
        assert!(rendu.contains("modifié"), "{rendu}");

        plan::application::apply(&planned.plan, false).expect("le plan doit s'appliquer");

        let manifeste = manifeste(&root);
        assert!(
            manifeste.contains(&format!("rbs-core = {{ version = \"{FUTUR}\"")),
            "le noyau n'a pas suivi :\n{manifeste}"
        );
        assert_eq!(
            metadata::read(&root.join("Cargo.toml"))
                .expect("le manifeste reste lisible")
                .version,
            FUTUR
        );
    }

    #[test]
    fn a_project_already_on_the_target_has_nothing_to_do_and_writes_nothing() {
        let (_parent, root) = project(None);
        assert!(
            !upgrade(&root, FUTUR).deja_a_jour,
            "la première mise à niveau doit avoir eu quelque chose à faire"
        );

        let avant = empreinte(&root);
        let planned = plan_for_with(&options(&root), FUTUR).expect("la mise à niveau se planifie");

        assert!(planned.deja_a_jour, "le projet est déjà en {FUTUR}");
        assert!(
            plan::render::plan(&planned.plan).contains("inchangé"),
            "{}",
            plan::render::plan(&planned.plan)
        );
        assert_eq!(empreinte(&root), avant, "un octet a été écrit");
    }

    #[test]
    fn a_project_ahead_of_the_cli_is_refused_by_naming_both_versions() {
        let (_parent, root) = project(None);
        upgrade(&root, "9.9.9");

        let error = plan_for_with(&options(&root), FUTUR)
            .expect_err("un CLI antérieur au projet doit refuser");

        let message = error.to_string();
        assert!(message.contains("9.9.9"), "{message}");
        assert!(message.contains(FUTUR), "{message}");
    }

    #[test]
    fn nothing_but_the_manifest_is_touched() {
        let (_parent, root) = project(None);
        commit(&root);

        upgrade(&root, FUTUR);

        assert_eq!(git(&root, &["status", "--porcelain"]), " M Cargo.toml\n");
    }

    #[test]
    fn a_core_taken_from_a_local_path_keeps_its_path() {
        let noyau = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rbs-core");
        let (_parent, root) = project(Some(noyau));

        upgrade(&root, FUTUR);

        let manifeste = manifeste(&root);
        assert!(
            manifeste.contains("rbs-core = { path ="),
            "le mode développement a été écrasé :\n{manifeste}"
        );
        assert_eq!(
            metadata::read(&root.join("Cargo.toml"))
                .expect("le manifeste reste lisible")
                .version,
            FUTUR,
            "la métadonnée doit suivre même sans version à changer sur le noyau"
        );
    }

    #[test]
    fn the_default_targets_the_version_of_the_cli() {
        let (_parent, root) = project(None);

        let planned = plan_for(&options(&root)).expect("la mise à niveau se planifie");

        assert_eq!(planned.vers, CLI);
        assert!(
            planned.deja_a_jour,
            "un projet neuf est déjà à la version du CLI"
        );
    }

    #[test]
    fn a_dirty_working_tree_blocks_an_upgrade_that_has_something_to_write() {
        let (_parent, root) = project(None);
        commit(&root);
        fs::write(root.join("src/main.rs"), "// en cours\n").expect("fichier réécrivable");

        let error =
            plan_for_with(&options(&root), FUTUR).expect_err("un working tree sale doit bloquer");

        assert!(error.to_string().contains("src/main.rs"), "{error}");
    }

    #[test]
    fn a_prerelease_is_not_taken_for_a_later_version() {
        assert!(!posterieure("1.0.0-rc.1", "1.0.0"));
        assert!(posterieure("1.0.1", "1.0.0"));
        assert!(!posterieure("0.4.0", "1.0.0"));
        // Un numéro qu'on ne sait pas lire ne doit pas se déguiser en refus.
        assert!(!posterieure("maison", "1.0.0"));
    }
}
