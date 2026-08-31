mod add;
mod agents;
mod anchors;
mod cargo;
mod cli;
mod database;
mod dev;
mod doctor;
mod dotenv;
mod errors;
#[cfg(test)]
mod fixtures;
mod generate;
mod git;
mod lang;
mod manifest;
mod metadata;
mod migrate;
mod new;
mod notes;
mod plan;
mod prompts;
mod secret;
mod seed;
mod template;
mod templates;
mod ui;
mod upgrade;
mod url;

use std::error::Error;
use std::path::PathBuf;

use clap::Parser;

use cli::{Cli, Commands, GenerateCommands, MigrateCommands};
use database::Database;

/// Le corps de la commande, appelé à l'identique par les deux binaires livrés.
pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            database_url,
            database,
            with,
            core_path,
            lang,
        } => {
            let resultat = create_project(
                name,
                database_url,
                database,
                with,
                core_path,
                cli.template_dir,
                cli.yes,
                lang,
            );

            if let Err(error) = resultat {
                ui::error(&error.to_string());
                std::process::exit(1);
            }
        }

        Commands::Add {
            feature,
            force,
            dry_run,
        } => {
            if let Err(error) = add(feature, force, dry_run, cli.template_dir) {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Generate { command } => {
            let (name, fields, complete, force, dry_run, has_many, role) = match command {
                GenerateCommands::Crud {
                    name,
                    fields,
                    force,
                    dry_run,
                    has_many,
                    role,
                } => (name, fields, true, force, dry_run, has_many, role),
                GenerateCommands::Feature {
                    name,
                    force,
                    dry_run,
                } => (name, None, false, force, dry_run, Vec::new(), None),
            };

            if let Err(error) = generate(name, fields, complete, force, dry_run, has_many, role) {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Migrate { command } => {
            let action = match command {
                MigrateCommands::Up => migrate::Action::Up,
                MigrateCommands::Down => migrate::Action::Down,
                MigrateCommands::Status => migrate::Action::Status,
                MigrateCommands::New { name } => migrate::Action::Fresh(name),
            };

            if let Err(error) = migrate(action) {
                ui::error(&error.to_string());
                std::process::exit(1);
            }
        }

        Commands::Seed { force } => {
            if let Err(error) = seed(force) {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Dev => {
            let resultat = std::env::current_dir()
                .map_err(dev::Error::Cwd)
                .and_then(|directory| dev::run(&directory));

            if let Err(error) = resultat {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Upgrade { force, dry_run } => {
            if let Err(error) = upgrade(force, dry_run) {
                ui::error(&error.to_string());
                std::process::exit(1);
            }
        }

        Commands::Doctor => match diagnose() {
            Ok(true) => {}
            // Un diagnostic qui trouve quelque chose n'est pas un échec de la commande,
            // mais un script doit pouvoir le distinguer d'un projet sain.
            Ok(false) => std::process::exit(1),
            Err(error) => {
                ui::error(&error.to_string());
                std::process::exit(1);
            }
        },
    }
}

// Chaque paramètre est un flag de `rbs new` répercuté tel quel ; les regrouper en
// structure déplacerait la question sans la résoudre, `new::Options` la portant déjà
// deux lignes plus bas.
#[allow(clippy::too_many_arguments)]
fn create_project(
    name: Option<String>,
    database_url: Option<String>,
    database: Database,
    with: Vec<String>,
    core_path: Option<PathBuf>,
    template_dir: Option<PathBuf>,
    yes: bool,
    lang: Option<lang::Lang>,
) -> Result<(), Box<dyn Error>> {
    // Un `--with` absent laisse la question ouverte ; un `--with` vide n'existe pas.
    let features = (!with.is_empty()).then_some(with);
    let disponibles = templates::feature_names(template_dir.as_deref());
    let options = prompts::resolve(name, database_url, database, features, &disponibles, yes)?;

    let project = new::create(
        &new::Options {
            name: options.name,
            database_url: options.database_url,
            database,
            features: options.features,
            core_path,
            template_dir,
            lang: lang.unwrap_or_else(|| lang::Lang::from_locale(locale().as_deref())),
        },
        &std::env::current_dir()?,
    )?;

    let name = project
        .root
        .file_name()
        .unwrap_or(project.root.as_os_str())
        .to_string_lossy();

    ui::success(&format!("{name} créé — {}", ui::files(project.files)));
    if !project.depot_git {
        ui::warn("`git init` n'a pas abouti : le projet est complet, mais sans dépôt");
    }
    for pose in &project.installed {
        let migration = if pose.migration { ", 1 migration" } else { "" };
        ui::info(&format!(
            "  + {:<10} {}{migration}",
            pose.name,
            ui::files(pose.files)
        ));
    }
    // `add` affiche ce conseil pour chaque feature qu'il installe ; `new` l'avalait,
    // laissant par exemple `--with auth` démarrer un projet où `RBS_AUTH__SECRET` manque
    // au `.env` sans que rien ne l'ait annoncé.
    for suite in suites_installees(&project.installed) {
        ui::info(&format!("\n  {suite}"));
    }
    let compose = project.root.join("docker-compose.yml").exists();
    let demarrage = if compose {
        "\n  docker compose up -d   # la base du .env, montée\n  cargo run              # ou `rbs dev`, qui enchaîne les deux"
    } else {
        "\n  cargo run          # la base visée est dans .env"
    };
    ui::info(&format!("\n  cd {name}{demarrage}"));

    Ok(())
}

/// La locale de l'environnement, `LC_ALL` d'abord.
fn locale() -> Option<String> {
    locale_from(
        std::env::var("LC_ALL").ok().as_deref(),
        std::env::var("LANG").ok().as_deref(),
    )
}

/// La même, les deux variables passées en paramètre.
///
/// Séparée pour que la précédence soit exerçable sans écrire dans l'environnement du
/// processus de test, que les autres tests partagent. Le filtre porte sur chaque valeur
/// avant que la précédence ne choisisse : une variable posée mais vide n'est pas
/// autoritaire en POSIX, et `LC_ALL=""` se rencontre en intégration continue — un filtre
/// posé après aurait figé le choix sur cette valeur vide plutôt que de laisser `LANG`
/// trancher.
fn locale_from(lc_all: Option<&str>, lang: Option<&str>) -> Option<String> {
    [lc_all, lang]
        .into_iter()
        .flatten()
        .find(|locale| !locale.is_empty())
        .map(str::to_owned)
}

/// Signale la zone de l'`AGENTS.md` qu'une commande n'a pas pu réécrire, et donne le bloc
/// à recoller.
///
/// Une seule fonction pour les trois commandes qui touchent au fichier : `rbs doctor`
/// renvoie vers elles pour rétablir une zone, et une des trois qui se tairait laisserait
/// l'utilisateur tourner en rond entre les deux commandes.
fn signaler_zone_manquante(zone: Option<&agents::MissingZone>) {
    if let Some(zone) = zone {
        ui::warn(&format!("{zone} — collez ce bloc pour la rétablir :"));
        ui::info(&format!("\n{}", zone.block()));
    }
}

/// Applique le plan, ou dit que `--dry-run` l'a laissé sur le papier.
///
/// Rend `false` quand rien n'a été écrit : l'appelant sort alors sans annoncer une
/// écriture qui n'a pas eu lieu.
fn appliquer(
    plan: &plan::Plan,
    force: bool,
    dry_run: bool,
) -> Result<bool, plan::application::Error> {
    if dry_run {
        ui::info("\n  rien n'a été écrit (--dry-run)");
        return Ok(false);
    }

    plan::application::apply(plan, force)?;

    Ok(true)
}

/// Installe une feature dans le projet courant, plan affiché avant écriture.
fn add(
    feature: String,
    force: bool,
    dry_run: bool,
    template_dir: Option<PathBuf>,
) -> Result<(), add::Error> {
    let directory = std::env::current_dir()
        .map_err(|source| crate::errors::Acces::new(std::path::Path::new("."), source))?;

    add_in(directory, feature, force, dry_run, template_dir)
}

/// La même, le projet visé donné en paramètre.
///
/// Ce qu'un `--dry-run` promet — ne rien écrire — ne se prouve qu'en comparant le projet
/// avant et après, et un test ne peut pas déplacer le répertoire courant qu'il partage
/// avec tous les autres.
fn add_in(
    directory: PathBuf,
    feature: String,
    force: bool,
    dry_run: bool,
    template_dir: Option<PathBuf>,
) -> Result<(), add::Error> {
    let planned = add::plan_for(&add::Options {
        feature: feature.clone(),
        directory,
        force,
        template_dir,
    })?;

    if planned.deja_installee {
        ui::success(&format!("{feature} est déjà installée — rien à faire"));
        return Ok(());
    }

    ui::info(&format!("{feature} : {}", planned.description));

    // Annoncé avant le plan et non après : ce que l'utilisateur n'a pas nommé, il doit le
    // lire au moment où il décide d'appliquer, pas une fois les fichiers écrits.
    if !planned.entrainees.is_empty() {
        ui::info(&format!(
            "{feature} exige {} : posée avec elle",
            planned.entrainees.join(", ")
        ));
    }

    println!();
    println!("{}", plan::render::plan(&planned.plan));

    signaler_zone_manquante(planned.zone_manquante.as_ref());

    if !appliquer(&planned.plan, force, dry_run)? {
        return Ok(());
    }

    ui::success(&format!(
        "{feature} installée — {}",
        ui::files(planned.files.len())
    ));

    if let Some(suite) = suite(&feature) {
        ui::info(&format!("\n  {suite}"));
    }

    Ok(())
}

/// Les conseils de suite des features que `new` vient d'installer, dans l'ordre
/// d'installation.
///
/// `rbs new --with` passe par le même `install` qu'`rbs add` (`new::install`), mais
/// n'appelait jamais `suite()` : le développeur perdait le seul avertissement qui compte
/// pour `auth`, dont le projet ne démarre pas sans lui.
fn suites_installees(installed: &[new::InstalledFeature]) -> Vec<&'static str> {
    installed
        .iter()
        .filter_map(|pose| suite(&pose.name))
        .collect()
}

/// Ce qu'il reste à faire de la main du développeur, une fois la feature posée.
fn suite(feature: &str) -> Option<&'static str> {
    match feature {
        // `migrate` et `api` portent `profiles: ["app"]` : sans ce flag, `docker compose
        // up` ne bâtit ni ne démarre ni l'un ni l'autre — seul `db` reste dans le
        // périmètre par défaut, celui que `rbs dev` monte.
        "docker" => Some("docker compose --profile app up --build"),
        "ci" => Some("git push : le workflow s'exécute à la prochaine poussée"),
        // Le secret est tiré à l'installation et déposé dans le `.env` : il ne reste que
        // la migration, sans quoi les tables d'authentification manqueraient au premier
        // login.
        "auth" => Some("rbs migrate up"),
        // Le fragment dépose lui-même un service `redis` dans le compose du projet : le
        // pool est paresseux, mais rien à fournir séparément une fois ce service monté.
        "redis" => Some(
            "le compose du projet porte déjà un service redis — docker compose up -d le \
             démarre ; sans compose, faites écouter un Redis à l'URL de [cache] de \
             config/default.toml",
        ),
        // Le défaut vise un Mailpit local : sans ce rappel, le premier envoi en
        // production partirait vers `localhost:1025`.
        "mail" => Some("réglez [mail] dans config/default.toml — un SMTP local par défaut"),
        // Le backend par défaut écrit sous une racine du dépôt, que rien n'ignore encore.
        "storage" => Some(
            "les objets vont sous ./storage : ajoutez-le à .gitignore, ou passez \
             storage.backend à \"s3\" et recopiez les RBS_STORAGE__* de .env.example",
        ),
        // La table n'existe pas encore, et le worker démarre avec l'API : sans la
        // migration, chaque tour de boucle échoue sur une relation absente.
        "jobs" => Some("rbs migrate up, puis inscrivez vos jobs dans src/jobs/mod.rs"),
        // La liste est vide à l'installation : sans ce rappel, le développeur croirait
        // avoir monté du CORS alors qu'aucune origine n'est encore autorisée.
        "cors" => Some(
            "énumérez vos origines dans [cors] de config/default.toml — la liste est vide, \
             donc aucune requête d'origine croisée ne passe",
        ),
        // Le compteur ne voit un client que si le serveur lui donne son adresse : un
        // projet derrière un proxy compte tout le monde ensemble tant que le drapeau
        // n'est pas levé.
        "rate-limit" => Some(
            "derrière un reverse proxy, passez rate_limit.trust_forwarded_for à true — \
             sinon tous les clients partagent l'adresse du proxy",
        ),
        _ => None,
    }
}

fn generate(
    name: String,
    fields: Option<String>,
    complete: bool,
    force: bool,
    dry_run: bool,
    has_many: Vec<String>,
    role: Option<String>,
) -> Result<(), generate::command::Error> {
    let feature = name.clone();
    // `--has-many` répare une feature déjà là : rien à générer, donc rien à annoncer sous
    // ce nom-là une fois l'écriture faite.
    let repairing = !has_many.is_empty();
    let directory = std::env::current_dir()
        .map_err(|source| crate::errors::Acces::new(std::path::Path::new("."), source))?;
    let planned = generate::command::plan_for(&generate::command::Options {
        name,
        fields,
        complete,
        directory,
        force,
        has_many,
        role,
    })?;

    // Le plan se montre avant toute écriture, `--dry-run` ou non : ce que la commande
    // s'apprête à faire ne doit pas se découvrir après coup.
    println!("{}", plan::render::plan(&planned.plan));

    signaler_zone_manquante(planned.zone_manquante.as_ref());

    // Avant le plan, l'avertissement se perdrait au-dessus de sept lignes de fichiers.
    if let Some(avertissement) = &planned.avertissement {
        ui::warn(avertissement);
    }

    // Ce qui n'apparaît pas dans le plan doit se justifier : sans cette ligne, l'absence
    // du seed se découvrirait en cherchant un fichier qui n'a jamais existé, et celle des
    // scénarios de création en lisant les tests.
    if let Some(relation) = &planned.required_reference {
        ui::info(&format!(
            "\n  la référence « {relation} » est requise : ni le seed de {feature} ni ses \
             scénarios de création ne peuvent deviner vers quelle ligne pointer — le seed \
             n'est pas engendré, et les tests s'arrêtent aux cas qui ne créent rien"
        ));
    }

    if !appliquer(&planned.plan, force, dry_run)? {
        return Ok(());
    }

    if repairing {
        ui::success(&format!("{feature} : côté inverse écrit"));
    } else {
        ui::success(&format!(
            "{feature} générée — {}",
            ui::files(planned.files.len())
        ));
    }

    if let Some(migration) = &planned.migration {
        ui::info(&format!(
            "\n  la migration {migration} reste à appliquer avant de lancer le projet"
        ));
    }

    Ok(())
}

/// Aligne le manifeste du projet courant sur la version du CLI, plan affiché avant
/// écriture.
fn upgrade(force: bool, dry_run: bool) -> Result<(), upgrade::Error> {
    let directory = std::env::current_dir()
        .map_err(|source| crate::errors::Acces::new(std::path::Path::new("."), source))?;

    upgrade_in(directory, force, dry_run)
}

/// La même, le projet visé donné en paramètre — pour la raison dite sur `add_in`.
fn upgrade_in(directory: PathBuf, force: bool, dry_run: bool) -> Result<(), upgrade::Error> {
    let planned = upgrade::plan_for(&upgrade::Options { directory, force })?;

    if planned.deja_a_jour {
        ui::success(&format!(
            "le projet est déjà en rbs {} — rien à faire",
            planned.vers
        ));
        // Le retour anticipé avalait ce bloc : un projet par ailleurs à jour dont une zone
        // a disparu n'a rien à aligner, et c'est exactement le cas où `rbs doctor` renvoie
        // ici. Sans cette ligne, les deux commandes se renvoyaient l'une à l'autre.
        signaler_zone_manquante(planned.zone_manquante.as_ref());
        return Ok(());
    }

    ui::info(&format!("rbs {} → {}\n", planned.depuis, planned.vers));
    println!("{}", plan::render::plan(&planned.plan));

    signaler_zone_manquante(planned.zone_manquante.as_ref());

    if !appliquer(&planned.plan, force, dry_run)? {
        return Ok(());
    }

    ui::success(&format!("manifeste aligné sur rbs {}", planned.vers));

    let notes = notes::traversees(&planned.depuis, &planned.vers);
    if notes.is_empty() {
        // Toutes les versions ne rompent pas quelque chose : un saut sans note n'est pas
        // un catalogue en défaut, et ne doit pas se lire comme un échec.
        ui::info(&format!(
            "\n  aucune note de migration pour rbs {} → {}",
            planned.depuis, planned.vers
        ));
    } else {
        for note in notes {
            println!("\n{}", note.trim_end());
        }
    }

    // Le manifeste ne fait qu'énoncer la version voulue : tant que le lock n'a pas suivi,
    // le projet compile encore contre l'ancien noyau.
    ui::info("\n  cargo update -p rbs-core, puis cargo test");

    Ok(())
}

fn migrate(action: migrate::Action) -> Result<(), Box<dyn Error>> {
    match migrate::run(action, &std::env::current_dir()?)? {
        migrate::Output::Appliquees => ui::success("migrations appliquées"),
        migrate::Output::Annulee => ui::success("dernière migration annulée"),
        migrate::Output::Inventaire(inventaire) => println!("{inventaire}"),
        migrate::Output::Creee(fresh) => {
            ui::success(&format!("{} créée", fresh.file));
            ui::info("\n  décrivez le changement de schéma, puis `rbs migrate up`");
        }
    }

    Ok(())
}

/// Insère les données de démonstration du projet courant.
fn seed(force: bool) -> Result<(), seed::Error> {
    let directory = std::env::current_dir()
        .map_err(|source| crate::errors::Acces::new(std::path::Path::new("."), source))?;

    match seed::run(&seed::Options { directory, force })? {
        seed::Output::Insere => ui::success("seeds insérés"),
        seed::Output::Rien => ui::success("aucun seed déclaré — rien à insérer"),
    }

    Ok(())
}

/// Rend le rapport et dit si le projet est sain.
fn diagnose() -> Result<bool, Box<dyn Error>> {
    let report = doctor::run(&std::env::current_dir()?)?;

    println!("{}", doctor::render::report(&report));

    if report.succeeded() {
        ui::success("le projet est sain");
    } else {
        ui::warn("le projet demande votre attention");
    }

    Ok(report.succeeded())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::fixtures::project as projet;

    /// Chemin et octets de chaque fichier du projet, triés : deux empreintes égales
    /// valent projets identiques.
    ///
    /// `.git` reste dehors — lire l'état du working tree rafraîchit l'index, et cette
    /// écriture-là n'est pas celle que `--dry-run` promet d'éviter.
    fn empreinte(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut vus = BTreeMap::new();
        let mut a_parcourir = vec![root.to_path_buf()];

        while let Some(directory) = a_parcourir.pop() {
            for entree in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = entree.expect("l'entrée se lit").path();

                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }

                if path.is_dir() {
                    a_parcourir.push(path);
                    continue;
                }

                let relatif = path
                    .strip_prefix(root)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();
                vus.insert(relatif, fs::read(&path).expect("le fichier se lit"));
            }
        }

        vus
    }

    /// Les fichiers dont le contenu a bougé, apparu ou disparu.
    ///
    /// Comparer les deux empreintes directement dirait « elles diffèrent » en déversant
    /// les octets des cinquante fichiers du projet ; ce qui aide est la liste courte de
    /// ceux qui ont bougé.
    fn ecarts(
        avant: &BTreeMap<PathBuf, Vec<u8>>,
        apres: &BTreeMap<PathBuf, Vec<u8>>,
    ) -> Vec<PathBuf> {
        avant
            .keys()
            .chain(apres.keys())
            .filter(|path| avant.get(*path) != apres.get(*path))
            .cloned()
            .collect()
    }

    /// La seule promesse de `--dry-run` est que le projet ne bouge pas : elle se vérifie
    /// à l'octet près, et la même commande sans le flag prouve que le test n'est pas
    /// vide de sens.
    #[test]
    fn add_dry_run_leaves_the_project_untouched_while_the_real_run_writes() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        add_in(root.clone(), "cors".to_string(), false, true, None)
            .expect("le plan doit se calculer");

        assert_eq!(
            ecarts(&avant, &empreinte(&root)),
            Vec::<PathBuf>::new(),
            "`--dry-run` a écrit dans le projet"
        );

        add_in(root.clone(), "cors".to_string(), false, false, None)
            .expect("l'installation doit aboutir");

        assert!(
            !ecarts(&avant, &empreinte(&root)).is_empty(),
            "sans `--dry-run`, la feature n'a rien posé : le test ne prouve rien"
        );
    }

    /// Le guide supprimé est ce qu'`upgrade` a toujours à rétablir, même sur un projet
    /// par ailleurs à jour : sans lui, le plan serait vide et le test ne prouverait rien.
    #[test]
    fn upgrade_dry_run_leaves_the_project_untouched_while_the_real_run_writes() {
        let (_parent, root) = projet();
        let guide = root.join(agents::FICHIER);
        fs::remove_file(&guide).expect("le guide est là");
        let avant = empreinte(&root);

        upgrade_in(root.clone(), false, true).expect("la mise à niveau doit se planifier");

        assert_eq!(
            ecarts(&avant, &empreinte(&root)),
            Vec::<PathBuf>::new(),
            "`--dry-run` a écrit dans le projet"
        );
        assert!(!guide.exists(), "`--dry-run` a recréé le guide");

        upgrade_in(root.clone(), false, false).expect("la mise à niveau doit aboutir");

        assert!(
            guide.exists(),
            "le guide n'a pas été rétabli : le test ne prouve rien"
        );
    }

    #[test]
    fn every_installable_feature_says_what_is_left_to_do() {
        // `auth` est celle dont l'étape suivante compte le plus — sans le secret, le
        // projet ne démarre pas — et fut la seule à n'en afficher aucune.
        let installables = templates::Source::feature(None, "_aucune_feature_de_ce_nom_")
            .expect_err("ce nom ne doit désigner aucun fragment")
            .known;

        for feature in installables.split(", ") {
            assert!(
                suite(feature).is_some(),
                "`{feature}` s'installe sans dire ce qu'il reste à faire"
            );
        }
    }

    /// `rbs new --with auth` pose la feature mais avalait le conseil qu'`add auth` aurait
    /// affiché : sans lui, la migration reste à lancer et le premier login échoue sur des
    /// tables absentes, sans que rien ne l'ait dit avant.
    #[test]
    fn new_names_the_suite_of_each_installed_feature() {
        let installed = vec![
            new::InstalledFeature {
                name: "auth".to_string(),
                files: 9,
                migration: true,
            },
            new::InstalledFeature {
                name: "redis".to_string(),
                files: 3,
                migration: false,
            },
        ];

        let suites = suites_installees(&installed);

        assert_eq!(
            suites,
            vec![suite("auth").unwrap(), suite("redis").unwrap()]
        );
    }

    /// `ci` n'a rien à ajouter au-delà de son propre conseil ; une feature sans suite ne
    /// doit pas laisser de ligne vide dans la liste.
    #[test]
    fn new_skips_features_without_a_suite() {
        let installed = vec![new::InstalledFeature {
            name: "storage".to_string(),
            files: 4,
            migration: false,
        }];

        assert_eq!(
            suites_installees(&installed),
            vec![suite("storage").unwrap()]
        );
    }

    /// `migrate` et `api` portent `profiles: ["app"]` : sans `--profile app`,
    /// `docker compose up --build` ne bâtit ni ne démarre rien de ce que la feature vient
    /// de poser, mesuré par `docker compose config --services` qui ne rend que `db`.
    #[test]
    fn the_docker_suite_names_the_app_profile() {
        let suite = suite("docker").expect("`docker` doit dire ce qu'il reste à faire");

        assert!(
            suite.contains("--profile app"),
            "la suite ne démarre pas les services du profil app : {suite}"
        );
    }

    /// `rbs add redis` dépose déjà le service `redis` dans le compose du projet : la
    /// suite ne doit pas laisser croire qu'un Redis reste à fournir séparément.
    #[test]
    fn the_redis_suite_names_the_compose_service_already_inserted() {
        let suite = suite("redis").expect("`redis` doit dire ce qu'il reste à faire");

        assert!(
            suite.contains("docker compose"),
            "la suite ne mentionne pas le compose où le service vient d'être inséré : {suite}"
        );
    }

    /// `add auth` dépose désormais le secret lui-même : renvoyer le lecteur vers
    /// `.env.example` lui ferait recopier un placeholder par-dessus sa propre valeur.
    #[test]
    fn the_auth_suite_names_the_migration_and_no_longer_the_secret() {
        let suite = suite("auth").expect("`auth` doit dire ce qu'il reste à faire");

        assert!(
            suite.contains("rbs migrate up"),
            "la suite ne renvoie pas vers la migration : {suite}"
        );
        assert!(
            !suite.contains("RBS_AUTH__SECRET"),
            "la suite demande encore de recopier un secret déjà écrit : {suite}"
        );
    }

    /// `LC_ALL` l'emporte sur `LANG`, comme partout ailleurs sous POSIX.
    #[test]
    fn lc_all_wins_over_lang() {
        assert_eq!(
            locale_from(Some("en_US.UTF-8"), Some("fr_FR.UTF-8")),
            Some("en_US.UTF-8".to_string())
        );
    }

    #[test]
    fn lang_is_read_when_lc_all_is_absent() {
        assert_eq!(
            locale_from(None, Some("fr_FR.UTF-8")),
            Some("fr_FR.UTF-8".to_string())
        );
    }

    #[test]
    fn an_environment_without_locale_gives_nothing() {
        assert_eq!(locale_from(None, None), None);
    }

    /// Une variable posée mais vide n'est pas autoritaire : POSIX veut qu'elle laisse la
    /// main à la suivante, et certains environnements d'intégration posent `LC_ALL=""`.
    #[test]
    fn an_empty_lc_all_gives_way_to_lang() {
        assert_eq!(
            locale_from(Some(""), Some("en_US.UTF-8")),
            Some("en_US.UTF-8".to_string())
        );
    }
}
