use super::*;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set, TransactionTrait};

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
///
/// La ligne est celle que rend l'`INSERT`, sans relecture : toute émission purge les
/// jetons échus, et celle d'un test voisin retirerait celui-ci avant qu'on le relise.
async fn reset_token_expiring_in(
    db: &DatabaseConnection,
    user_id: Uuid,
    decalage: chrono::Duration,
) -> crate::auth::repository::one_time_token::Model {
    crate::auth::model::one_time_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(rbs_core::token::fingerprint(&rbs_core::token::random())),
        purpose: Set(crate::auth::model::TokenPurpose::PasswordReset),
        expires_at: Set((Utc::now() + decalage).fixed_offset()),
        created_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("le jeton s'émet")
}

/// Chaque émission purge les jetons échus, ceux de tous les comptes : sans elle, la table
/// croîtrait d'une ligne par demande sans jamais en perdre, au rythme du trafic anonyme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_emission_purges_the_expired_tokens_of_every_account() {
    let db = connection().await;
    let autre = registered_user(&db).await;
    let demandeur = registered_user(&db).await;
    let echu = reset_token_expiring_in(&db, autre.id, -chrono::Duration::seconds(1)).await;

    crate::auth::service::password::request_reset(&db, 3600, &demandeur.email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let survivant = crate::auth::model::one_time_token::Entity::find_by_id(echu.id)
        .one(&db)
        .await
        .expect("la lecture aboutit");
    assert!(
        survivant.is_none(),
        "le jeton échu d'un autre compte a survécu à une émission"
    );
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

/// Ce que `reset` enchaîne — consommer le jeton, poser le mot de passe — se défait d'un
/// bloc quand la transaction qui le porte est abandonnée : les dépôts acceptent la
/// transaction du service, et rien de ce qu'ils y ont écrit ne lui survit. C'est la
/// mécanique sur laquelle `change` et `reset` reposent pour ne pas brûler un jeton sans
/// poser le mot de passe qu'il promettait.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rolled_back_transaction_leaves_the_token_and_the_password_untouched() {
    let db = connection().await;
    let compte = registered_user(&db).await;
    let jeton = reset_token_expiring_in(&db, compte.id, chrono::Duration::hours(1)).await;

    let transaction = db.begin().await.expect("transaction ouvrable");
    let consomme = crate::auth::repository::one_time_token::consume(&transaction, jeton.id)
        .await
        .expect("pas d'erreur");
    assert!(consomme, "le jeton vient d'être émis");
    crate::auth::repository::user::set_password(&transaction, compte.id, "un hash abandonné")
        .await
        .expect("pas d'erreur");
    transaction.rollback().await.expect("transaction annulable");

    let consomme = crate::auth::repository::one_time_token::consume(&db, jeton.id)
        .await
        .expect("pas d'erreur");
    assert!(
        consomme,
        "le jeton a été consommé par une transaction annulée"
    );

    let relu = crate::auth::repository::find(&db, compte.id)
        .await
        .expect("la lecture aboutit")
        .expect("le compte existe");
    assert_eq!(
        relu.password_hash, compte.password_hash,
        "le mot de passe a été posé par une transaction annulée"
    );
}
