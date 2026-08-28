use rbs_core::{Pagination, Result};
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use super::model::{Column, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::{ActiveModel, Model};

pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {
    let total = Entity::find().count(db).await?;
    // L'`id` est un UUIDv7 : son ordre est celui des insertions. Trier dessus donne une
    // liste du plus récent au plus ancien, et une pagination stable, sans colonne de plus.
    let subscribers = Entity::find()
        .order_by_desc(Column::Id)
        .offset(pagination.offset())
        .limit(pagination.per_page())
        .all(db)
        .await?;

    Ok((subscribers, total))
}

// region: confirmed
/// Les abonnés qui ont confirmé, dans leur entier.
///
/// Générique sur la connexion, là où les autres portes prennent la connexion elle-même :
/// la diffusion lit et enfile dans une même transaction, et une transaction n'est pas un
/// `DatabaseConnection`. Aucune pagination — l'appelant veut la liste entière, et c'est
/// aussi ce qui dit qu'une liste d'abonnés très longue demandera un autre découpage.
pub async fn confirmed<C: ConnectionTrait>(db: &C) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::Confirmed.eq(true))
        .order_by_asc(Column::Id)
        .all(db)
        .await?)
}
// endregion: confirmed

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

pub async fn create(db: &DatabaseConnection, subscriber: ActiveModel) -> Result<Model> {
    Ok(subscriber.insert(db).await?)
}

pub async fn update(db: &DatabaseConnection, subscriber: ActiveModel) -> Result<Model> {
    Ok(subscriber.update(db).await?)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let effet = Entity::delete_by_id(id).exec(db).await?;

    Ok(effet.rows_affected > 0)
}
