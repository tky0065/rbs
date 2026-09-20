use chrono::Utc;
use rbs_core::{Error, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::super::model::user::{self, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::super::model::user::Model;

pub async fn find(db: &impl ConnectionTrait, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

pub async fn find_by_email(db: &impl ConnectionTrait, email: &str) -> Result<Option<Model>> {
    Ok(Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await?)
}

/// Inscrit un utilisateur, le rôle et les horodatages venant des défauts de la table.
///
/// `None` quand l'adresse est déjà prise. Deux inscriptions simultanées de la même
/// adresse franchissent toutes deux la lecture qui précède, et seule la contrainte
/// d'unicité les départage : la perdante n'est pas une erreur interne, elle reçoit la
/// même réponse que la gagnante.
pub async fn create(
    db: &impl ConnectionTrait,
    email: &str,
    password_hash: &str,
) -> Result<Option<Model>> {
    let nouveau = user::ActiveModel {
        email: Set(email.to_owned()),
        password_hash: Set(password_hash.to_owned()),
        ..Default::default()
    };

    match nouveau.insert(db).await {
        Ok(cree) => Ok(Some(cree)),
        Err(error) if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
            Ok(None)
        }
        Err(error) => Err(Error::from(error)),
    }
}

/// Remplace le hash du mot de passe.
///
/// L'`UPDATE` ne touche que cette colonne : charger le modèle pour le réécrire en entier
/// écraserait ce qu'une autre requête a changé entre-temps.
pub async fn set_password(db: &impl ConnectionTrait, id: Uuid, hash: &str) -> Result<()> {
    Entity::update_many()
        .col_expr(user::Column::PasswordHash, Expr::value(hash))
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(())
}

/// Remplace l'adresse et retire du même coup la preuve, qui portait sur l'ancienne.
///
/// `false` quand l'adresse est déjà prise, comme `create` : la lecture qui précède ne
/// suffit pas — deux changements simultanés vers la même adresse la franchissent tous
/// deux, et seule la contrainte d'unicité les départage. La perdante n'est pas une erreur
/// interne, elle reçoit la même réponse que la gagnante.
///
/// Les deux colonnes dans le même `UPDATE` : une adresse changée qui garderait la date de
/// l'ancienne serait une adresse prouvée que personne n'a prouvée.
pub async fn set_email(db: &impl ConnectionTrait, id: Uuid, email: &str) -> Result<bool> {
    let ecriture = Entity::update_many()
        .col_expr(user::Column::Email, Expr::value(email))
        .col_expr(
            user::Column::EmailVerifiedAt,
            Expr::value(None::<DateTimeWithTimeZone>),
        )
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await;

    match ecriture {
        Ok(_) => Ok(true),
        Err(error) if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
            Ok(false)
        }
        Err(error) => Err(Error::from(error)),
    }
}

/// Date la vérification de l'adresse, si elle ne l'est pas déjà.
///
/// La date et non un booléen : savoir *quand* une adresse a été prouvée est ce qui
/// permet, un jour, d'en redemander la preuve aux plus anciennes. La première preuve est
/// donc la seule qui s'écrit — la condition est dans l'`UPDATE`, pour qu'un jeton émis
/// avant la vérification et consommé après ne la rajeunisse pas non plus.
pub async fn mark_verified(db: &impl ConnectionTrait, id: Uuid) -> Result<()> {
    Entity::update_many()
        .col_expr(user::Column::EmailVerifiedAt, Expr::current_timestamp())
        .filter(user::Column::Id.eq(id))
        .filter(user::Column::EmailVerifiedAt.is_null())
        .exec(db)
        .await?;

    Ok(())
}

/// Date la fermeture de toutes les sessions, et rend l'instant écrit.
///
/// L'instant vient de Rust et se lie en paramètre, comme toute date de ces dépôts : c'est
/// à lui qu'`issue` compare le `iat` du prochain jeton, et une horloge SQL ne rendrait pas
/// la même seconde. Rendu plutôt que relu : une relecture ne dirait pas mieux que ce que
/// cet appel vient de lier.
pub async fn stamp_sessions_revoked(
    db: &impl ConnectionTrait,
    id: Uuid,
) -> Result<DateTimeWithTimeZone> {
    let maintenant = Utc::now().fixed_offset();

    Entity::update_many()
        .col_expr(user::Column::SessionsRevokedAt, Expr::value(maintenant))
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(maintenant)
}
