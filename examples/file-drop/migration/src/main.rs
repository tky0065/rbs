//! Applique, annule et inventorie les migrations du projet.
//!
//! `rbs migrate` enveloppe ce binaire : il transmet `DATABASE_URL` et met l'inventaire
//! en forme. Rien n'oblige à passer par lui — `cargo run -p migration -- status` rend le
//! même état, une migration par ligne.

use std::error::Error;
use std::process::ExitCode;

use migration::{Migrator, MigratorTrait};
use sea_orm_migration::MigrationStatus;
use sea_orm_migration::sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};

#[tokio::main]
async fn main() -> ExitCode {
    let commande = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "status".to_string());

    match run(&commande).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(commande: &str) -> Result<(), Box<dyn Error>> {
    let url = std::env::var("RBS_DATABASE__URL")
        .map_err(|_| "RBS_DATABASE__URL n'est pas définie : renseignez-la dans .env")?;

    let db = Database::connect(&url).await?;

    match commande {
        "up" => Migrator::up(&db, None).await?,
        // Annuler tout d'un coup se demande explicitement, migration par migration.
        "down" => Migrator::down(&db, Some(1)).await?,
        "status" => {
            for migration in Migrator::get_migration_with_status(&db).await? {
                let etat = match migration.status() {
                    MigrationStatus::Applied => "applied",
                    MigrationStatus::Pending => "pending",
                };
                println!("{etat}\t{}", migration.name());
            }
        }
        // Chaque moteur dit sa version à sa façon, et aucun ne comprend la requête des
        // autres. `rbs doctor` lit cette ligne : il sait quel moteur il interroge, et
        // l'interprète en conséquence.
        "version" => {
            let backend = db.get_database_backend();
            let requete = match backend {
                DatabaseBackend::Postgres => "SHOW server_version_num",
                DatabaseBackend::MySql => "SELECT VERSION()",
                DatabaseBackend::Sqlite => "SELECT sqlite_version()",
                autre => return Err(format!("moteur inconnu : {autre:?}").into()),
            };

            let response = db
                .query_one_raw(Statement::from_string(backend, requete))
                .await?
                .ok_or("la base n'a pas rendu sa version")?;

            println!("version\t{}", response.try_get_by_index::<String>(0)?);
        }
        _ => return Err(format!("commande inconnue : {commande}").into()),
    }

    Ok(())
}
