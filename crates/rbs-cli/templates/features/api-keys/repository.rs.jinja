use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, Set,
};

use super::model::{ActiveModel, Column, Entity};
use crate::auth::model::Role;

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::Model;

/// La clé que porte cette empreinte, si elle est encore bonne à `maintenant`.
///
/// Révocation et péremption sont **dans la condition** et non dans une vérification qui
/// suivrait la lecture : une clé révoquée entre les deux ne doit pas ouvrir la requête en
/// cours, et une condition portée par le SQL ne laisse pas cette fenêtre.
pub async fn active<C: ConnectionTrait>(
    db: &C,
    fingerprint: &str,
    maintenant: DateTimeWithTimeZone,
) -> Result<Option<Model>> {
    Ok(Entity::find()
        .filter(Column::TokenHash.eq(fingerprint))
        .filter(Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(Column::ExpiresAt.is_null())
                .add(Column::ExpiresAt.gt(maintenant)),
        )
        .one(db)
        .await?)
}

/// Inscrit une clé. L'empreinte seule est écrite ; la clé est déjà partie vers l'appelant.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    user_id: Uuid,
    name: &str,
    prefix: &str,
    fingerprint: &str,
    role: Role,
    expires_at: Option<DateTimeWithTimeZone>,
) -> Result<Model> {
    Ok(ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_owned()),
        prefix: Set(prefix.to_owned()),
        token_hash: Set(fingerprint.to_owned()),
        role: Set(role),
        expires_at: Set(expires_at),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

/// Les clés d'un compte, révoquées comprises : la révocation est datée pour rester lisible.
pub async fn list_for_user<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await?)
}

/// Révoque une clé **du porteur donné**, et dit si c'est bien cet appel qui l'a fait.
///
/// `user_id` est dans la condition : sans lui, l'identifiant d'une clé d'autrui suffirait
/// à la couper. Le contrôleur rend 404 sur un `false`, jamais 403 — un 403 confirmerait
/// que la clé existe.
pub async fn revoke<C: ConnectionTrait>(db: &C, id: Uuid, user_id: Uuid) -> Result<bool> {
    let touchees = Entity::update_many()
        .col_expr(
            Column::RevokedAt,
            Expr::value(chrono::Utc::now().fixed_offset()),
        )
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}

/// Révoque toutes les clés encore vivantes d'un compte, et dit combien.
pub async fn revoke_all<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<u64> {
    let touchees = Entity::update_many()
        .col_expr(
            Column::RevokedAt,
            Expr::value(chrono::Utc::now().fixed_offset()),
        )
        .filter(Column::UserId.eq(user_id))
        .filter(Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}

/// Marque la clé comme ayant servi à `maintenant`.
///
/// La connexion est prise **par valeur**, là où toutes ses voisines l'empruntent : l'appel
/// part détaché, et une tâche détachée est `'static`. `DatabaseConnection` se clone à coût
/// nul, un `Arc` interne.
///
/// La borne temporelle est **aussi** dans la condition, alors que le service l'a déjà
/// vérifiée en mémoire : deux requêtes simultanées de la même clé la franchiraient sinon
/// toutes deux, et écriraient chacune leur tour.
pub async fn touch(
    db: sea_orm::DatabaseConnection,
    id: Uuid,
    maintenant: DateTimeWithTimeZone,
    seuil: chrono::TimeDelta,
) -> Result<bool> {
    let touchees = Entity::update_many()
        .col_expr(Column::LastUsedAt, Expr::value(maintenant))
        .filter(Column::Id.eq(id))
        .filter(
            Condition::any()
                .add(Column::LastUsedAt.is_null())
                .add(Column::LastUsedAt.lt(maintenant - seuil)),
        )
        .exec(&db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
