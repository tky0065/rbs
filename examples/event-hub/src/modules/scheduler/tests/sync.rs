use super::*;

use rbs_core::HasCoreState;

use super::super::sync;

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_newly_declared_schedule_is_inserted_with_its_next_occurrence() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");

    let ligne = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance déclarée doit avoir été insérée");

    assert!(ligne.next_run_at > chrono::Utc::now());
    assert!(ligne.last_run_at.is_none(), "rien n'a encore été déclenché");
}

/// Le pendant, au démarrage, de la garantie que porte le tick.
///
/// Un déploiement lance ses réplicas ensemble, et tous réconcilient une table où l'échéance
/// n'existe pas encore. Une insertion qui ne tolère pas d'avoir été devancée fait échouer
/// le démarrage de tous sauf un — sur le fragment dont l'argument même est de tenir le
/// multi-réplica.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "joint la base du projet"]
async fn concurrent_boots_all_succeed_and_insert_one_row() {
    const REPLICAS: usize = 8;

    let (_garde, state) = table_a_soi().await;

    let mut taches = Vec::new();
    for _ in 0..REPLICAS {
        let db = state.core().db().clone();
        taches.push(tokio::spawn(async move {
            sync::reconcilier(&db, &calendrier("0 3 * * *")).await
        }));
    }

    for (rang, tache) in taches.into_iter().enumerate() {
        tache
            .await
            .expect("la réconciliation ne panique pas")
            .unwrap_or_else(|error| panic!("le démarrage du réplica {rang} a échoué : {error}"));
    }

    let lignes = Entity::find()
        .filter(super::super::model::Column::Kind.eq(Scheduled::KIND))
        .all(state.core().db())
        .await
        .expect("lecture possible");

    assert_eq!(lignes.len(), 1, "{} lignes pour une échéance", lignes.len());
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_schedule_removed_from_the_code_is_removed_from_the_table() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    // Sans cette suppression, une échéance retirée du code resterait due pour toujours,
    // que plus personne ne réserverait ni ne ferait avancer.
    sync::reconcilier(db, &[])
        .await
        .expect("la réconciliation aboutit");

    assert!(
        Entity::find_by_id(Scheduled::KIND)
            .one(db)
            .await
            .expect("lecture possible")
            .is_none(),
        "l'échéance retirée du code survit dans la table"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_redeploy_does_not_move_the_next_occurrence_of_a_known_schedule() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    let avant = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    let apres = Entity::find_by_id(Scheduled::KIND)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    // C'est ce qui rend un déploiement invisible pour le calendrier : un redémarrage ne
    // rejoue pas une échéance passée et ne repousse pas une échéance imminente.
    assert_eq!(avant, apres);
}

/// Le pendant du test précédent : une échéance dont l'expression a changé n'a pas à
/// attendre l'occurrence de l'ancienne — un mois, ici — pour que le code fasse foi.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_changed_expression_moves_the_next_occurrence() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 1 * *"))
        .await
        .expect("la réconciliation aboutit");
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

    assert_ne!(
        avant, apres,
        "l'échéance est restée sur l'ancienne expression"
    );
    assert!(
        apres <= chrono::Utc::now() + chrono::TimeDelta::minutes(5),
        "{apres} n'est pas une occurrence de `*/5 * * * *`"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unparsable_expression_stops_the_reconciliation() {
    let (_garde, state) = table_a_soi().await;

    let erreur = sync::reconcilier(state.core().db(), &calendrier("0 99 * * *"))
        .await
        .expect_err("une expression illisible doit arrêter le démarrage");

    assert!(erreur.to_string().contains("0 99 * * *"), "{erreur}");
}
