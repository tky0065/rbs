//! Données de démonstration de `uploads`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

// Le binaire des seeds est une racine de crate distincte de celle de l'application : il
// rejoint l'entité par son chemin, et n'en appelle qu'une part.
#[path = "../uploads/model.rs"]
mod model;

/// Insère les uploads de démonstration.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id`, `created_at` et `updated_at` tiennent leur valeur du défaut de la colonne.
    model::ActiveModel {
        title: Set("title-1".to_owned()),
        owner_email: Set("owner_email-1@example.com".to_owned()),
        content_type: Set("content_type-1".to_owned()),
        size: Set(42),
        ..Default::default()
    }
    .insert(db)
    .await?;

    model::ActiveModel {
        title: Set("title-2".to_owned()),
        owner_email: Set("owner_email-2@example.com".to_owned()),
        content_type: Set("content_type-2".to_owned()),
        size: Set(43),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
