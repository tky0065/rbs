use rbs_core::{Pagination, Result};
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder, QuerySelect,
};

use super::model::{Column, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::{ActiveModel, Model};

pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {
    let total = Entity::find().count(db).await?;
    // L'`id` est un UUIDv7 : son ordre est celui des insertions. Trier dessus donne une
    // liste du plus récent au plus ancien, et une pagination stable, sans colonne de plus.
    let uploads = Entity::find()
        .order_by_desc(Column::Id)
        .offset(pagination.offset())
        .limit(pagination.per_page())
        .all(db)
        .await?;

    Ok((uploads, total))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

pub async fn create(db: &DatabaseConnection, upload: ActiveModel) -> Result<Model> {
    Ok(upload.insert(db).await?)
}

pub async fn update(db: &DatabaseConnection, upload: ActiveModel) -> Result<Model> {
    Ok(upload.update(db).await?)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let effet = Entity::delete_by_id(id).exec(db).await?;

    Ok(effet.rows_affected > 0)
}
