use super::*;

use std::collections::HashSet;

use rbs_core::HasCoreState;
use sea_orm::TransactionTrait;

use super::super::queue;

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
