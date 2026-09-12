use super::*;

use chrono::Utc;

/// Deux consommations simultanées du même jeton : une seule passe.
///
/// C'est la garantie que la condition portée par l'`UPDATE` achète, et qu'une lecture
/// suivie d'une écriture ne donnerait pas — les deux franchiraient la lecture avant que
/// l'une ait écrit, et poseraient chacune leur mot de passe.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_token_is_consumed_once_even_under_concurrency() {
    let db = connection().await;
    let compte = registered_user(&db).await;

    let jeton = rbs_core::token::random();
    crate::auth::repository::one_time_token::issue(
        &db,
        compte.id,
        crate::auth::model::TokenPurpose::PasswordReset,
        rbs_core::token::fingerprint(&jeton),
        (Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("le jeton s'émet");

    let ligne = crate::auth::repository::one_time_token::find(
        &db,
        &rbs_core::token::fingerprint(&jeton),
        crate::auth::model::TokenPurpose::PasswordReset,
    )
    .await
    .expect("la lecture aboutit")
    .expect("le jeton vient d'être émis");

    let (un, deux) = tokio::join!(
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
    );

    let passes = [un.expect("pas d'erreur"), deux.expect("pas d'erreur")]
        .into_iter()
        .filter(|passe| *passe)
        .count();

    assert_eq!(passes, 1, "deux consommations concurrentes ont abouti");
}

/// Émet un jeton de réinitialisation dont l'échéance est `decalage` après maintenant, et
/// rend sa ligne.
async fn reset_token_expiring_in(
    db: &DatabaseConnection,
    user_id: Uuid,
    decalage: chrono::Duration,
) -> crate::auth::repository::one_time_token::Model {
    let jeton = rbs_core::token::random();
    crate::auth::repository::one_time_token::issue(
        db,
        user_id,
        crate::auth::model::TokenPurpose::PasswordReset,
        rbs_core::token::fingerprint(&jeton),
        (Utc::now() + decalage).fixed_offset(),
    )
    .await
    .expect("le jeton s'émet");

    crate::auth::repository::one_time_token::find(
        db,
        &rbs_core::token::fingerprint(&jeton),
        crate::auth::model::TokenPurpose::PasswordReset,
    )
    .await
    .expect("la lecture aboutit")
    .expect("le jeton vient d'être émis")
}

/// Une seconde de retard suffit : l'échéance est comparée à l'instant, pas au jour —
/// par `consume`, qui refuse, comme par `purge_expired`, qui retire le jeton échu et
/// garde le vivant du même compte.
///
/// Le décalage est court à dessein. Sur SQLite, l'horloge du moteur et la colonne
/// n'avaient pas le même format textuel, et toute échéance du jour passait pour future :
/// un jeton échu d'une heure y restait consommable jusqu'à minuit UTC.
///
/// Les deux portes dans un seul test : la purge est globale, et jouée en parallèle d'un
/// test qui vient d'émettre un jeton échu, elle le lui retirerait avant qu'il l'ait relu.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_expired_reset_token_is_refused_by_consume_and_removed_by_the_purge() {
    let db = connection().await;
    let compte = registered_user(&db).await;

    let echu = reset_token_expiring_in(&db, compte.id, -chrono::Duration::seconds(1)).await;
    let vivant = reset_token_expiring_in(&db, compte.id, chrono::Duration::hours(1)).await;

    let consomme = crate::auth::repository::one_time_token::consume(&db, echu.id)
        .await
        .expect("pas d'erreur");
    assert!(!consomme, "un jeton échu a été consommé");

    crate::auth::repository::one_time_token::purge_expired(&db)
        .await
        .expect("la purge aboutit");

    let restants: Vec<Uuid> = crate::auth::model::one_time_token::Entity::find()
        .filter(crate::auth::model::one_time_token::Column::UserId.eq(compte.id))
        .all(&db)
        .await
        .expect("la lecture aboutit")
        .into_iter()
        .map(|ligne| ligne.id)
        .collect();

    assert_eq!(
        restants,
        vec![vivant.id],
        "le jeton échu {} devait seul disparaître",
        echu.id
    );
}

