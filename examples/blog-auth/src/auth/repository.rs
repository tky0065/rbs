use rbs_core::{Error, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::model::user::Entity;
use super::model::{refresh_token, user};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::user::Model;

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<Model>> {
    Ok(Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?)
}

/// Inscrit un utilisateur, le rôle et les horodatages venant des défauts de la table.
///
/// La violation de la contrainte d'unicité devient un conflit plutôt qu'une erreur
/// interne : sans cela, deux inscriptions simultanées de la même adresse rendraient une
/// 409 et une 500 selon celle qui gagne la course.
pub async fn create(db: &DatabaseConnection, email: &str, password_hash: &str) -> Result<Model> {
    let nouveau = user::ActiveModel {
        email: Set(email.to_owned()),
        password_hash: Set(password_hash.to_owned()),
        ..Default::default()
    };

    nouveau
        .insert(db)
        .await
        .map_err(|erreur| match erreur.sql_err() {
            Some(SqlErr::UniqueConstraintViolation(_)) => {
                Error::Conflict(format!("l'adresse {email} est déjà inscrite"))
            }
            _ => Error::from(erreur),
        })
}

/// Ouvre une session de rafraîchissement.
///
/// `empreinte` et non le jeton : c'est ce que la table doit porter pour qu'une base lue
/// par un tiers ne lui donne aucune session utilisable.
pub async fn create_refresh_token(
    db: &DatabaseConnection,
    user_id: Uuid,
    empreinte: String,
    expire_a: DateTimeWithTimeZone,
) -> Result<()> {
    refresh_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(empreinte),
        expires_at: Set(expire_a),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn find_refresh_token(
    db: &DatabaseConnection,
    empreinte: &str,
) -> Result<Option<refresh_token::Model>> {
    Ok(refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(empreinte))
        .one(db)
        .await?)
}

/// Consomme une session, et dit si c'est bien cet appel qui l'a fait.
///
/// L'`UPDATE` porte sa propre condition plutôt que de suivre une lecture : deux
/// rafraîchissements simultanés du même jeton franchiraient tous deux la lecture avant
/// que l'un ait écrit, et repartiraient chacun avec une paire valide.
pub async fn consommer(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(refresh_token::Column::RevokedAt, Expr::current_timestamp())
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
