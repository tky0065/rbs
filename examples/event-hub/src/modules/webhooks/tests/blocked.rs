use super::*;

use serde_json::json;

use super::super::delivery::Event;
use super::super::target::Refusal;

/// La route refuse une URL interne avant d'inscrire quoi que ce soit, qu'elle enfreigne
/// `Https` (un `http` hors développement) ou `PrivateHost` (un littéral privé, même en
/// `https`) : le contrôleur ne construit rien lui-même, il transmet
/// `state.webhooks().policy()` au service, qui refuse les deux de la même façon.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_subscribing_a_private_url_gets_400() {
    let (_garde, state) = table_a_soi_stricte().await;
    let db = state.core().db().clone();
    let api = crate::router::router(state);

    let corps = json!({ "url": "http://169.254.169.254/latest/meta-data", "events": ["*"] });
    let (statut, reponse) = call(
        &api,
        request(&db, "POST", "/webhooks/subscriptions", "admin", Some(corps)).await,
    )
    .await;
    assert_eq!(statut, StatusCode::BAD_REQUEST, "{reponse}");
    assert!(
        reponse["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("https"),
        "le message doit nommer la règle enfreinte : {reponse}"
    );

    let corps = json!({ "url": "https://10.0.0.5/hooks", "events": ["*"] });
    let (statut, reponse) = call(
        &api,
        request(&db, "POST", "/webhooks/subscriptions", "admin", Some(corps)).await,
    )
    .await;
    assert_eq!(statut, StatusCode::BAD_REQUEST, "{reponse}");

    // Rien n'a été inscrit : c'est la table des abonnements qui le dit, pas la file, où
    // rien n'entre sans `emit`.
    let inscrits = Entity::find().all(&db).await.expect("lecture possible");
    assert!(
        inscrits.is_empty(),
        "un abonnement refusé a été inscrit : {inscrits:?}"
    );
}

/// `post` rend le type de refus, ce que `run` ne laisse plus voir : pour une URL en clair,
/// la première règle enfreinte est le schéma, avant même que l'hôte soit regardé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn post_refuses_a_blocked_target_before_sending() {
    let (_garde, state) = table_a_soi_stricte().await;
    let db = state.core().db();
    let abonnement = abonne(db, "http://169.254.169.254/latest", &["*"]).await;

    let resultat = state
        .webhooks()
        .post(
            &abonnement.url,
            "t=0,v1=0",
            "user.created",
            Uuid::now_v7(),
            Vec::new(),
        )
        .await;

    assert!(
        matches!(
            resultat,
            Err(super::super::delivery::PostError::Blocked(Refusal::Https))
        ),
        "{resultat:?}"
    );
}

/// Une cible interdite ne se réessaie pas : cinq tentatives de plus n'y changeraient rien, et
/// `run` le dit en rendant `Ok` après un simple avertissement, sans qu'aucune requête ne
/// parte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_delivery_to_a_blocked_target_is_abandoned_not_retried() {
    let (_garde, state) = table_a_soi_stricte().await;
    let db = state.core().db();
    let (port, recu) = receveur_local().await;

    let abonnement = abonne(db, &format!("https://127.0.0.1:{port}/hook"), &["*"]).await;
    let livraison = Delivery {
        subscription: abonnement.id,
        event: Event {
            id: Uuid::now_v7(),
            event: "user.created".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            data: donnees(),
        },
    };

    livraison
        .run(&state)
        .await
        .expect("une cible interdite n'a rien à réessayer");
    assert_eq!(recu.load(std::sync::atomic::Ordering::SeqCst), 0);
}

/// En développement, un receveur local est joint : c'est le chemin nominal du poste de
/// travail, et il traverse le résolveur filtrant du client — qui doit laisser passer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn in_development_a_local_receiver_is_reached() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let (port, recu) = receveur_local().await;

    let abonnement = abonne(db, &format!("http://localhost:{port}/hook"), &["*"]).await;
    let livraison = Delivery {
        subscription: abonnement.id,
        event: Event {
            id: Uuid::now_v7(),
            event: "user.created".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            data: donnees(),
        },
    };

    livraison.run(&state).await.expect("livraison acceptée");
    assert_eq!(recu.load(std::sync::atomic::Ordering::SeqCst), 1);
}
