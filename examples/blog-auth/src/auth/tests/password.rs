use super::*;

use chrono::Utc;

/// Deux consommations simultanées du même jeton : une seule passe.
///
/// C'est la garantie que la condition portée par l'`UPDATE` achète, et qu'une lecture
/// suivie d'une écriture ne donnerait pas — les deux franchiraient la lecture avant que
/// l'une ait écrit, et poseraient chacune leur mot de passe.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
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
