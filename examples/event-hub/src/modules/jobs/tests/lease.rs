use super::*;

use std::time::Duration;

use rbs_core::HasCoreState;

use super::super::{queue, worker};

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

/// Un worker tué entre la réservation et l'inscription du sort laissait sa ligne en
/// `running` pour toujours : rien ne la dépilait plus.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_left_running_past_the_lease_returns_to_the_queue() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let config = config(5);

    let id = queue::enqueue(
        db,
        &Succeeds {
            marque: "abandonné".to_string(),
        },
    )
    .await
    .expect("le job s'enfile");
    let reserve = queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job est dépilable");
    assert_eq!(reserve.status, Status::Running);
    reserved_since(db, id, Duration::from_secs(config.lease_secs + 60)).await;

    let reprise = queue::requeue_stale(db, &config)
        .await
        .expect("reprise possible");
    assert_eq!(
        reprise,
        queue::Reprise {
            rendus: 1,
            condamnes: 0
        }
    );

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne survit");
    assert_eq!(ligne.status, Status::Pending);
    // La tentative abandonnée reste dépensée : c'est elle que `max_attempts` compte.
    assert_eq!(ligne.attempts, 1);

    let repris = queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job rendu est de nouveau dépilable");
    assert_eq!(repris.id, id);
    assert_eq!(repris.attempts, 2);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_running_within_the_lease_is_left_alone() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let config = config(5);

    let id = queue::enqueue(
        db,
        &Succeeds {
            marque: "en cours".to_string(),
        },
    )
    .await
    .expect("le job s'enfile");
    queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job est dépilable");

    let reprise = queue::requeue_stale(db, &config)
        .await
        .expect("reprise possible");
    assert_eq!(reprise, queue::Reprise::default());

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne survit");
    assert_eq!(ligne.status, Status::Running);
}

/// Un job qui tue son worker à chaque tentative n'atteint jamais `retry_or_fail` : sans
/// borne ici, il serait rendu à la file puis réservé sans fin.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_abandoned_on_its_last_attempt_is_failed_rather_than_requeued() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let config = config(1);

    let id = queue::enqueue(
        db,
        &Succeeds {
            marque: "fatal".to_string(),
        },
    )
    .await
    .expect("le job s'enfile");
    queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job est dépilable");
    reserved_since(db, id, Duration::from_secs(config.lease_secs + 60)).await;

    let reprise = queue::requeue_stale(db, &config)
        .await
        .expect("reprise possible");
    assert_eq!(
        reprise,
        queue::Reprise {
            rendus: 0,
            condamnes: 1
        }
    );

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne survit");
    assert_eq!(ligne.status, Status::Failed);
    assert!(ligne.last_error.is_some(), "l'abandon n'est pas consigné");
    assert!(
        queue::reserver_prochain_job(db)
            .await
            .expect("dépilage possible")
            .is_none(),
        "un job condamné est revenu dans la file"
    );
}
