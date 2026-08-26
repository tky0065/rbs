mod cli;
mod ui;

use clap::Parser;

use cli::{Cli, Commands, GenerateCommands, MigrateCommands};

/// Distinct de 1 pour qu'un script sache différencier « pas encore là » d'un échec réel.
const EXIT_NON_IMPLEMENTE: i32 = 2;

fn main() {
    let cli = Cli::parse();

    let commande = match cli.command {
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
    };

    ui::error(&format!("`rbs {commande}` n'est pas encore implémentée."));
    std::process::exit(EXIT_NON_IMPLEMENTE);
}
