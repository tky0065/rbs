//! Données de démonstration de `articles`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

use hello_crud::articles::model;

/// Insère les articles de démonstration.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id`, `created_at` et `updated_at` tiennent leur valeur du défaut de la colonne.
    model::ActiveModel {
        title: Set("title-1".to_owned()),
        body: Set("body-1".to_owned()),
        published: Set(true),
        ..Default::default()
    }
    .insert(db)
    .await?;

    model::ActiveModel {
        title: Set("title-2".to_owned()),
        body: Set("body-2".to_owned()),
        published: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
