mod add;
mod anchors;
mod cli;
mod doctor;
mod dotenv;
mod generate;
mod git;
mod manifest;
mod metadata;
mod migrate;
mod new;
mod plan;
mod prompts;
mod template;
mod templates;
mod ui;

use std::error::Error;
use std::path::PathBuf;

use clap::Parser;

use cli::{Cli, Commands, GenerateCommands, MigrateCommands};

/// Le corps de la commande, appelé à l'identique par les deux binaires livrés.
pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            database_url,
            with,
            core_path,
        } => {
            let resultat = create_project(
                name,
                database_url,
                with,
                core_path,
                cli.template_dir,
                cli.yes,
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
            let (name, fields, complete, force, dry_run) = match command {
                GenerateCommands::Crud {
                    name,
                    fields,
                    force,
                    dry_run,
                } => (name, fields, true, force, dry_run),
                GenerateCommands::Feature {
                    name,
                    force,
                    dry_run,
                } => (name, None, false, force, dry_run),
            };

            if let Err(error) = generate(name, fields, complete, force, dry_run) {
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

fn create_project(
    name: String,
    database_url: Option<String>,
    with: Vec<String>,
    core_path: Option<PathBuf>,
    template_dir: Option<PathBuf>,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    // Un `--with` absent laisse la question ouverte ; un `--with` vide n'existe pas.
    let features = (!with.is_empty()).then_some(with);
    let options = prompts::resolve(Some(name), database_url, features, yes)?;

    let project = new::create(
        &new::Options {
            name: options.name,
            database_url: options.database_url,
            features: options.features,
            core_path,
            template_dir,
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
    ui::info(&format!(
        "\n  cd {name}\n  cargo run          # la base visée est dans .env"
    ));

    Ok(())
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

/// Ce qu'il reste à faire de la main du développeur, une fois la feature posée.
fn suite(feature: &str) -> Option<&'static str> {
    match feature {
        "docker" => Some("docker compose up --build"),
        "ci" => Some("git push : le workflow s'exécute à la prochaine poussée"),
        // Le secret n'est écrit que dans `.env.example` : le `.env` du projet, lui, est
        // hors du fragment, et sans la variable le serveur refuse de démarrer.
        "auth" => {
            Some("recopiez RBS_AUTH__SECRET de .env.example vers votre .env, puis rbs migrate up")
        }
        // Le pool est paresseux : le projet démarre sans Redis, et ne le joint qu'au
        // premier appel au cache.
        "redis" => {
            Some("un Redis doit écouter à l'URL de la section [cache] de config/default.toml")
        }
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
) -> Result<(), generate::command::Error> {
    let feature = name.clone();
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
    })?;

    // Le plan se montre avant toute écriture, `--dry-run` ou non : ce que la commande
    // s'apprête à faire ne doit pas se découvrir après coup.
    println!("{}", plan::render::plan(&planned.plan));

    // Avant le plan, l'avertissement se perdrait au-dessus de sept lignes de fichiers.
    if let Some(avertissement) = &planned.avertissement {
        ui::warn(avertissement);
    }

    if dry_run {
        ui::info("\n  rien n'a été écrit (--dry-run)");
        return Ok(());
    }

    plan::application::apply(&planned.plan, force)?;

    ui::success(&format!(
        "{feature} générée — {}",
        ui::files(planned.files.len())
    ));

    if let Some(migration) = &planned.migration {
        ui::info(&format!(
            "\n  la migration {migration} reste à appliquer avant de lancer le projet"
        ));
    }

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

    #[test]
    fn the_auth_suite_names_the_secret_missing_at_startup() {
        let suite = suite("auth").expect("`auth` doit dire ce qu'il reste à faire");

        // `add auth` n'écrit la variable que dans `.env.example` : le lecteur qui ne la
        // recopie pas obtient un serveur qui refuse de démarrer.
        assert!(
            suite.contains("RBS_AUTH__SECRET") && suite.contains(".env"),
            "la suite ne dit pas où recopier le secret : {suite}"
        );
    }
}
