use super::*;

use rbs_core::HasCoreState;

use super::super::{sync, ticker};

/// Une échéance échue reste au tick, qui la rejoue puis l'avance selon l'expression
/// nouvelle : la réconciliation qui la déplacerait effacerait une occurrence gagnée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_due_schedule_is_not_moved_by_a_changed_expression() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    echeance_due(db).await;
    let avant = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    sync::reconcilier(db, &calendrier("*/5 * * * *"))
        .await
        .expect("la réconciliation aboutit");
    let apres = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    assert_eq!(
        avant, apres,
        "l'échéance échue a été déplacée sans avoir été jouée"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_due_schedule_enqueues_its_job_and_moves_on() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    echeance_due(db).await;

    let declenchees = ticker::tick(db, &calendrier("0 3 * * *")).await;
    assert_eq!(declenchees, 1);

    let ligne = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe");
    assert!(
        ligne.next_run_at > chrono::Utc::now(),
        "l'échéance n'a pas avancé"
    );
    assert!(
        ligne.last_run_at.is_some(),
        "le déclenchement n'est pas daté"
    );

    // C'est le job qui compte : une échéance qui avance sans rien enfiler serait une
    // horloge qui tourne à vide.
    let jobs = jobs_declenches(db).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].kind, Scheduled::KIND);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_schedule_that_is_not_due_is_left_alone() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");

    let declenchees = ticker::tick(db, &calendrier("0 3 * * *")).await;

    assert_eq!(declenchees, 0);
    assert!(
        jobs_declenches(db).await.is_empty(),
        "un job est né d'une échéance qui n'était pas due"
    );
}

/// La garantie qui justifie d'avoir mis le calendrier en base plutôt que dans le
/// processus : trois réplicas, une seule purge nocturne.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "joint la base du projet"]
async fn concurrent_tickers_trigger_a_due_schedule_exactly_once() {
    const TICKERS: usize = 8;

    let (_garde, state) = table_a_soi().await;
    echeance_due(state.core().db()).await;

    let mut taches = Vec::new();
    for _ in 0..TICKERS {
        let db = state.core().db().clone();
        taches.push(tokio::spawn(async move {
            ticker::tick(&db, &calendrier("0 3 * * *")).await
        }));
    }

    let mut declenchees = 0;
    for tache in taches {
        declenchees += tache.await.expect("le ticker ne panique pas");
    }

    assert_eq!(
        declenchees, 1,
        "{declenchees} réplicas ont cru gagner l'échéance"
    );

    let jobs = jobs_declenches(state.core().db()).await;
    assert_eq!(jobs.len(), 1, "l'échéance a enfilé {} job(s)", jobs.len());
}

/// Le ticker que `spawn` détache rend la main quand l'arrêt est demandé : sans cela,
/// `main` l'attendrait jusqu'à l'échéance de `server.shutdown_timeout_secs`.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_ticker_stops_when_shutdown_is_requested() {
    let (_garde, state) = table_a_soi().await;

    super::super::spawn(state.clone())
        .await
        .expect("le calendrier du projet se réconcilie");

    assert_eq!(
        state
            .core()
            .shutdown()
            .wait(std::time::Duration::from_secs(5))
            .await,
        0,
        "le ticker n'a pas rendu la main"
    );
}
