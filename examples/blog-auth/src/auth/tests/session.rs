use super::*;

use std::time::Instant;

use axum::routing::get;
use chrono::Utc;
use rbs_core::Identity;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::auth::guard::RequireRole;
use crate::auth::model::{Role, refresh_token};

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// La garde tient avant même que le service existe : `Identity` refuse la requête sans
/// jamais atteindre le controller.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_without_a_token_returns_401() {
    let api = application().await;

    let (status, body) = call(&api, without_body("GET", "/auth/me")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

/// Un jeton que le service n'a pas signé ne vaut pas mieux qu'aucun jeton.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_with_an_unreadable_token_returns_401() {
    let api = application().await;
    let requete = Request::builder()
        .method("GET")
        .uri("/auth/me")
        .header("authorization", "Bearer pas-un-jeton")
        .body(Body::empty())
        .expect("requête bien formée");

    let (status, body) = call(&api, requete).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn registration_returns_201_and_the_created_profile() {
    let api = application().await;
    let email = fresh_email();

    let (status, body) = register(&api, &email).await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["email"], email, "{body}");
    assert_eq!(body["role"], "user", "{body}");
    assert!(body["id"].is_string(), "{body}");
}

/// Ni le hash ni le mot de passe reçu ne repartent vers le client.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_hash_does_not_appear_in_the_response() {
    let api = application().await;

    let (_, body) = register(&api, &fresh_email()).await;

    let texte = body.to_string();
    assert!(
        !texte.contains("$argon2"),
        "hash dans la réponse :\n{texte}"
    );
    assert!(
        !texte.contains(PASSWORD),
        "mot de passe dans la réponse :\n{texte}"
    );
    assert!(
        !texte.contains("password"),
        "champ de mot de passe dans la réponse :\n{texte}"
    );
}

/// Le 409 dit qu'il refuse, sans redire quoi.
///
/// Une réponse qui cite l'adresse la confirme à qui la soumet : l'inscription
/// deviendrait l'oracle d'énumération que la connexion s'applique à ne pas être.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_email_already_taken_returns_409_without_repeating_it() {
    let api = application().await;
    let email = fresh_email();

    let (premier, body) = register(&api, &email).await;
    assert_eq!(premier, StatusCode::CREATED, "{body}");

    let (second, body) = register(&api, &email).await;

    assert_eq!(second, StatusCode::CONFLICT, "{body}");
    assert!(
        !body.to_string().contains(&email),
        "le refus répète l'adresse soumise : {body}"
    );
}

/// Les deux échecs sont indiscernables : un corps qui différerait dirait à un attaquant
/// quelles adresses sont inscrites.
///
/// `request_id` est retiré avant la comparaison — il change à chaque requête par
/// construction, et corréler une réponse à une ligne de journal ne renseigne personne sur
/// l'existence d'un compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_wrong_password_and_an_unknown_email_return_the_same_401() {
    let api = application().await;
    let inscrit = fresh_email();
    register(&api, &inscrit).await;

    let (statut_faux, mut corps_faux) =
        authenticate(&api, &inscrit, "un tout autre mot de passe").await;
    let (statut_inconnu, mut corps_inconnu) = authenticate(&api, &fresh_email(), PASSWORD).await;

    assert_eq!(statut_faux, StatusCode::UNAUTHORIZED, "{corps_faux}");
    assert_eq!(statut_inconnu, StatusCode::UNAUTHORIZED, "{corps_inconnu}");

    for body in [&mut corps_faux, &mut corps_inconnu] {
        if let Some(objet) = body.as_object_mut() {
            objet.remove("request_id");
        }
    }

    assert_eq!(
        corps_faux, corps_inconnu,
        "les deux échecs doivent être indiscernables"
    );
}

/// Le temps de réponse ne renseigne pas davantage que le corps.
///
/// Sans vérification sur le chemin « adresse inconnue », la réponse tombe en une fraction
/// de milliseconde là où Argon2 en coûte plusieurs dizaines : cet écart seul énumère les
/// comptes. Le rapport toléré est large — il sépare l'absence de vérification, d'un ordre
/// de grandeur, d'un hachage à vide, qui coûte le même temps.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_email_costs_the_same_time_as_a_wrong_password() {
    let api = application().await;
    let inscrit = fresh_email();
    register(&api, &inscrit).await;

    // Un tour à vide : le hash de comparaison se calcule au premier passage, et son coût
    // ne doit pas être imputé à la mesure.
    authenticate(&api, &fresh_email(), PASSWORD).await;

    let depart = Instant::now();
    authenticate(&api, &inscrit, "un tout autre mot de passe").await;
    let mot_de_passe_faux = depart.elapsed();

    let depart = Instant::now();
    authenticate(&api, &fresh_email(), PASSWORD).await;
    let email_inconnu = depart.elapsed();

    assert!(
        email_inconnu * 5 >= mot_de_passe_faux,
        "une adresse inconnue répond en {email_inconnu:?} contre {mot_de_passe_faux:?} \
         pour un mot de passe faux : l'écart énumère les comptes"
    );
}

