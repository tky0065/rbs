use std::sync::OnceLock;
use std::time::Duration;

use sea_orm::DatabaseConnection;
use sea_orm::prelude::{Expr, Uuid};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tokio::sync::{Barrier, Mutex, MutexGuard};

use super::model::{Column, Entity, Status};
use super::{Config, Job, Registry};
use crate::state::AppState;

mod lease;
mod reservation;
mod retry;
mod worker;

/// Un job qui réussit toujours.
#[derive(Debug, Serialize, Deserialize)]
struct Succeeds {
    marque: String,
}

/// Un job qui échoue toujours : c'est le seul moyen d'observer le réessai.
#[derive(Debug, Serialize, Deserialize)]
struct AlwaysFails;

/// Un job qui dure : c'est le seul moyen d'observer un arrêt demandé en plein job.
#[derive(Debug, Serialize, Deserialize)]
struct Slow;

/// Un job qui n'avance que si un second est en cours au même instant : c'est le seul
/// moyen d'observer que le worker en exécute plusieurs de front.
#[derive(Debug, Serialize, Deserialize)]
struct Meet;

fn rendez_vous() -> &'static Barrier {
    static RENDEZ_VOUS: OnceLock<Barrier> = OnceLock::new();

    RENDEZ_VOUS.get_or_init(|| Barrier::new(2))
}

#[async_trait::async_trait]
impl Job for Succeeds {
    const KIND: &'static str = "tests::succeeds";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl Job for AlwaysFails {
    const KIND: &'static str = "tests::always_fails";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        anyhow::bail!("ce job échoue par construction")
    }
}

#[async_trait::async_trait]
impl Job for Slow {
    const KIND: &'static str = "tests::slow";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Job for Meet {
    const KIND: &'static str = "tests::meet";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        rendez_vous().wait().await;

        Ok(())
    }
}

fn registry() -> Registry {
    Registry::new()
        .register::<Succeeds>()
        .register::<AlwaysFails>()
        .register::<Slow>()
        .register::<Meet>()
}

/// Un état dont la connexion n'est jamais ouverte : ces tests-là n'interrogent rien.
fn detached_state() -> AppState {
    let config = rbs_core::Config::load().expect("configuration lisible");

    AppState::new(DatabaseConnection::default(), config).expect("état constructible")
}

// Les tests qui suivent joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Le verrou de tout test qui joint la base de ce projet.
///
/// Il est rendu au reste du projet parce que la file est une table partagée : un fragment
/// qui y enfile — le calendrier — a des tests qui doivent se relayer avec ceux-ci, faute
/// de quoi le vidage de l'un emporte les lignes que l'autre vient d'observer.
pub(crate) fn verrou_base() -> &'static Mutex<()> {
    static VERROU: OnceLock<Mutex<()>> = OnceLock::new();

    VERROU.get_or_init(Mutex::default)
}

/// Les tests qui dépilent partagent l'unique table `jobs` : ils se relaient plutôt que de
/// se voler leurs lignes.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    let garde = verrou_base().lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table jobs doit se vider");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

fn config(max_attempts: i32) -> Config {
    Config {
        max_attempts,
        // Aucun délai : le test rejoue la tentative suivante tout de suite.
        retry_delay_secs: 0,
        retry_max_delay_secs: 3600,
        poll_interval_secs: 1,
        lease_secs: 300,
        concurrency: 1,
    }
}

/// Une configuration dont seul le délai de reprise compte.
fn delai(retry_delay_secs: u64, retry_max_delay_secs: u64) -> Config {
    Config {
        retry_delay_secs,
        retry_max_delay_secs,
        ..config(5)
    }
}

/// Vieillit la réservation de `id` : ce que ferait le temps, sans attendre le bail.
async fn reserved_since(db: &DatabaseConnection, id: Uuid, age: Duration) {
    let alors = (chrono::Utc::now() - chrono::TimeDelta::from_std(age).expect("durée finie"))
        .fixed_offset();
    Entity::update_many()
        .col_expr(Column::UpdatedAt, Expr::value(alors))
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .expect("la réservation se vieillit");
}

/// Attend que la ligne `id` atteigne `attendu`, et échoue passé `limite`.
async fn attendre_le_statut(db: &DatabaseConnection, id: Uuid, attendu: Status, limite: Duration) {
    let fin = tokio::time::Instant::now() + limite;

    loop {
        let ligne = Entity::find_by_id(id)
            .one(db)
            .await
            .expect("lecture possible")
            .expect("la ligne survit");
        if ligne.status == attendu {
            return;
        }
        assert!(
            tokio::time::Instant::now() < fin,
            "la ligne est restée `{}` au lieu de passer `{}`",
            ligne.status.as_str(),
            attendu.as_str()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
