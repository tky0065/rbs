//! Insère les données de démonstration du projet.
//!
//! `rbs seed` enveloppe ce binaire et refuse de le lancer sous `RBS_ENV=production`.
//! Rien n'oblige à passer par lui : `cargo run --bin seed` fait le même travail.

use std::error::Error;
use std::process::ExitCode;

use sea_orm::{Database, DatabaseConnection};

/// Déclare les seeds et les enchaîne dans l'ordre où ils sont listés.
///
/// Un `mod` non inline ne s'écrit pas dans un bloc : la déclaration des modules et leur
/// enchaînement se font donc d'un seul geste, à hauteur d'item.
macro_rules! seeds {
    ($($module:ident),* $(,)?) => {
        $(mod $module;)*

        /// Applique chaque seed déclaré, dans l'ordre de sa déclaration.
        async fn apply(db: &DatabaseConnection) -> Result<(), Box<dyn Error>> {
            // Une base injoignable se dit ici, et non au milieu du premier seed.
            db.ping().await?;

            $(
                $module::seed(db).await?;
                println!("{} : inséré", stringify!($module));
            )*

            Ok(())
        }
    };
}

seeds! {
    // <rbs:seeds>
    posts,
    // </rbs:seeds>
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let url = std::env::var("RBS_DATABASE__URL")
        .map_err(|_| "RBS_DATABASE__URL n'est pas définie : renseignez-la dans .env")?;
    let db = Database::connect(&url).await?;

    apply(&db).await
}
