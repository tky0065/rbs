mod ancres;
mod cli;
mod dotenv;
mod generate;
mod metadata;
mod migrate;
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

        Commands::Generate { command } => {
            let (nom, fields, complete, force) = match command {
                GenerateCommands::Crud { nom, fields, force } => (nom, fields, true, force),
                GenerateCommands::Feature { nom, force } => (nom, None, false, force),
            };

            if force {
                ui::warn(
                    "`--force` est sans effet : la vérification du working tree n'est pas \
                     encore livrée",
                );
            }

            if let Err(erreur) = generer(nom, fields, complete) {
                ui::error(&erreur.to_string());
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

fn generer(nom: String, fields: Option<String>, complete: bool) -> Result<(), Box<dyn Error>> {
    let feature = nom.clone();
    let generee = generate::commande::executer(&generate::commande::Options {
        nom,
        fields,
        complete,
        repertoire: std::env::current_dir()?,
    })?;

    ui::success(&format!(
        "{feature} générée — {} fichiers",
        generee.fichiers.len()
    ));

    if let Some(migration) = &generee.migration {
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

fn nommer(commande: &Commands) -> &'static str {
    match commande {
        Commands::New { .. } => "new",
        Commands::Add { .. } => "add",
        Commands::Generate { command } => match command {
            GenerateCommands::Crud { .. } => "generate crud",
            GenerateCommands::Feature { .. } => "generate feature",
        },
        Commands::Doctor => "doctor",
        // `new`, `generate` et `migrate` ont leur propre bras : seules les commandes
        // encore absentes passent par ici.
        _ => "commande",
    }
}
