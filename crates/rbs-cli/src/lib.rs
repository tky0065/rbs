mod add;
mod ancres;
mod cli;
mod doctor;
mod dotenv;
mod generate;
mod git;
mod manifeste;
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
pub fn executer() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            nom,
            database_url,
            with,
            core_path,
        } => {
            let resultat = creer_projet(
                nom,
                database_url,
                with,
                core_path,
                cli.template_dir,
                cli.yes,
            );

            if let Err(erreur) = resultat {
                ui::error(&erreur.to_string());
                std::process::exit(1);
            }
        }

        Commands::Add { feature, force } => {
            if let Err(erreur) = ajouter(feature, force, cli.template_dir) {
                ui::error(&erreur.to_string());
                std::process::exit(1);
            }
        }

        Commands::Generate { command } => {
            let (nom, fields, complete, force, dry_run) = match command {
                GenerateCommands::Crud {
                    nom,
                    fields,
                    force,
                    dry_run,
                } => (nom, fields, true, force, dry_run),
                GenerateCommands::Feature {
                    nom,
                    force,
                    dry_run,
                } => (nom, None, false, force, dry_run),
            };

            if let Err(erreur) = generer(nom, fields, complete, force, dry_run) {
                ui::error(&erreur.to_string());
                if let Some(remede) = erreur.remede() {
                    ui::info(&format!("\n{remede}"));
                }
                std::process::exit(1);
            }
        }

        Commands::Migrate { command } => {
            let action = match command {
                MigrateCommands::Up => migrate::Action::Up,
                MigrateCommands::Down => migrate::Action::Down,
                MigrateCommands::Status => migrate::Action::Status,
                MigrateCommands::New { nom } => migrate::Action::Nouvelle(nom),
            };

            if let Err(erreur) = migrer(action) {
                ui::error(&erreur.to_string());
                std::process::exit(1);
            }
        }

        Commands::Doctor => match diagnostiquer() {
            Ok(true) => {}
            // Un diagnostic qui trouve quelque chose n'est pas un échec de la commande,
            // mais un script doit pouvoir le distinguer d'un projet sain.
            Ok(false) => std::process::exit(1),
            Err(erreur) => {
                ui::error(&erreur.to_string());
                std::process::exit(1);
            }
        },
    }
}

fn creer_projet(
    nom: String,
    database_url: Option<String>,
    with: Vec<String>,
    core_path: Option<PathBuf>,
    template_dir: Option<PathBuf>,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    // Un `--with` absent laisse la question ouverte ; un `--with` vide n'existe pas.
    let features = (!with.is_empty()).then_some(with);
    let options = prompts::resoudre(Some(nom), database_url, features, yes)?;

    let projet = new::creer(
        &new::Options {
            nom: options.nom,
            database_url: options.database_url,
            features: options.features,
            core_path,
            template_dir,
        },
        &std::env::current_dir()?,
    )?;

    let nom = projet
        .racine
        .file_name()
        .unwrap_or(projet.racine.as_os_str())
        .to_string_lossy();

    ui::success(&format!("{nom} créé — {}", ui::fichiers(projet.fichiers)));
    if !projet.depot_git {
        ui::warn("`git init` n'a pas abouti : le projet est complet, mais sans dépôt");
    }
    ui::info(&format!(
        "\n  cd {nom}\n  cargo run          # la base visée est dans .env"
    ));

    Ok(())
}

/// Installe une feature dans le projet courant, plan affiché avant écriture.
fn ajouter(feature: String, force: bool, template_dir: Option<PathBuf>) -> Result<(), add::Erreur> {
    let repertoire = std::env::current_dir().map_err(|source| add::Erreur::Acces {
        chemin: ".".to_string(),
        source,
    })?;
    let planifiee = add::planifier(&add::Options {
        feature: feature.clone(),
        repertoire,
        force,
        template_dir,
    })?;

    println!("{}", plan::rendu::plan(&planifiee.plan));

    plan::application::appliquer(&planifiee.plan, force)?;

    ui::success(&format!(
        "{feature} installée — {}",
        ui::fichiers(planifiee.fichiers.len())
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
        _ => None,
    }
}

fn generer(
    nom: String,
    fields: Option<String>,
    complete: bool,
    force: bool,
    dry_run: bool,
) -> Result<(), generate::commande::Erreur> {
    let feature = nom.clone();
    let repertoire =
        std::env::current_dir().map_err(|source| generate::commande::Erreur::Acces {
            chemin: ".".to_string(),
            source,
        })?;
    let planifiee = generate::commande::planifier(&generate::commande::Options {
        nom,
        fields,
        complete,
        repertoire,
        force,
    })?;

    // Le plan se montre avant toute écriture, `--dry-run` ou non : ce que la commande
    // s'apprête à faire ne doit pas se découvrir après coup.
    println!("{}", plan::rendu::plan(&planifiee.plan));

    if dry_run {
        ui::info("\n  rien n'a été écrit (--dry-run)");
        return Ok(());
    }

    plan::application::appliquer(&planifiee.plan, force)?;

    ui::success(&format!(
        "{feature} générée — {}",
        ui::fichiers(planifiee.fichiers.len())
    ));

    if let Some(migration) = &planifiee.migration {
        ui::info(&format!(
            "\n  la migration {migration} reste à appliquer avant de lancer le projet"
        ));
    }

    Ok(())
}

fn migrer(action: migrate::Action) -> Result<(), Box<dyn Error>> {
    match migrate::executer(action, &std::env::current_dir()?)? {
        migrate::Sortie::Appliquees => ui::success("migrations appliquées"),
        migrate::Sortie::Annulee => ui::success("dernière migration annulée"),
        migrate::Sortie::Inventaire(inventaire) => println!("{inventaire}"),
        migrate::Sortie::Creee(nouvelle) => {
            ui::success(&format!("{} créée", nouvelle.fichier));
            ui::info("\n  décrivez le changement de schéma, puis `rbs migrate up`");
        }
    }

    Ok(())
}

/// Rend le rapport et dit si le projet est sain.
fn diagnostiquer() -> Result<bool, Box<dyn Error>> {
    let rapport = doctor::executer(&std::env::current_dir()?)?;

    println!("{}", doctor::rendu::rapport(&rapport));

    if rapport.reussi() {
        ui::success("le projet est sain");
    } else {
        ui::warn("le projet demande votre attention");
    }

    Ok(rapport.reussi())
}
