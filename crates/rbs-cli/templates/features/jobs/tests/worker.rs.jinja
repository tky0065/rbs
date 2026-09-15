use super::*;

use std::time::Duration;

use rbs_core::HasCoreState;

use super::super::{queue, worker};

/// Un arrêt demandé pendant un job le laisse finir, puis le worker rend la main : c'est
/// ce qui rend le bail inutile en temps normal.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_worker_finishes_its_job_and_stops_when_shutdown_is_requested() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let id = queue::enqueue(db, &Slow).await.expect("le job s'enfile");

    let shutdown = state.core().shutdown().clone();
    shutdown.spawn(worker::run_with(state.clone(), registry(), config(5)));
    attendre_le_statut(db, id, Status::Running, Duration::from_secs(5)).await;

    assert_eq!(
        shutdown.wait(Duration::from_secs(10)).await,
        0,
        "le worker n'a pas rendu la main"
    );

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne survit");
    assert_eq!(
        ligne.status,
        Status::Done,
        "le job en cours n'a pas été mené à son terme"
    );
    assert_eq!(ligne.attempts, 1);
}

/// Deux jobs qui doivent se rencontrer ne le peuvent que si le worker les exécute de
/// front : un worker strictement séquentiel bloque le premier sur la barrière, et le
/// second n'est jamais réservé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn jobs_run_side_by_side_up_to_the_configured_concurrency() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let premier = queue::enqueue(db, &Meet).await.expect("le job s'enfile");
    let second = queue::enqueue(db, &Meet).await.expect("le job s'enfile");

    let mut config = config(5);
    config.concurrency = 2;
    let shutdown = state.core().shutdown().clone();
    shutdown.spawn(worker::run_with(state.clone(), registry(), config));

    attendre_le_statut(db, premier, Status::Done, Duration::from_secs(10)).await;
    attendre_le_statut(db, second, Status::Done, Duration::from_secs(10)).await;

    assert_eq!(
        shutdown.wait(Duration::from_secs(10)).await,
        0,
        "le worker n'a pas rendu la main"
    );
}
