use super::*;

use axum::body::to_bytes;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};
use tower::ServiceExt;

use crate::auth::model::refresh_token;
use crate::auth::repository::refresh_token::Rotation;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// L'unique ligne de `refresh_tokens` ouverte pour `user_id`.
///
/// La recherche porte sur le compte et non sur l'empreinte : chercher par ce qu'on veut
/// vérifier ferait échouer le test à la lecture, sans jamais atteindre l'assertion.
async fn session_row(db: &DatabaseConnection, user_id: Uuid) -> refresh_token::Model {
    refresh_token::Entity::find()
        .filter(refresh_token::Column::UserId.eq(user_id))
        .one(db)
        .await
        .expect("la table doit être interrogeable")
        .expect("la session ouverte doit avoir sa ligne")
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_valid_refresh_returns_a_new_pair() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;

    let (status, nouvelle) = refresh(&api, &refresh_for(&paire)).await;

    assert_eq!(status, StatusCode::OK, "{nouvelle}");
    assert_eq!(nouvelle["token_type"], "Bearer", "{nouvelle}");
    assert_ne!(
        nouvelle["refresh_token"], paire["refresh_token"],
        "le jeton de rafraîchissement doit tourner"
    );
    assert_ne!(
        nouvelle["access_token"], paire["access_token"],
        "le jeton d'accès doit être réémis"
    );
}

/// Ce que `refresh` enchaîne — tourner la session, en ouvrir une neuve — se défait d'un
/// bloc quand la transaction qui le porte est abandonnée : une erreur entre les deux ne
/// laisse pas au client une session tournée sans paire pour la remplacer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rolled_back_rotation_leaves_the_session_open_and_opens_no_other() {
    let api = application().await;
    let db = connection().await;
    let (id, _) = login_as(&api).await;
    let session = session_row(&db, id).await;

    let transaction = db.begin().await.expect("transaction ouvrable");
    let tournee = crate::auth::repository::refresh_token::rotate(&transaction, session.id)
        .await
        .expect("pas d'erreur");
    assert_eq!(tournee, Rotation::Done, "la session vient d'être ouverte");
    crate::auth::repository::create_refresh_token(
        &transaction,
        id,
        rbs_core::token::fingerprint(&rbs_core::token::random()),
        (Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("pas d'erreur");
    transaction.rollback().await.expect("transaction annulable");

    let ouvertes = crate::auth::repository::open_sessions_of(&db, id)
        .await
        .expect("la lecture aboutit");
    assert_eq!(
        ouvertes.iter().map(|ligne| ligne.id).collect::<Vec<_>>(),
        vec![session.id],
        "une transaction annulée a tourné la session ou en a ouvert une autre"
    );

    let tournee = crate::auth::repository::refresh_token::rotate(&db, session.id)
        .await
        .expect("pas d'erreur");
    assert_eq!(
        tournee,
        Rotation::Done,
        "la session a été tournée par une transaction annulée"
    );
}

/// Une réponse portant des jetons ne se met pas en cache — RFC 6749 §5.1.
///
/// La garantie tient au type `TokenPair`, qui porte sa propre réponse : un handler ajouté
/// ici plus tard la reçoit sans y penser. C'est cette propriété que le test éprouve, en
/// interrogeant les deux routes qui rendent la paire.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_response_carrying_tokens_forbids_its_own_caching() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let connexion = post_json(
        "/auth/login",
        json!({ "email": email, "password": PASSWORD }),
    );
    let response = api
        .clone()
        .oneshot(connexion)
        .await
        .expect("l'application doit répondre");

    assert_eq!(response.status(), StatusCode::OK);
    // Les en-têtes sont relevés avant que la lecture du corps ne consomme la réponse.
    let entetes = response.headers().clone();
    assert_eq!(
        entetes.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("no-store"),
        "la paire de `login` doit refuser le cache"
    );
    assert_eq!(
        entetes.get("pragma").and_then(|v| v.to_str().ok()),
        Some("no-cache"),
        "la paire de `login` doit refuser le cache des intermédiaires HTTP/1.0"
    );

    let octets = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");
    let paire: Value = serde_json::from_slice(&octets).expect("la paire est du JSON");

    let renouvellement = post_json(
        "/auth/refresh",
        json!({ "refresh_token": refresh_for(&paire) }),
    );
    let response = api
        .clone()
        .oneshot(renouvellement)
        .await
        .expect("l'application doit répondre");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-store"),
        "la paire de `refresh` doit refuser le cache"
    );
}

