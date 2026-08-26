mod cli;
mod metadata;
mod new;
mod prompts;
mod template;
mod templates;
mod ui;

use std::error::Error;
use std::path::PathBuf;

use clap::Parser;

use cli::{Cli, Commands, GenerateCommands, MigrateCommands};

/// Distinct de 1 pour qu'un script sache différencier « pas encore là » d'un échec réel.
const EXIT_NON_IMPLEMENTE: i32 = 2;

fn main() {
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

        commande => {
            ui::error(&format!(
                "`rbs {}` n'est pas encore implémentée.",
                nommer(&commande)
            ));
            std::process::exit(EXIT_NON_IMPLEMENTE);
        }
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

    ui::success(&format!("{nom} créé — {} fichiers", projet.fichiers));
    if !projet.depot_git {
        ui::warn("`git init` n'a pas abouti : le projet est complet, mais sans dépôt");
    }
    ui::info(&format!(
        "\n  cd {nom}\n  cargo run          # la base visée est dans .env"
    ));

    Ok(())
}

fn nommer(commande: &Commands) -> &'static str {
    match commande {
        Commands::New { .. } => "new",
        Commands::Add { .. } => "add",
        Commands::Generate { command } => match command {
            GenerateCommands::Crud { .. } => "generate crud",
            GenerateCommands::Feature { .. } => "generate feature",
        },
        Commands::Migrate { command } => match command {
            MigrateCommands::Up => "migrate up",
            MigrateCommands::Down => "migrate down",
            MigrateCommands::Status => "migrate status",
            MigrateCommands::New { .. } => "migrate new",
        },
        Commands::Doctor => "doctor",
    }
}
