use super::*;

use std::time::Instant;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// L'inscription rend 202 sans corps, et le compte existe aussitôt : un client se
/// connecte sans attendre.
///
/// Sans corps, ni le hash ni le mot de passe reçu n'ont de chemin vers le client.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn registration_returns_202_without_a_body() {
    let api = application().await;
    let email = fresh_email();

    let (status, body) = register(&api, &email).await;

    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body, Value::Null, "l'inscription rend un corps : {body}");
    assert_eq!(account(&email).await.email, email);
    verified(&email).await;
    login(&api, &email, PASSWORD).await;
}

/// Une adresse prise rend le même 202 qu'une adresse neuve, et n'inscrit rien : un statut
/// ou un corps qui différerait dirait à qui essaie plusieurs adresses lesquelles sont
/// inscrites.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_taken_address_returns_the_same_202_and_creates_nothing() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    let (premier, corps_premier) = register(&api, &email).await;
    let (second, corps_second) = register(&api, &email).await;

    assert_eq!(premier, StatusCode::ACCEPTED, "{corps_premier}");
    assert_eq!(second, StatusCode::ACCEPTED, "{corps_second}");
    assert_eq!(
        corps_premier, corps_second,
        "les deux réponses se distinguent"
    );

    let comptes = crate::auth::model::user::Entity::find()
        .filter(crate::auth::model::user::Column::Email.eq(&email))
        .count(&db)
        .await
        .expect("le comptage aboutit");
    assert_eq!(comptes, 1, "l'adresse prise a inscrit un second compte");
}

/// Une seconde inscription ne touche pas au compte : un tiers qui connaît l'adresse ne
/// remplace pas le mot de passe de son titulaire par le sien.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_taken_address_keeps_its_password() {
    const AUTRE: &str = "le mot de passe d'un tiers";

    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;

    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/register",
            json!({ "email": email, "password": AUTRE }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");

    login(&api, &email, PASSWORD).await;
    let (refus, corps) = authenticate(&api, &email, AUTRE).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{corps}");
}

/// Le temps de réponse ne dit pas davantage que le statut.
///
/// Sans hachage sur le chemin « adresse prise », la réponse tomberait en quelques
/// millisecondes là où Argon2 en coûte des dizaines. Le rapport toléré est large, pour la
/// même raison qu'à la connexion.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_taken_address_costs_the_same_time_as_a_new_one() {
    let api = application().await;
    let prise = fresh_email();
    register(&api, &prise).await;

    let inscription = |email: String| {
        post_json(
            "/auth/register",
            json!({ "email": email, "password": PASSWORD }),
        )
    };

    let depart = Instant::now();
    call(&api, inscription(fresh_email())).await;
    let neuve = depart.elapsed();

    let depart = Instant::now();
    call(&api, inscription(prise)).await;
    let deja_prise = depart.elapsed();

    assert!(
        deja_prise * 5 >= neuve,
        "une adresse prise répond en {deja_prise:?} contre {neuve:?} pour une adresse \
         neuve : l'écart énumère les comptes"
    );
}

/// La casse ne fait pas deux comptes : sans cela, l'attaquant inscrit `Victime@ex.fr`,
/// la victime clique le lien de vérification qu'elle reçoit, et le compte de l'attaquant
/// porte son adresse, vérifiée.
///
/// Les blancs, eux, n'atteignent pas la route : `#[validate(email)]` les refuse avant le
/// service. Le test unitaire de `normalise`, plus bas, couvre ce que la route ne peut pas
/// montrer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn registration_lowercases_the_address() {
    let api = application().await;
    let base = fresh_email();

    let db = connection().await;

    let (status, body) = register(&api, &base.to_uppercase()).await;

    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert!(
        crate::auth::repository::find_by_email(&db, &base)
            .await
            .expect("la lecture aboutit")
            .is_some(),
        "l'adresse n'est pas inscrite en minuscules"
    );
}

/// `registration_enabled = false` ferme la route elle-même, et non le seul écran : la
/// réponse est un 403 nommé, et aucun compte n'est inscrit.
///
/// Un interrupteur qui n'aurait masqué qu'un bouton aurait fermé l'inscription pour les
/// navigateurs et pour eux seuls.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_closed_registration_refuses_the_route_itself() {
    let api = configured(|flows| flows.registration_enabled = false).await;
    let db = connection().await;
    let email = fresh_email();

    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/register",
            json!({ "email": email, "password": PASSWORD }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::FORBIDDEN, "{corps}");
    assert_eq!(
        corps["title"], "registration_closed",
        "le refus ne se distingue pas d'un autre 403 : {corps}"
    );
    assert!(
        crate::auth::repository::find_by_email(&db, &email)
            .await
            .expect("la lecture aboutit")
            .is_none(),
        "une inscription fermée a tout de même inscrit"
    );
}

/// L'écran ne devine pas l'état de l'interrupteur : une route le dit, et c'est la même
/// valeur que celle qui ferme l'inscription.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_registration_status_says_what_the_route_will_do() {
    for ouverte in [true, false] {
        let api = configured(|flows| flows.registration_enabled = ouverte).await;

        let (statut, corps) = call(&api, without_body("GET", "/auth/registration")).await;

        assert_eq!(statut, StatusCode::OK, "{corps}");
        assert_eq!(corps["enabled"], ouverte, "{corps}");
    }
}

/// Ce que la base voit d'une adresse : ni casse ni blancs, quel que soit le parcours
/// qui la reçoit.
#[test]
fn an_address_is_trimmed_and_lowercased_before_the_table() {
    assert_eq!(
        crate::auth::service::normalise("  Victime@Exemple.TEST \n"),
        "victime@exemple.test"
    );
}

/// La même adresse dans une autre casse est la même adresse : le même 202, et aucun
/// second compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_address_taken_in_another_case_is_the_same_account() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (status, body) = register(&api, &email.to_uppercase()).await;

    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    for (adresse, attendus) in [(email.clone(), 1), (email.to_uppercase(), 0)] {
        let comptes = crate::auth::model::user::Entity::find()
            .filter(crate::auth::model::user::Column::Email.eq(&adresse))
            .count(&db)
            .await
            .expect("le comptage aboutit");
        assert_eq!(comptes, attendus, "comptes inscrits sous `{adresse}`");
    }
}