/// Inscrit une adresse neuve et ouvre une session : l'identifiant du compte et sa paire.
async fn login_as(api: &Router) -> (Uuid, Value) {
    let email = fresh_email();
    let (_, profile) = register(api, &email).await;
    let id = Uuid::parse_str(
        profile["id"]
            .as_str()
            .expect("le profil porte un identifiant"),
    )
    .expect("identifiant lisible");

    let (status, paire) = authenticate(api, &email, PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{paire}");

    (id, paire)
}

/// Le jeton de rafraîchissement d'une paire.
fn refresh_for(paire: &Value) -> String {
    paire["refresh_token"]
        .as_str()
        .expect("la paire doit porter un jeton de rafraîchissement")
        .to_owned()
}

async fn refresh(api: &Router, token: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json("/auth/refresh", json!({ "refresh_token": token })),
    )
    .await
}

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
    register(&api, &email).await;

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
    register(&api, &email).await;

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

async fn logout(api: &Router, token: &str) -> (StatusCode, Value) {
    call(
        api,
        post_json("/auth/logout", json!({ "refresh_token": token })),
    )
    .await
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn logout_returns_204() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;

    let (status, body) = logout(&api, &refresh_for(&paire)).await;

    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(body, Value::Null, "un 204 ne porte pas de corps");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_revoked_refresh_returns_401() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;
    let token = refresh_for(&paire);

    let (status, body) = logout(&api, &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");

    let (status, body) = refresh(&api, &token).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// Un jeton fermé par `logout` puis rejoué n'est pas un jeton volé : c'est un client qui
/// réessaie. Le traiter comme un rejeu fermerait toutes les sessions du compte à la
/// demande de qui tient un jeton mort — trente jours durant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_refresh_closed_by_logout_when_replayed_leaves_the_other_sessions_open() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (_, fermee) = authenticate(&api, &email, PASSWORD).await;
    let (_, vivante) = authenticate(&api, &email, PASSWORD).await;

    let (status, _) = logout(&api, &refresh_for(&fermee)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (rejeu, body) = refresh(&api, &refresh_for(&fermee)).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&vivante)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la session sœur est tombée sur le rejeu d'un jeton fermé : {body}"
    );
}

/// Se déconnecter d'un appareil ne déconnecte pas les autres.
///
/// C'est la garantie que ce lot ajoute : la révocation porte sur la ligne présentée, et
/// non sur le compte qui la détient.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_other_sessions_of_the_same_account_stay_valid() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (_, premiere) = authenticate(&api, &email, PASSWORD).await;
    let (_, seconde) = authenticate(&api, &email, PASSWORD).await;

    let (status, body) = logout(&api, &refresh_for(&premiere)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&seconde)).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "la seconde session a été fermée avec la première : {body}"
    );
}

/// Une route protégée, montée pour les tests seuls.
///
/// Le fragment n'en livre aucune : c'est à vous d'en poser sur vos propres routes, et
/// voici comment. Le handler se contente d'exiger le rôle avant de répondre.
async fn admin_only_route() -> Router {
    async fn restricted(identite: Identity) -> rbs_core::Result<StatusCode> {
        identite.require_role(Role::Admin)?;

        Ok(StatusCode::OK)
    }

    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");

    Router::new()
        .route("/reserve", get(restricted))
        .with_state(AppState::new(db, config).expect("état partagé constructible"))
}

/// Une route ouverte à tout compte authentifié, montée pour les tests seuls.
///
/// C'est la forme que `rbs generate crud` pose par défaut : le seuil le plus bas, qu'un
/// rôle plus étendu satisfait aussi.
async fn user_or_above_route() -> Router {
    async fn restricted(identite: Identity) -> rbs_core::Result<StatusCode> {
        identite.require_role(Role::User)?;

        Ok(StatusCode::OK)
    }

    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");

    Router::new()
        .route("/ouverte", get(restricted))
        .with_state(AppState::new(db, config).expect("état partagé constructible"))
}

