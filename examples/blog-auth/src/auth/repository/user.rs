use rbs_core::{Error, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::super::model::user::{self, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::super::model::user::Model;

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<Model>> {
    Ok(Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?)
}

/// Le refus d'une adresse déjà prise, dit sans la répéter.
///
/// Citer l'adresse la confirme à qui la soumet, dans la réponse comme dans le journal :
/// l'inscription deviendrait l'oracle d'énumération que le hash témoin de `login` écarte
/// de l'autre côté.
pub const ADRESSE_PRISE: &str = "cette adresse est déjà inscrite";

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
        .map_err(|error| match error.sql_err() {
            Some(SqlErr::UniqueConstraintViolation(_)) => Error::Conflict(ADRESSE_PRISE.to_owned()),
            _ => Error::from(error),
        })
}
