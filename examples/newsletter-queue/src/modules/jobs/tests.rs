use std::collections::HashSet;
use std::sync::OnceLock;

use rbs_core::HasCoreState;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, MutexGuard};

use super::model::{Entity, Status};
use super::{Config, Job, Registry, queue, worker};
use crate::state::AppState;

/// Un job qui réussit toujours.
#[derive(Debug, Serialize, Deserialize)]
struct Succeeds {
    marque: String,
}

/// Un job qui échoue toujours : c'est le seul moyen d'observer le réessai.
#[derive(Debug, Serialize, Deserialize)]
struct AlwaysFails;

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

fn registry() -> Registry {
    Registry::new()
        .register::<Succeeds>()
        .register::<AlwaysFails>()
}

/// Un état dont la connexion n'est jamais ouverte : ces tests-là n'interrogent rien.
fn detached_state() -> AppState {
    let config = rbs_core::Config::load().expect("configuration lisible");

    AppState::new(DatabaseConnection::default(), config).expect("état constructible")
}

#[test]
fn each_status_carries_the_value_the_column_holds() {
    // La requête de réservation compare ces chaînes : les renommer sans toucher à la
    // table ferait un dépilage qui ne trouve jamais rien, et aucun test de compilation
    // ne le verrait.
    assert_eq!(Status::Pending.as_str(), "pending");
    assert_eq!(Status::Running.as_str(), "running");
    assert_eq!(Status::Done.as_str(), "done");
    assert_eq!(Status::Failed.as_str(), "failed");
}

#[test]
fn the_payload_of_a_job_reads_back_into_its_type() {
    let job = Succeeds {
        marque: "ada".to_string(),
    };

    let payload = serde_json::to_value(&job).expect("le job est sérialisable");
    let relu: Succeeds = serde_json::from_value(payload).expect("le payload est relisible");

    assert_eq!(relu.marque, "ada");
}

#[tokio::test]
async fn an_unregistered_kind_is_reported_rather_than_silently_dropped() {
    let error = registry()
        .run(&detached_state(), "inconnu", serde_json::json!({}))
        .await
        .expect_err("un `kind` absent du registre ne peut pas s'exécuter");

    assert!(error.to_string().contains("inconnu"), "{error}");
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
        poll_interval_secs: 1,
    }
}

/// Le critère qui justifie d'avoir mis la file en base plutôt que dans Redis.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_enqueued_in_a_rolled_back_transaction_does_not_exist() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let transaction = db.begin().await.expect("transaction ouvrable");
    let id = queue::enqueue(
        &transaction,
        &Succeeds {
            marque: "annulée".to_string(),
        },
    )
    .await
    .expect("le job s'enfile dans la transaction");
    transaction.rollback().await.expect("transaction annulable");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible");

    assert!(
        ligne.is_none(),
        "le job a survécu au rollback de la transaction qui le motivait"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_enqueued_in_a_committed_transaction_is_visible_to_the_worker() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let transaction = db.begin().await.expect("transaction ouvrable");
    let id = queue::enqueue(
        &transaction,
        &Succeeds {
            marque: "committée".to_string(),
        },
    )
    .await
    .expect("le job s'enfile dans la transaction");
    transaction.commit().await.expect("transaction committable");

    // C'est le dépilage qui dit la visibilité, non un `SELECT` : le worker ne voit la
    // file que par lui.
    let reserve = queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job committé doit être dépilable");

    assert_eq!(reserve.id, id);
    assert_eq!(reserve.status, Status::Running);
    assert_eq!(reserve.attempts, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "joint la base du projet"]
async fn two_concurrent_workers_never_reserve_the_same_job() {
    const JOBS: usize = 200;
    const WORKERS: usize = 8;

    let (_garde, state) = table_a_soi().await;

    for rang in 0..JOBS {
        queue::enqueue(
            state.core().db(),
            &Succeeds {
                marque: rang.to_string(),
            },
        )
        .await
        .expect("le job s'enfile");
    }

    let mut taches = Vec::new();
    for _ in 0..WORKERS {
        let db = state.core().db().clone();
        taches.push(tokio::spawn(async move {
            let mut reserves = Vec::new();
            while let Some(job) = queue::reserver_prochain_job(&db)
                .await
                .expect("dépilage possible")
            {
                reserves.push(job.id);
            }
            reserves
        }));
    }

    let mut tous = Vec::new();
    for tache in taches {
        tous.extend(tache.await.expect("le worker ne panique pas"));
    }

    let distincts: HashSet<Uuid> = tous.iter().copied().collect();

    assert_eq!(
        distincts.len(),
        tous.len(),
        "{} job(s) réservé(s) deux fois",
        tous.len() - distincts.len()
    );
    assert_eq!(tous.len(), JOBS, "la file n'a pas été vidée en entier");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_failing_job_is_retried_then_marked_failed_after_the_last_attempt() {
    const TENTATIVES: i32 = 3;

    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let config = config(TENTATIVES);
    let registry = registry();

    let id = queue::enqueue(db, &AlwaysFails)
        .await
        .expect("le job s'enfile");

    for tentative in 1..=TENTATIVES {
        let job = queue::reserver_prochain_job(db)
            .await
            .expect("dépilage possible")
            .unwrap_or_else(|| panic!("le job doit être dépilable à la tentative {tentative}"));

        worker::execute(&state, &registry, &config, job).await;

        let ligne = Entity::find_by_id(id)
            .one(db)
            .await
            .expect("lecture possible")
            .expect("la ligne survit à ses tentatives");

        assert_eq!(ligne.attempts, tentative);
        assert!(ligne.last_error.is_some(), "l'échec n'est pas consigné");

        let attendu = if tentative < TENTATIVES {
            Status::Pending
        } else {
            Status::Failed
        };
        assert_eq!(ligne.status, attendu, "à la tentative {tentative}");
    }

    assert!(
        queue::reserver_prochain_job(db)
            .await
            .expect("dépilage possible")
            .is_none(),
        "un job en échec définitif est revenu dans la file"
    );
}