/// Un jeton rejoué ferme le compte, et non la seule ligne qu'il présente.
///
/// Sans cela, un jeton volé et joué avant la rotation légitime laisse son porteur avec
/// une paire valide, qu'il renouvelle indéfiniment et sans bruit : c'est la détection de
/// réutilisation, et elle n'a de sens que si toute la famille tombe.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn replaying_a_refresh_closes_the_other_sessions_of_the_account() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let (_, seconde) = authenticate(&api, &email, PASSWORD).await;

    let ancien = refresh_for(&premiere);
    let (status, tournee) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK, "{tournee}");

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    // La paire née de la rotation légitime tombe elle aussi : rien ne dit lequel des deux
    // porteurs du jeton rejoué la détient.
    let (status, body) = refresh(&api, &refresh_for(&tournee)).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "la paire issue de la rotation survit au rejeu : {body}"
    );

    let (status, body) = refresh(&api, &refresh_for(&seconde)).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "la session sœur survit au rejeu : {body}"
    );
}

/// Le rejeu d'un jeton tourné est instruit une fois : la famille tombe, la ligne se ferme.
/// Le rejouer encore ne ferme plus rien — sinon qui tient un jeton mort déconnecterait le
/// titulaire à chaque reconnexion, jusqu'à l'échéance du jeton.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_replayed_token_presented_again_leaves_the_sessions_opened_since_alive() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let ancien = refresh_for(&premiere);
    let (status, _) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK);

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    // Le titulaire se reconnecte après l'expulsion.
    let (_, reconnexion) = authenticate(&api, &email, PASSWORD).await;

    let (rejeu, body) = refresh(&api, &ancien).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&reconnexion)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la session ouverte après le rejeu est tombée sur un second rejeu : {body}"
    );
}

/// Une ligne tournée n'est pas une session ouverte : sans le filtre sur `replaced_at`,
/// la liste grossirait d'une ligne à chaque rafraîchissement, trente jours durant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rotated_session_is_listed_once_after_two_refreshes() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (_, paire) = authenticate(&api, &email, PASSWORD).await;
    let (status, tournee) = refresh(&api, &refresh_for(&paire)).await;
    assert_eq!(status, StatusCode::OK, "{tournee}");
    let (status, retournee) = refresh(&api, &refresh_for(&tournee)).await;
    assert_eq!(status, StatusCode::OK, "{retournee}");

    let (status, sessions) = call(
        &api,
        get_authenticated("/auth/sessions", &access_for(&retournee)),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{sessions}");
    assert_eq!(
        sessions.as_array().map(Vec::len),
        Some(1),
        "les lignes tournées sont listées : {sessions}"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_old_refresh_is_then_rejected() {
    let api = application().await;
    let ancien = refresh_for(&login_as(&api).await.1);

    let (status, body) = refresh(&api, &ancien).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (rejeu, body) = refresh(&api, &ancien).await;

    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");
}

/// La ligne est reculée dans le passé plutôt que forgée : elle garde ainsi tout ce que
/// `login` y a mis, et seule son expiration la disqualifie.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_expired_refresh_returns_401() {
    let api = application().await;
    let db = connection().await;
    let (compte, paire) = login_as(&api).await;
    let token = refresh_for(&paire);

    let mut ligne: refresh_token::ActiveModel = session_row(&db, compte).await.into();
    ligne.expires_at = Set((Utc::now() - chrono::Duration::seconds(1)).fixed_offset());
    ligne.update(&db).await.expect("expiration reculée");

    let (status, body) = refresh(&api, &token).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// La liste juge l'échéance à l'instant, comme `refresh` : une session échue d'une
/// seconde n'y figure plus.
///
/// `refresh` compare en Rust, la liste dans la requête — et sur SQLite l'horloge du
/// moteur n'avait pas le format textuel de la colonne : une session échue le jour même y
/// restait listée jusqu'à minuit UTC.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_expired_session_is_no_longer_listed() {
    let api = application().await;
    let db = connection().await;
    let (compte, paire) = login_as(&api).await;

    let mut ligne: refresh_token::ActiveModel = session_row(&db, compte).await.into();
    ligne.expires_at = Set((Utc::now() - chrono::Duration::seconds(1)).fixed_offset());
    ligne.update(&db).await.expect("expiration reculée");

    let (status, body) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            paire["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body.as_array().map(Vec::len),
        Some(0),
        "une session échue est encore listée : {body}"
    );
}

/// Une base lue par un tiers ne lui donne aucune session utilisable.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_table_carries_the_fingerprint_and_never_the_token() {
    let api = application().await;
    let db = connection().await;
    let (compte, paire) = login_as(&api).await;
    let token = refresh_for(&paire);

    let ligne = session_row(&db, compte).await;

    assert_eq!(
        ligne.token_hash,
        rbs_core::token::fingerprint(&token),
        "la colonne ne porte pas l'empreinte du jeton"
    );
    assert_ne!(ligne.token_hash, token, "le jeton lui-même est stocké");

    let en_clair = refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(token.clone()))
        .one(&db)
        .await
        .expect("la table doit être interrogeable");

    assert!(
        en_clair.is_none(),
        "une ligne porte le jeton remis au client"
    );
}