fn with_token(methode: &str, chemin: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Le jeton d'accès d'une paire.
fn access_for(paire: &Value) -> String {
    paire["access_token"]
        .as_str()
        .expect("la paire doit porter un jeton d'accès")
        .to_owned()
}

/// Inscrit un compte, le promeut administrateur, et ouvre une session à ce titre.
///
/// La promotion passe par la base : l'inscription rend toujours un `user`, par défaut de
/// la table, et le rôle ne voyage que dans un jeton émis après coup.
async fn login_as_admin(api: &Router, db: &DatabaseConnection) -> Value {
    let email = fresh_email();
    let (_, profile) = register(api, &email).await;
    let id = Uuid::parse_str(
        profile["id"]
            .as_str()
            .expect("le profil porte un identifiant"),
    )
    .expect("identifiant lisible");

    let compte = crate::auth::model::user::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("la table doit être interrogeable")
        .expect("le compte inscrit doit exister");

    let mut promu: crate::auth::model::user::ActiveModel = compte.into();
    promu.role = Set(Role::Admin);
    promu.update(db).await.expect("compte promu");

    let (status, paire) = authenticate(api, &email, PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{paire}");

    paire
}

/// Sans jeton, la réponse dit « identifie-toi », et non « tu n'as pas le droit ».
///
/// Les deux se confondent aisément : ici c'est l'extractor `Identity` qui tranche, avant
/// que le corps du handler — et donc la garde — s'exécute.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn without_a_token_the_admin_route_returns_401() {
    let api = admin_only_route().await;

    let (status, body) = call(&api, without_body("GET", "/reserve")).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_ne!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_on_an_admin_route_returns_403() {
    let api = application().await;
    let restricted = admin_only_route().await;
    let (_, paire) = login_as(&api).await;

    let (status, body) = call(
        &restricted,
        with_token("GET", "/reserve", &access_for(&paire)),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_on_the_same_route_returns_200() {
    let api = application().await;
    let db = connection().await;
    let restricted = admin_only_route().await;
    let paire = login_as_admin(&api, &db).await;

    let (status, body) = call(
        &restricted,
        with_token("GET", "/reserve", &access_for(&paire)),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Un administrateur traverse une garde posée sur `User`.
///
/// C'est ce que le seuil promet, et ce qu'une égalité stricte refuserait — le cas est
/// exactement celui de toute route générée par `rbs generate crud` sous `auth`.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_satisfies_a_user_requirement() {
    let api = application().await;
    let db = connection().await;
    let restricted = user_or_above_route().await;
    let paire = login_as_admin(&api, &db).await;

    let (status, body) = call(
        &restricted,
        with_token("GET", "/ouverte", &access_for(&paire)),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "un admin doit satisfaire une exigence User : {body}"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn me_returns_the_callers_profile() {
    let api = application().await;
    let email = fresh_email();
    let (_, inscrit) = register(&api, &email).await;
    let (_, paire) = authenticate(&api, &email, PASSWORD).await;

    let (status, profile) = call(&api, with_token("GET", "/auth/me", &access_for(&paire))).await;

    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["id"], inscrit["id"], "{profile}");
    assert_eq!(profile["email"], email, "{profile}");
    assert_eq!(profile["role"], "user", "{profile}");
}

/// Le document tel que le serveur le publie.
async fn openapi_document(api: &Router) -> Value {
    let (status, document) = call(api, without_body("GET", "/api-docs/openapi.json")).await;

    assert_eq!(status, StatusCode::OK, "le document doit être exposé");

    document
}

/// Douze chemins, treize points d'entrée : `/auth/sessions` porte à la fois la liste et
/// la révocation globale. Un `#[utoipa::path]` qui disparaîtrait du document sans que
/// cette suite rougisse laisserait le client TypeScript, déduit de ce même document, en
/// silence sur la route perdue.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_openapi_document_carries_the_thirteen_auth_operations() {
    let api = application().await;

    let document = openapi_document(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/auth/me",
        "/auth/change-password",
        "/auth/forgot-password",
        "/auth/reset-password",
        "/auth/verify-email",
        "/auth/resend-verification",
        "/auth/sessions",
        "/auth/sessions/{id}",
    ] {
        assert!(
            document["paths"][chemin].is_object(),
            "`{chemin}` ne figure pas dans le document"
        );
    }

    for methode in ["get", "delete"] {
        assert!(
            document["paths"]["/auth/sessions"][methode].is_object(),
            "`{methode} /auth/sessions` ne figure pas dans le document"
        );
    }
}

/// Un client généré depuis ce document doit savoir comment s'authentifier, et sur quelles
/// routes le faire.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_bearer_scheme_is_declared_and_me_carries_it() {
    let api = application().await;

    let document = openapi_document(&api).await;

    let schema = &document["components"]["securitySchemes"]["bearer"];
    assert_eq!(schema["type"], "http", "{schema}");
    assert_eq!(schema["scheme"], "bearer", "{schema}");
    assert_eq!(schema["bearerFormat"], "JWT", "{schema}");

    let securite = &document["paths"]["/auth/me"]["get"]["security"];
    assert!(
        securite.is_array() && !securite.as_array().expect("tableau").is_empty(),
        "`/auth/me` ne déclare pas exiger de jeton : {securite}"
    );
    assert!(
        securite[0]["bearer"].is_array(),
        "`/auth/me` n'exige pas le schéma `bearer` : {securite}"
    );
}

/// `refresh` et `logout` s'authentifient par leur corps : leur apposer le schéma
/// décrirait une exigence que le serveur ne pose pas.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_routes_without_a_header_do_not_declare_the_scheme() {
    let api = application().await;

    let document = openapi_document(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
    ] {
        assert!(
            document["paths"][chemin]["post"]["security"].is_null(),
            "`{chemin}` déclare exiger un jeton alors qu'il n'en lit aucun"
        );
    }
}

/// La liste ne montre que les sessions du seul appelant, et jamais l'empreinte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_session_list_shows_only_the_callers_own() {
    let api = application().await;

    let mien = fresh_email();
    register(&api, &mien).await;
    let premiere = login(&api, &mien, PASSWORD).await;
    login(&api, &mien, PASSWORD).await;

    // Un second compte, dont les sessions ne doivent pas apparaître.
    let autre = fresh_email();
    register(&api, &autre).await;
    login(&api, &autre, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK);
    let sessions = corps.as_array().expect("une liste");
    assert_eq!(sessions.len(), 2, "sessions rendues : {corps}");
    assert!(
        sessions
            .iter()
            .all(|session| session.get("token_hash").is_none()),
        "la liste porte l'empreinte d'un jeton : {corps}"
    );
}

/// Révoquer une session nommée ferme celle-ci, et laisse tourner les autres du même
/// compte.
///
/// La session sœur est vérifiée **avant** de rejouer celle qu'on vient de fermer : rejouer
/// un jeton de rafraîchissement révoqué arme la défense anti-rejeu de `refresh`, qui
/// referme tout le compte — sœur comprise. Inverser l'ordre ferait échouer cette
/// assertion pour une tout autre raison que celle qu'elle éprouve.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_a_named_session_leaves_the_others_of_the_same_account_running() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let premiere = login(&api, &email, PASSWORD).await;
    let seconde = login(&api, &email, PASSWORD).await;

    let (_, liste) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    let sessions = liste.as_array().expect("une liste");
    assert_eq!(sessions.len(), 2, "{liste}");

    // `open_sessions_of` trie la plus récente en tête : c'est la session de `seconde`.
    let cible = sessions[0]["id"]
        .as_str()
        .expect("un identifiant de session");

    let (statut, _) = call(
        &api,
        delete_authenticated(
            &format!("/auth/sessions/{cible}"),
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    // La session sœur, jamais nommée par la révocation, tourne toujours — vérifié avant
    // de toucher à celle qui vient de fermer.
    let (encore, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": premiere["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(encore, StatusCode::OK, "{corps}");

    // La session fermée, elle, refuse.
    let (rejet, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": seconde["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED, "{corps}");
}

/// La session d'autrui ne se révoque pas, et rend 404 plutôt que 403 : un identifiant qui
/// n'est pas le vôtre ne désigne, de votre côté, aucune session.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_someone_elses_session_is_not_found() {
    let api = application().await;

    let victime = fresh_email();
    register(&api, &victime).await;
    let sienne = login(&api, &victime, PASSWORD).await;

    let attaquant = fresh_email();
    register(&api, &attaquant).await;
    let paire = login(&api, &attaquant, PASSWORD).await;

    let (_, liste) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            sienne["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    let cible = liste[0]["id"].as_str().expect("un identifiant de session");

    let (statut, _) = call(
        &api,
        delete_authenticated(
            &format!("/auth/sessions/{cible}"),
            paire["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);

    // La tentative a rendu 404 sans rien révoquer : ceci n'est donc pas le rejeu d'un
    // jeton déjà fermé, et la défense anti-rejeu de `refresh` ne s'arme pas ici.
    let (encore, corps) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": sienne["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(encore, StatusCode::OK, "{corps}");
}

/// La révocation globale ferme tout, y compris la session qui l'a demandée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_every_session_closes_them_all() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;
    let seconde = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        delete_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    for paire in [&premiere, &seconde] {
        let (rejet, corps) = call(
            &api,
            post_json(
                "/auth/refresh",
                json!({ "refresh_token": paire["refresh_token"] }),
            ),
        )
        .await;
        assert_eq!(rejet, StatusCode::UNAUTHORIZED, "{corps}");
    }
}
