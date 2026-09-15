use super::*;

use sea_orm::TransactionTrait;

use super::super::delivery::Event;

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn emitting_an_event_enqueues_one_delivery_per_listening_subscription() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    abonne(db, "https://un.example.com/hook", &["user.created"]).await;
    abonne(db, "https://deux.example.com/hook", &["user.*"]).await;

    let enfilees = super::super::emit(db, "user.created", &donnees())
        .await
        .expect("l'émission aboutit");

    assert_eq!(enfilees, 2);
    assert_eq!(livraisons(db).await, 2);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_revoked_subscription_is_not_delivered_to() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    abonne(db, "https://vivant.example.com/hook", &["*"]).await;
    abonne_revoque(db, "https://parti.example.com/hook", &["*"]).await;

    let enfilees = super::super::emit(db, "user.created", &donnees())
        .await
        .expect("l'émission aboutit");

    assert_eq!(enfilees, 1, "un abonnement révoqué a reçu une livraison");
    assert_eq!(livraisons(db).await, 1);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_subscription_that_does_not_listen_receives_nothing() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    abonne(db, "https://commandes.example.com/hook", &["order.*"]).await;

    let enfilees = super::super::emit(db, "user.created", &donnees())
        .await
        .expect("l'émission aboutit");

    // Aucun abonné concerné n'est pas une erreur : un projet sans webhook configuré émet
    // dans le vide, ce qui est le cas nominal.
    assert_eq!(enfilees, 0);
    assert_eq!(livraisons(db).await, 0);
}

/// L'abonnement est relu au dépilage, et c'est ce qui rend la révocation rétroactive sur
/// les livraisons déjà en file.
///
/// L'URL est injoignable par construction : si la relecture cessait d'arrêter la livraison,
/// le job partirait poster et échouerait au lieu de rendre `Ok(())`. Ce test n'a donc besoin
/// d'aucun serveur.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_delivery_whose_subscription_was_revoked_succeeds_without_a_request() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let abonnement = abonne_revoque(db, "http://127.0.0.1:1/hook", &["*"]).await;

    let job = Delivery {
        subscription: abonnement.id,
        event: Event {
            id: Uuid::now_v7(),
            event: "user.created".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            data: donnees(),
        },
    };

    job.run(&state)
        .await
        .expect("une livraison sans destinataire n'a rien à réessayer");
}

/// Le critère qui justifie qu'`emit` prenne un `ConnectionTrait` et non une connexion.
///
/// Un `user.created` livré alors que l'inscription a été annulée serait un mensonge que
/// rien ne rattrape : les livraisons naissent si et seulement si la transaction du métier
/// est committée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_emission_rolled_back_with_its_transaction_enqueues_nothing() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    abonne(db, "https://un.example.com/hook", &["*"]).await;

    let transaction = db.begin().await.expect("transaction ouvrable");
    let enfilees = super::super::emit(&transaction, "user.created", &donnees())
        .await
        .expect("l'émission aboutit dans la transaction");
    assert_eq!(enfilees, 1);
    transaction.rollback().await.expect("transaction annulable");

    assert_eq!(
        livraisons(db).await,
        0,
        "une livraison a survécu au rollback de la transaction qui la motivait"
    );
}
