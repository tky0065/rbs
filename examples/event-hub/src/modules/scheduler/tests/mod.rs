use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tokio::sync::MutexGuard;

use super::Schedule;
use super::model::{ActiveModel, Entity};
use crate::modules::jobs::Job;
use crate::state::AppState;

mod expression;
mod sync;
mod ticker;

/// Le job que ces tests font déclencher.
///
/// Un type à eux, et non celui d'exemple du fragment `jobs` : la file est une table
/// partagée, et les tests des deux fragments tournent de front dans le même binaire. Un
/// `KIND` qui n'appartient qu'ici est ce qui permet de ne vider et de ne compter que ses
/// propres lignes, sans emporter celles que le voisin est en train d'observer.
#[derive(Debug, Serialize, Deserialize)]
struct Scheduled;

#[async_trait::async_trait]
impl Job for Scheduled {
    const KIND: &'static str = "tests::scheduled";

    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Les lignes de la file que ces tests ont fait naître, et elles seules.
async fn jobs_declenches(db: &DatabaseConnection) -> Vec<crate::modules::jobs::model::Model> {
    crate::modules::jobs::model::Entity::find()
        .filter(crate::modules::jobs::model::Column::Kind.eq(Scheduled::KIND))
        .all(db)
        .await
        .expect("lecture possible")
}

// Les tests qui suivent joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Les tests qui réconcilient partagent l'unique table `schedules` : ils se relaient
/// plutôt que de se voler leurs lignes.
///
/// Le verrou est celui du fragment `jobs`, et non un second : un tick enfile dans la
/// file, dont les tests vident la table sans filtre. Deux verrous indépendants
/// laisseraient chaque suite effacer ce que l'autre vient d'écrire, et les deux
/// paraîtraient fautives à tour de rôle.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    let garde = crate::modules::jobs::tests::verrou_base().lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table schedules doit se vider");
    // De la file, seules les lignes de ce module : les tests du fragment `jobs` tournent
    // dans le même binaire et sur la même table, et un vidage sans filtre leur retirerait
    // sous les pieds les jobs qu'ils viennent d'enfiler.
    crate::modules::jobs::model::Entity::delete_many()
        .filter(crate::modules::jobs::model::Column::Kind.eq(Scheduled::KIND))
        .exec(&db)
        .await
        .expect("la file doit se vider de ses jobs déclenchés");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Un calendrier de test, indépendant de celui que le projet déclare.
fn calendrier(expression: &'static str) -> Vec<Schedule> {
    vec![Schedule::every::<Scheduled>(expression, || Scheduled)]
}

/// Pose une échéance déjà due, en court-circuitant la réconciliation.
async fn echeance_due(db: &DatabaseConnection) {
    use sea_orm::ActiveValue::Set;

    let maintenant = super::a_la_seconde(chrono::Utc::now().fixed_offset());

    ActiveModel {
        kind: Set(Scheduled::KIND.to_string()),
        next_run_at: Set((chrono::Utc::now() - chrono::TimeDelta::hours(1)).fixed_offset()),
        last_run_at: Set(None),
        created_at: Set(maintenant),
        updated_at: Set(maintenant),
    }
    .insert(db)
    .await
    .expect("l'échéance due s'insère");
}
