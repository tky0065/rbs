mod add;
mod agents;
mod anchors;
mod cargo;
mod cli;
mod database;
mod dev;
mod doctor;
mod dotenv;
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

        Commands::Add { feature, force } => {
            if let Err(error) = add(feature, force, cli.template_dir) {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Generate { command } => {
            let (name, fields, complete, force, dry_run, has_many) = match command {
                GenerateCommands::Crud {
                    name,
                    fields,
                    force,
                    dry_run,
                    has_many,
                } => (name, fields, true, force, dry_run, has_many),
                GenerateCommands::Feature {
                    name,
                    force,
                    dry_run,
                } => (name, None, false, force, dry_run, Vec::new()),
            };

            if let Err(error) = generate(name, fields, complete, force, dry_run, has_many) {
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

        Commands::Upgrade { force } => {
            if let Err(error) = upgrade(force) {
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
    name: String,
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
    let options = prompts::resolve(
        Some(name),
        database_url,
        database,
        features,
        &disponibles,
        yes,
    )?;

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
            "  + {:<8} {}{migration}",
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

/// Installe une feature dans le projet courant, plan affiché avant écriture.
fn add(feature: String, force: bool, template_dir: Option<PathBuf>) -> Result<(), add::Error> {
    let directory = std::env::current_dir().map_err(|source| add::Error::Acces {
        path: ".".to_string(),
        source,
    })?;
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

    ui::info(&format!("{feature} : {}\n", planned.description));
    println!("{}", plan::render::plan(&planned.plan));

    signaler_zone_manquante(planned.zone_manquante.as_ref());

    plan::application::apply(&planned.plan, force)?;

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
) -> Result<(), generate::command::Error> {
    let feature = name.clone();
    // `--has-many` répare une feature déjà là : rien à générer, donc rien à annoncer sous
    // ce nom-là une fois l'écriture faite.
    let repairing = !has_many.is_empty();
    let directory = std::env::current_dir().map_err(|source| generate::command::Error::Acces {
        path: ".".to_string(),
        source,
    })?;
    let planned = generate::command::plan_for(&generate::command::Options {
        name,
        fields,
        complete,
        directory,
        force,
        has_many,
    })?;

    // Le plan se montre avant toute écriture, `--dry-run` ou non : ce que la commande
    // s'apprête à faire ne doit pas se découvrir après coup.
    println!("{}", plan::render::plan(&planned.plan));

    signaler_zone_manquante(planned.zone_manquante.as_ref());

    // Avant le plan, l'avertissement se perdrait au-dessus de sept lignes de fichiers.
    if let Some(avertissement) = &planned.avertissement {
        ui::warn(avertissement);
    }

    // Un fichier qui n'apparaît pas dans le plan doit se justifier : sans cette ligne,
    // l'absence du seed se découvrirait en cherchant un fichier qui n'a jamais existé.
    if let Some(relation) = &planned.seed_skipped {
        ui::info(&format!(
            "\n  aucun seed pour {feature} : la référence « {relation} » est requise, et un \
             seed ne peut pas deviner vers quelle ligne pointer"
        ));
    }

    if dry_run {
        ui::info("\n  rien n'a été écrit (--dry-run)");
        return Ok(());
    }

    plan::application::apply(&planned.plan, force)?;

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
fn upgrade(force: bool) -> Result<(), upgrade::Error> {
    let directory = std::env::current_dir().map_err(|source| upgrade::Error::Acces {
        path: ".".to_string(),
        source,
    })?;
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

    plan::application::apply(&planned.plan, force)?;

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
    let directory = std::env::current_dir().map_err(|source| seed::Error::Acces {
        path: ".".to_string(),
        source,
    })?;

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
    use super::*;

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
