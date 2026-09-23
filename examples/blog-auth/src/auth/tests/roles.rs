use super::*;

use axum::routing::get;
use rbs_core::Identity;
use sea_orm::{ActiveModelTrait, Set};

use crate::auth::guard::RequireRole;
use crate::auth::model::Role;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

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

/// Inscrit un compte, le promeut administrateur, et ouvre une session à ce titre.
///
/// La promotion passe par la base : l'inscription rend toujours un `user`, par défaut de
/// la table, et le rôle ne voyage que dans un jeton émis après coup.
// region: jeton_admin
pub(super) async fn login_as_admin(api: &Router, db: &DatabaseConnection) -> Value {
    let email = fresh_email();
    signed_up(api, &email).await;

    let compte = crate::auth::repository::find_by_email(db, &email)
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
// endregion: jeton_admin

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

/// Un rôle retiré en base cesse d'ouvrir la route à la requête suivante, pas au bout du
/// jeton : le rôle du Bearer est comparé à celui de la ligne à chaque requête.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_demoted_admin_is_refused_with_its_old_token() {
    let api = application().await;
    let db = connection().await;
    let restricted = admin_only_route().await;
    let paire = login_as_admin(&api, &db).await;
    let acces = access_for(&paire);

    let (ok, _) = call(&restricted, with_token("GET", "/reserve", &acces)).await;
    assert_eq!(ok, StatusCode::OK);

    let secret = rbs_core::Config::load()
        .expect("configuration lisible")
        .auth
        .secret;
    let sub = rbs_core::jwt::verify(&acces, &secret)
        .expect("jeton lisible")
        .sub;
    let compte =
        crate::auth::model::user::Entity::find_by_id(Uuid::parse_str(&sub).expect("sub lisible"))
            .one(&db)
            .await
            .expect("la table doit être interrogeable")
            .expect("le compte promu doit exister");
    let mut retrograde: crate::auth::model::user::ActiveModel = compte.into();
    retrograde.role = Set(Role::User);
    retrograde.update(&db).await.expect("compte rétrogradé");

    let (refus, body) = call(&restricted, with_token("GET", "/reserve", &acces)).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{body}");
}