/// Le parcours nominal : la paire rendue est utilisable, et l'ancienne ne l'est plus.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_password_returns_a_usable_pair_and_closes_the_others() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            premiere["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": PASSWORD,
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK, "corps : {corps}");

    // La nouvelle paire d'abord, puis l'ancienne : l'ordre n'importe plus depuis qu'un
    // jeton fermé rejoué ne ferme rien d'autre, mais lire le succès avant le refus est ce
    // qu'un lecteur attend.

    // Celle que le changement vient de rendre tourne.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": corps["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);

    // Et son jeton d'accès ouvre : émis dans la seconde même de la révocation, il n'est
    // pas né mort — c'est le plancher d'`issue` qui le garantit.
    let (ouvre, profil) = call(
        &api,
        get_authenticated(
            "/auth/me",
            corps["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(ouvre, StatusCode::OK, "{profil}");

    // L'ancienne session est fermée : son jeton de rafraîchissement ne tourne plus.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": premiere["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);
}

/// Une boîte compromise a pu recevoir une demande de réinitialisation d'un attaquant :
/// le titulaire qui reprend la main par `/auth/change-password` ferme ce lien-là, et pas
/// seulement ses sessions.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_password_closes_pending_reset_links() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, corps) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            paire["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": PASSWORD,
                "new_password": "un mot de passe repris en main",
            }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::OK, "corps : {corps}");

    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "le mot de passe de l'attaquant" }),
        ),
    )
    .await;
    assert_eq!(
        rejet,
        StatusCode::UNAUTHORIZED,
        "le lien de réinitialisation aurait dû être fermé par le changement de mot de passe"
    );
}

/// Un mot de passe courant faux rend 403 et non 401 : l'appelant est identifié, son
/// Bearer est bon. Un 401 lui dirait que son jeton est mort et déclencherait un
/// rafraîchissement inutile.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_wrong_current_password_is_forbidden_not_unauthorized() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            paire["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": "ce n'est pas le bon mot de passe",
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::FORBIDDEN);
}

/// Le parcours entier, jeton compris.
///
/// Le test passe par la couche service pour l'émission : la base ne garde que
/// l'empreinte, et aucune lecture ne rendrait le jeton en clair que le courriel porte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_reset_token_sets_a_new_password_and_closes_every_session() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "corps : {corps}");

    // Le nouveau mot de passe ouvre, l'ancien non.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);

    let (refuse, _) = call(
        &api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    // Et la session ouverte avant la réinitialisation est tombée.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": paire["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);
}

/// Le jeton d'accès de l'attaquant tombe avec les sessions : « toutes révoquées » ne
/// voulait rien dire tant qu'un Bearer émis avant restait bon un quart d'heure.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_access_token_issued_before_a_reset_is_refused() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();
    register(&api, &email).await;
    let avant = login(&api, &email, PASSWORD).await;
    let acces = avant["access_token"].as_str().expect("jeton d'accès");

    let (ok, _) = call(&api, get_authenticated("/auth/me", acces)).await;
    assert_eq!(ok, StatusCode::OK);

    // Le jeton de réinitialisation est tiré par le service, comme les tests voisins.
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "un autre mot de passe assez long" }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "{corps}");

    let (refus, corps) = call(&api, get_authenticated("/auth/me", acces)).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{corps}");
}

/// Le même jeton, deux fois : la seconde est refusée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_reset_token_serves_once() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let corps = json!({ "token": jeton, "new_password": "un quatrieme mot de passe long" });

    let (premier, _) = call(&api, post_json("/auth/reset-password", corps.clone())).await;
    assert_eq!(premier, StatusCode::NO_CONTENT);

    let (second, _) = call(&api, post_json("/auth/reset-password", corps)).await;
    assert_eq!(second, StatusCode::UNAUTHORIZED);
}

/// Une seconde demande ferme la première : un lien parti dans une boîte qu'on ne
/// contrôle plus cesse de valoir dès qu'on en redemande un.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_new_request_invalidates_the_previous_link() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (compte, premier) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    let (_, second) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    // La fermeture du premier jeton ne l'efface pas : les deux lignes existent encore,
    // une seule reste consommable. Trois et non deux : `register` ouvre elle-même un
    // jeton de vérification, que ce compteur ne distingue pas par usage.
    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        3,
        "la fermeture du premier jeton ne doit pas en effacer la ligne"
    );

    let (refuse, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": premier, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": second, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::NO_CONTENT);
}

/// Une adresse qu'aucun compte ne porte rend 202 et n'écrit rien : distinguer les deux
/// cas ferait de cette route l'oracle d'énumération que le hash témoin de `login` écarte
/// de l'autre côté.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn forgetting_an_unknown_address_is_accepted_and_writes_nothing() {
    let api = application().await;
    let db = connection().await;
    let inconnue = fresh_email();

    let (statut, _) = call(
        &api,
        post_json("/auth/forgot-password", json!({ "email": inconnue })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);

    // Aucun compte ne porte l'adresse, donc aucun jeton n'a pu être émis : l'émission
    // part de `find_by_email`, et rien d'autre n'y mène.
    assert!(
        crate::auth::repository::find_by_email(&db, &inconnue)
            .await
            .expect("la lecture aboutit")
            .is_none(),
        "la demande a créé un compte"
    );
}
