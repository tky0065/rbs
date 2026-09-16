use super::*;

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

    signed_up(&api, &email).await;
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
    signed_up(&api, &email).await;
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

/// Une adresse inscrite rend le même 202 qu'une adresse inconnue, et un jeton est émis.
///
/// C'est la branche qui envoie le courriel : un échec d'envoi y rendrait un 500 que
/// l'adresse inconnue ne rend jamais, et l'écart dirait lesquelles sont inscrites.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn forgetting_a_registered_address_is_accepted_and_opens_a_token() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");
    let avant = one_time_tokens_count_for(&db, compte.id).await;

    let (statut, _) = call(
        &api,
        post_json("/auth/forgot-password", json!({ "email": email })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);
    let (db, id) = (&db, compte.id);
    assert!(
        eventually(move || async move { one_time_tokens_count_for(db, id).await == avant + 1 })
            .await,
        "la demande n'a ouvert aucun jeton de réinitialisation"
    );
}
