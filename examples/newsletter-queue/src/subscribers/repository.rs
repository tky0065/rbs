use rbs_core::{Error, Pagination, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use super::filter::{self, SubscriberFilter};
use super::model::{Column, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::{ActiveModel, Model};

pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {
    // Un seul chemin de lecture : la liste est le filtre vide, qui trie sur l'`id`
    // décroissant. Deux chemins divergeraient au premier tri ajouté.
    filter(db, &SubscriberFilter::default(), pagination).await
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &SubscriberFilter,
    pagination: &Pagination,
) -> Result<(Vec<Model>, u64)> {
    let requete = filter::apply(Entity::find(), filtre)?;

    let page = requete
        .clone()
        .offset(pagination.offset())
        .limit(pagination.per_page())
        .all(db);

    // Le total compte les lignes que le filtre retient : l'attendre avant la page ferait
    // deux allers-retours en série à chaque appel. Les deux partent donc ensemble —
    // `max_connections` vaut 10 dans config/default.toml, le pool en sert bien deux.
    let (subscribers, total) = tokio::try_join!(page, requete.count(db))?;

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
    subscriber.insert(db).await.map_err(conflict_on_duplicate)
}

pub async fn update(db: &DatabaseConnection, subscriber: ActiveModel) -> Result<Model> {
    subscriber.update(db).await.map_err(conflict_on_duplicate)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let effet = Entity::delete_by_id(id).exec(db).await?;

    Ok(effet.rows_affected > 0)
}

/// Une valeur déjà prise sur une colonne `unique` est une faute du client, pas une panne :
/// sans cette traduction, le doublon remonterait en erreur interne, donc en 500.
///
/// Le message reste générique — la base nomme la contrainte, pas la colonne. Précisez-le
/// si votre API doit dire laquelle.
fn conflict_on_duplicate(error: DbErr) -> Error {
    match error.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(_)) => {
            Error::Conflict("cette valeur est déjà prise".to_owned())
        }
        _ => Error::from(error),
    }
}
