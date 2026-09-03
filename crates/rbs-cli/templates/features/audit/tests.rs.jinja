use rbs_core::HasCoreState;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};
use tokio::sync::{Mutex, MutexGuard};

use std::sync::OnceLock;

use super::model::{Column, Entity};
use super::{Entry, record};
use crate::state::AppState;

// Ces tests joignent la base que décrit `.env`, et sont donc `#[ignore]` : `cargo test`
// ne les lance pas, `cargo test -- --ignored` les lance contre la base du projet,
// migrations appliquées.

/// Les tests partagent l'unique table `audit_log` : ils se relaient plutôt que de se
/// voler leurs lignes.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    static VERROU: OnceLock<Mutex<()>> = OnceLock::new();

    let garde = VERROU.get_or_init(Mutex::default).lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table audit_log doit se vider");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Le critère qui justifie d'avoir mis le journal en base plutôt que dans un fichier.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_written_in_a_rolled_back_transaction_does_not_exist() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let transaction = db.begin().await.expect("transaction ouvrable");
    let id = record(
        &transaction,
        Entry::new(super::UPDATE, "posts", "annulée").actor("ada"),
    )
    .await
    .expect("l'entrée s'inscrit dans la transaction");
    transaction.rollback().await.expect("transaction annulable");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible");

    assert!(
        ligne.is_none(),
        "la trace a survécu au rollback du changement qu'elle décrit"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_reads_back_with_every_field_it_was_given() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let changes = serde_json::json!({ "title": { "from": "a", "to": "b" } });
    let id = record(
        db,
        Entry::new(super::UPDATE, "posts", "42")
            .actor("ada")
            .changes(changes.clone()),
    )
    .await
    .expect("l'entrée s'inscrit");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne inscrite doit se relire");

    assert_eq!(ligne.actor_id.as_deref(), Some("ada"));
    assert_eq!(ligne.action, "update");
    assert_eq!(ligne.entity, "posts");
    assert_eq!(ligne.entity_id, "42");
    assert_eq!(ligne.changes, changes);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_without_an_actor_stores_null_rather_than_an_empty_string() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let id = record(db, Entry::new(super::DELETE, "posts", "42"))
        .await
        .expect("l'entrée s'inscrit sans acteur");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne inscrite doit se relire");

    // La distinction porte : une chaîne vide dirait « un acteur anonyme », `NULL` dit
    // « aucune identité HTTP », ce qui est le cas d'un job ou d'un seed.
    assert!(ligne.actor_id.is_none(), "{:?}", ligne.actor_id);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_entries_of_one_row_read_back_in_order() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    for action in [super::CREATE, super::UPDATE, super::DELETE] {
        record(db, Entry::new(action, "posts", "42"))
            .await
            .expect("l'entrée s'inscrit");
    }
    // Une ligne voisine ne doit pas entrer dans l'histoire de celle-ci.
    record(db, Entry::new(super::CREATE, "posts", "43"))
        .await
        .expect("l'entrée s'inscrit");

    // Le second tri n'est pas décoratif : MySQL tronque `created_at` à la seconde, et
    // trois entrées écrites dans la même n'auraient sinon aucun ordre défini. L'UUIDv7
    // est monotone, il tranche.
    let histoire = Entity::find()
        .filter(Column::Entity.eq("posts"))
        .filter(Column::EntityId.eq("42"))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(db)
        .await
        .expect("lecture possible");

    let actions: Vec<&str> = histoire.iter().map(|ligne| ligne.action.as_str()).collect();
    assert_eq!(actions, ["create", "update", "delete"]);
}
