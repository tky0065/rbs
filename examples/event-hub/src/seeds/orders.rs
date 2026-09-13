//! Données de démonstration de `orders`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

use event_hub::orders::model;

/// Insère les orders de démonstration.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id` vient d'`ActiveModelBehavior::new()`, `created_at` et `updated_at` du défaut de colonne.
    model::ActiveModel {
        reference: Set("reference-1".to_owned()),
        amount: Set(42),
        ..Default::default()
    }
    .insert(db)
    .await?;

    model::ActiveModel {
        reference: Set("reference-2".to_owned()),
        amount: Set(43),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
