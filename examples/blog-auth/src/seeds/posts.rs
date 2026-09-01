//! Données de démonstration de `posts`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

use blog_auth::posts::model;

/// Insère les posts de démonstration.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id` vient d'`ActiveModelBehavior::new()`, `created_at` et `updated_at` du défaut de colonne.
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
