use rbs_core::{Cursor, Error, Pagination, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::Expr;
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

use super::filter::{self, ArticleFilter};
use super::model::{Column, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::{ActiveModel, Model};

pub async fn list(db: &DatabaseConnection, cursor: &Cursor) -> Result<Vec<Model>> {
    // Deux chemins de lecture, et c'est voulu : un curseur sur l'`id` n'a de sens que
    // trié sur l'`id`, quand le filtre accepte tout tri. La route de filtre garde donc
    // sa pagination par page.
    let mut requete = Entity::find().filter(Column::DeletedAt.is_null());
    if let Some(after) = cursor.after() {
        requete = requete.filter(Column::Id.lt(after));
    }

    Ok(requete
        .order_by_desc(Column::Id)
        .limit(cursor.per_page())
        .all(db)
        .await?)
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &ArticleFilter,
    pagination: &Pagination,
) -> Result<(Vec<Model>, u64)> {
    // Le filtre s'applique à ce qui reste : une ligne supprimée n'est plus une ligne que
    // l'API connaisse, et la faire réapparaître par un filtre serait une fuite.
    let requete = filter::apply(Entity::find().filter(Column::DeletedAt.is_null()), filtre)?;

    let page = requete
        .clone()
        .offset(pagination.offset())
        .limit(pagination.per_page())
        .all(db);

    // Le total compte les lignes que le filtre retient : l'attendre avant la page ferait
    // deux allers-retours en série à chaque appel. Les deux partent donc ensemble —
    // `max_connections` vaut 10 dans config/default.toml, le pool en sert bien deux.
    let (articles, total) = tokio::try_join!(page, requete.count(db))?;

    Ok((articles, total))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id)
        .filter(Column::DeletedAt.is_null())
        .one(db)
        .await?)
}

pub async fn create(db: &DatabaseConnection, article: ActiveModel) -> Result<Model> {
    article.insert(db).await.map_err(conflict_on_duplicate)
}

pub async fn update(db: &DatabaseConnection, article: ActiveModel) -> Result<Model> {
    article.update(db).await.map_err(conflict_on_duplicate)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    // La ligne déjà supprimée n'est pas retouchée : sans cette seconde condition, un
    // second DELETE rendrait 204 là où il doit rendre 404.
    let effet = Entity::update_many()
        .col_expr(Column::DeletedAt, Expr::value(chrono::Utc::now()))
        .filter(Column::Id.eq(id))
        .filter(Column::DeletedAt.is_null())
        .exec(db)
        .await?;

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
