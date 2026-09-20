use super::*;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// `PATCH /auth/me` avec `email`, et rend statut et corps.
async fn change_address(api: &Router, jeton: &str, email: &str) -> (StatusCode, Value) {
    call(
        api,
        patch_json_authenticated("/auth/me", jeton, json!({ "email": email })),
    )
    .await
}

/// L'écriture est faite avant la réponse : l'adresse est celle du corps, et la preuve qui
/// portait sur l'ancienne est tombée.
///
/// Sans ce retrait, le compte porterait une adresse que personne n'a prouvée avec la date
/// d'une adresse que quelqu'un avait prouvée — exactement ce qu'un attaquant qui obtient
/// une session cherche à obtenir.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_address_writes_it_and_drops_the_proof() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;
    let compte = account(&email).await;
    assert!(compte.email_verified_at.is_some(), "l'adresse est prouvée");

    let neuve = fresh_email();
    let (statut, corps) = change_address(&api, &access_token_for(&compte), &neuve).await;

    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");
    assert_eq!(corps, Value::Null, "le changement rend un corps : {corps}");

    let relu = account(&neuve).await;
    assert_eq!(relu.id, compte.id, "l'adresse est allée à un autre compte");
    assert!(
        relu.email_verified_at.is_none(),
        "la preuve de l'ancienne adresse a suivi la nouvelle"
    );
    assert!(
        crate::auth::repository::find_by_email(&connection().await, &email)
            .await
            .expect("la lecture aboutit")
            .is_none(),
        "l'ancienne adresse est restée inscrite"
    );
}

/// Le lien de vérification part vers la nouvelle adresse : sans lui, le compte ne pourrait
/// plus jamais prouver l'adresse qu'il vient de se donner.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_address_opens_a_verification_token() {
    let api = application().await;
    let db = connection().await;
    let (compte, jeton) = account_with_token(&api).await;

    let (statut, corps) = change_address(&api, &jeton, &fresh_email()).await;
    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");

    let (db, id) = (&db, compte.id);
    assert!(
        eventually(move || async move { one_time_tokens_count_for(db, id).await > 1 }).await,
        "le changement d'adresse n'a ouvert aucun jeton de vérification"
    );
}

/// Une adresse prise rend le même 202 qu'une adresse libre, et ne touche à rien : un
/// statut ou un corps qui différerait ferait d'un simple compte un moyen de savoir
/// quelles adresses le service connaît.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_taken_address_returns_the_same_202_and_changes_nothing() {
    let api = application().await;

    let occupee = fresh_email();
    signed_up(&api, &occupee).await;

    let sienne = fresh_email();
    signed_up(&api, &sienne).await;
    let compte = account(&sienne).await;
    let jeton = access_token_for(&compte);

    let (libre, corps_libre) = change_address(&api, &jeton, &fresh_email()).await;
    let (prise, corps_prise) = change_address(&api, &jeton, &occupee).await;

    assert_eq!(libre, StatusCode::ACCEPTED, "{corps_libre}");
    assert_eq!(prise, StatusCode::ACCEPTED, "{corps_prise}");
    assert_eq!(corps_libre, corps_prise, "les deux réponses se distinguent");

    let titulaire = account(&occupee).await;
    assert!(
        titulaire.email_verified_at.is_some(),
        "la tentative a déplacé la preuve du titulaire"
    );
    assert_ne!(titulaire.id, compte.id, "l'adresse a changé de compte");
}

/// La sienne ne change rien, et surtout ne retire pas une preuve déjà acquise pour la
/// redemander aussitôt.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_address_it_already_has_keeps_its_proof() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;
    let compte = account(&email).await;

    let (statut, corps) = change_address(&api, &access_token_for(&compte), &email).await;

    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");
    assert!(
        account(&email).await.email_verified_at.is_some(),
        "la preuve est tombée pour une adresse qui n'a pas bougé"
    );
}

/// La casse ne fait pas deux comptes ici non plus : la table voit l'adresse normalisée,
/// quel que soit le parcours qui la reçoit.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_new_address_reaches_the_table_in_lowercase() {
    let api = application().await;
    let (compte, jeton) = account_with_token(&api).await;
    let neuve = fresh_email();

    let (statut, corps) = change_address(&api, &jeton, &neuve.to_uppercase()).await;
    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");

    assert_eq!(
        account(&neuve).await.id,
        compte.id,
        "l'adresse n'est pas inscrite en minuscules"
    );
}

/// La route est une écriture du compte de l'appelant : sans jeton, elle ne désigne aucun
/// compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn changing_the_address_without_a_token_is_refused() {
    let api = application().await;

    let (statut, corps) = call(
        &api,
        Request::builder()
            .method("PATCH")
            .uri("/auth/me")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "email": fresh_email() }).to_string()))
            .expect("requête bien formée"),
    )
    .await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
}

/// Le rôle et la date de vérification n'ont aucun chemin d'écriture : le DTO ne porte que
/// l'adresse, et ce qui l'accompagne est ignoré plutôt qu'appliqué.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_route_writes_nothing_but_the_address() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;
    let compte = account(&email).await;
    let neuve = fresh_email();

    let (statut, corps) = call(
        &api,
        patch_json_authenticated(
            "/auth/me",
            &access_token_for(&compte),
            json!({
                "email": neuve,
                "role": "admin",
                "email_verified_at": "2020-01-01T00:00:00Z",
            }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::ACCEPTED, "{corps}");

    let relu = account(&neuve).await;
    assert_eq!(relu.role, compte.role, "le rôle a suivi le corps");
    assert!(
        relu.email_verified_at.is_none(),
        "la date de vérification a suivi le corps"
    );
}

/// Une adresse qui n'en est pas une n'atteint pas le service : `#[validate(email)]` la
/// refuse, et le compte ne bouge pas.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_address_that_is_not_one_leaves_the_account_alone() {
    let api = application().await;
    let email = fresh_email();
    signed_up(&api, &email).await;
    let compte = account(&email).await;

    let (statut, corps) = change_address(&api, &access_token_for(&compte), "pas une adresse").await;

    assert_eq!(statut, StatusCode::UNPROCESSABLE_ENTITY, "{corps}");
    assert_eq!(
        account(&email).await.email_verified_at,
        compte.email_verified_at,
        "une entrée refusée a touché le compte"
    );
}
