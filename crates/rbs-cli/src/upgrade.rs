//! `rbs upgrade` : le projet aligné sur la version du CLI, et rien d'autre.
//!
//! La commande n'écrit que dans `Cargo.toml` et dans les deux zones réservées de
//! `AGENTS.md`. Le reste du projet — contrôleurs, configuration, migrations, et tout ce
//! que le développeur écrit hors de ces zones — appartient au développeur dès que
//! `rbs new` l'a posé : le re-rendre sur une version plus récente effacerait son travail
//! sans qu'il l'ait demandé nommément. Le guide, lui, est du texte que rbs produit et
//! versionne : un projet mis à niveau doit recevoir le mode d'emploi de sa nouvelle
//! version, sans quoi tout agent qui le lit travaille sur une documentation périmée.
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
    /// La zone de l'`AGENTS.md` que le plan n'a pas pu réécrire, faute de la trouver.
    pub zone_manquante: Option<crate::agents::MissingZone>,
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

    /// Le guide n'a pas pu être rendu.
    #[error(transparent)]
    Agents(#[from] crate::agents::Error),
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

    let metadonnees = metadata::read(&root.join("Cargo.toml"))?;
    let depuis = metadonnees.version.clone();

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

    // `upgrade` est la seule commande qui recrée le fichier : sa mission est d'aligner le
    // projet, et un guide absent est précisément ce qu'elle doit rétablir.
    let zone_manquante = if builder.exists(crate::agents::FICHIER)? {
        let corps = crate::agents::guide(metadonnees.lang, &root)?;
        // L'inventaire porte la version *visée*, non celle que le manifeste porte encore
        // sur le disque : la lire depuis là écrirait dans le guide la version d'avant la
        // mise à niveau, et `rbs doctor` la déclarerait aussitôt périmée.
        let inventaire = crate::agents::inventory_of(
            &metadonnees.features,
            cli,
            metadonnees.database,
            &root,
            metadonnees.lang,
        );

        // Deux appels successifs, et non un chaînage : `builder.replace_zone(..).and_then(
        // |_| builder.replace_zone(..))` prendrait deux emprunts mutables du builder dans
        // une même expression, ce que le compilateur refuse.
        //
        // Une zone absente n'arrête pas la seconde : les deux sont indépendantes, et un
        // guide supprimé n'a aucune raison d'empêcher l'inventaire de suivre la version
        // visée. Seule la première manquante est retenue — c'est celle dont le bloc
        // s'affiche.
        let mut manquante = None;
        for (zone, contenu, version) in [
            (crate::agents::GUIDE, corps, Some(cli)),
            (crate::agents::INVENTORY, inventaire, None),
        ] {
            match builder.replace_zone(crate::agents::FICHIER, zone, &contenu, version) {
                Ok(()) => {}
                Err(plan::Error::ZoneAbsente { zone: absente, .. }) => {
                    manquante.get_or_insert(absente);
                }
                Err(autre) => return Err(autre.into()),
            }
        }

        manquante
    } else {
        // Lu ici seulement : le nom du paquet ne sert qu'au titre du document recréé, et
        // un `[package] name` illisible faisait échouer une mise à niveau qui n'en avait
        // pas besoin.
        let package = metadata::package_name(&root.join("Cargo.toml"))?;
        let document = crate::agents::document(&root, metadonnees.lang, &package, cli)?;
        builder.create(crate::agents::FICHIER, &document)?;
        None
    };

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
        zone_manquante,
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

    /// Une version strictement postérieure à celle du workspace, **dérivée d'elle**.
    ///
    /// Un numéro figé ici finit par être rattrapé : `"1.0.0"` l'a été le jour de sa
    /// publication, et cinq tests ont cessé d'exercer ce qu'ils décrivaient — le projet
    /// neuf n'était plus en retard sur un futur devenu présent.
    fn futur() -> &'static str {
        static FUTUR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            let [majeur, ..] = super::nombres(CLI).expect("la version du CLI se lit");
            format!("{}.0.0", majeur + 1)
        });

        &FUTUR
    }

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
                lang: crate::lang::Lang::Fr,
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

        let planned =
            plan_for_with(&options(&root), futur()).expect("la mise à niveau se planifie");

        assert!(
            !planned.deja_a_jour,
            "un projet en retard a quelque chose à faire"
        );
        assert_eq!(planned.depuis, CLI);
        assert_eq!(planned.vers, futur());

        let rendu = plan::render::plan(&planned.plan);
        assert!(rendu.contains("Cargo.toml"), "{rendu}");
        assert!(rendu.contains("modifié"), "{rendu}");

        plan::application::apply(&planned.plan, false).expect("le plan doit s'appliquer");

        let manifeste = manifeste(&root);
        assert!(
            manifeste.contains(&format!("rbs-core = {{ version = \"{}\"", futur())),
            "le noyau n'a pas suivi :\n{manifeste}"
        );
        assert_eq!(
            metadata::read(&root.join("Cargo.toml"))
                .expect("le manifeste reste lisible")
                .version,
            futur()
        );
    }

    #[test]
    fn a_project_already_on_the_target_has_nothing_to_do_and_writes_nothing() {
        let (_parent, root) = project(None);
        assert!(
            !upgrade(&root, futur()).deja_a_jour,
            "la première mise à niveau doit avoir eu quelque chose à faire"
        );

        let avant = empreinte(&root);
        let planned =
            plan_for_with(&options(&root), futur()).expect("la mise à niveau se planifie");

        assert!(planned.deja_a_jour, "le projet est déjà en {}", futur());
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

        let error = plan_for_with(&options(&root), futur())
            .expect_err("un CLI antérieur au projet doit refuser");

        let message = error.to_string();
        assert!(message.contains("9.9.9"), "{message}");
        assert!(message.contains(futur()), "{message}");
    }

    #[test]
    fn nothing_but_the_manifest_and_the_agents_zones_is_touched() {
        let (_parent, root) = project(None);
        commit(&root);

        // `futur()` diffère de la version que le guide du projet porte déjà : contrairement
        // au cas où le CLI met à niveau vers sa propre version, l'AGENTS.md a ici
        // légitimement quelque chose à changer, en plus du manifeste.
        upgrade(&root, futur());

        assert_eq!(
            git(&root, &["status", "--porcelain"]),
            " M AGENTS.md\n M Cargo.toml\n"
        );
    }

    #[test]
    fn a_core_taken_from_a_local_path_keeps_its_path() {
        let noyau = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rbs-core");
        let (_parent, root) = project(Some(noyau));

        upgrade(&root, futur());

        let manifeste = manifeste(&root);
        assert!(
            manifeste.contains("rbs-core = { path ="),
            "le mode développement a été écrasé :\n{manifeste}"
        );
        assert_eq!(
            metadata::read(&root.join("Cargo.toml"))
                .expect("le manifeste reste lisible")
                .version,
            futur(),
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
            plan_for_with(&options(&root), futur()).expect_err("un working tree sale doit bloquer");

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

    /// Un projet mis à niveau doit recevoir le mode d'emploi de sa nouvelle version :
    /// sinon tout agent qui le lit travaille sur une documentation périmée.
    #[test]
    fn upgrading_rewrites_the_guide_with_the_version_of_the_cli() {
        let (_parent, root) = project(None);
        let agents = root.join("AGENTS.md");
        let vieilli = fs::read_to_string(&agents)
            .expect("AGENTS.md est écrit")
            .replace(
                &crate::agents::opening(crate::agents::GUIDE, Some(crate::agents::VERSION)),
                &crate::agents::opening(crate::agents::GUIDE, Some("0.9.0")),
            );
        fs::write(&agents, vieilli).expect("l'écriture aboutit");

        let planned = plan_for_with(
            &Options {
                directory: root.clone(),
                force: true,
            },
            "2.0.0",
        )
        .expect("le plan doit se calculer");

        let projete = planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == "AGENTS.md")
            .expect("AGENTS.md est visé par le plan");

        assert!(
            projete.after.contains("<!-- rbs:guide 2.0.0 -->"),
            "{}",
            projete.after
        );
        assert!(!projete.after.contains("0.9.0"), "{}", projete.after);
    }

    /// Ce que le développeur écrit hors des zones lui appartient, mise à niveau comprise.
    #[test]
    fn upgrading_keeps_what_the_developer_wrote_outside_the_zones() {
        let (_parent, root) = project(None);
        let agents = root.join("AGENTS.md");
        let augmente = format!(
            "{}\n## Nos conventions à nous\n\nne jamais toucher au module facturation\n",
            fs::read_to_string(&agents).expect("AGENTS.md est écrit")
        );
        fs::write(&agents, augmente).expect("l'écriture aboutit");

        let planned = plan_for_with(
            &Options {
                directory: root.clone(),
                force: true,
            },
            "2.0.0",
        )
        .expect("le plan doit se calculer");

        let projete = planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == "AGENTS.md")
            .expect("AGENTS.md est visé par le plan");

        assert!(
            projete
                .after
                .contains("ne jamais toucher au module facturation"),
            "{}",
            projete.after
        );
    }

    /// `upgrade` a mandat d'aligner le projet : un fichier supprimé se recrée, là où
    /// `add` et `generate` passent leur chemin.
    #[test]
    fn upgrading_recreates_a_deleted_agents_file() {
        let (_parent, root) = project(None);
        fs::remove_file(root.join("AGENTS.md")).expect("le fichier existe");

        let planned = plan_for_with(
            &Options {
                directory: root.clone(),
                force: true,
            },
            "2.0.0",
        )
        .expect("le plan doit se calculer");

        assert!(
            planned
                .plan
                .files()
                .iter()
                .any(|file| file.path == "AGENTS.md"),
            "le plan ne recrée pas AGENTS.md"
        );
    }

    /// Les deux zones sont indépendantes : un guide supprimé n'empêche pas l'inventaire de
    /// suivre la version visée. La boucle rompait sur la première absente, et une zone
    /// retirée en emportait donc une autre, intacte.
    #[test]
    fn a_deleted_guide_does_not_stop_the_inventory_from_being_refreshed() {
        let (_parent, root) = project(None);
        let agents = root.join("AGENTS.md");
        let ampute: String = fs::read_to_string(&agents)
            .expect("AGENTS.md est écrit")
            .lines()
            .filter(|ligne| !ligne.starts_with("<!-- rbs:guide"))
            .filter(|ligne| *ligne != crate::agents::closing(crate::agents::GUIDE))
            .map(|ligne| format!("{ligne}\n"))
            .collect();
        fs::write(&agents, ampute).expect("l'écriture aboutit");

        let planned = plan_for_with(
            &Options {
                directory: root.clone(),
                force: true,
            },
            "2.0.0",
        )
        .expect("le plan doit se calculer");

        assert_eq!(
            planned
                .zone_manquante
                .as_ref()
                .map(|zone| zone.zone.as_str()),
            Some(crate::agents::GUIDE),
            "la zone absente doit rester nommée"
        );

        let projete = planned
            .plan
            .files()
            .iter()
            .find(|file| file.path == "AGENTS.md")
            .expect("l'inventaire reste à réécrire, guide ou non");

        assert!(
            projete.after.contains("- rbs 2.0.0 ·"),
            "l'inventaire n'a pas suivi la version visée :\n{}",
            projete.after
        );
    }

    /// Un projet déjà à jour ne doit rien écrire : c'est ce qui fait dire « rien à faire »
    /// à la commande.
    #[test]
    fn a_project_already_up_to_date_plans_nothing_on_the_agents_file() {
        let (_parent, root) = project(None);

        let planned = plan_for_with(
            &Options {
                directory: root,
                force: true,
            },
            crate::agents::VERSION,
        )
        .expect("le plan doit se calculer");

        assert!(planned.deja_a_jour, "{:?}", planned.plan.files());
    }
}
