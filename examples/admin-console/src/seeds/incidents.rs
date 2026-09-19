//! Données de démonstration de `incidents`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

use admin_console::incidents::model;

/// Insère les incidents de démonstration.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id` vient d'`ActiveModelBehavior::new()`, `created_at` et `updated_at` du défaut de colonne.
    model::ActiveModel {
        reference: Set("reference-1".to_owned()),
        sujet: Set("sujet-1".to_owned()),
        detail: Set("detail-1".to_owned()),
        gravite: Set(model::IncidentGravite::Basse),
        ouvert: Set(true),
        duree_minutes: Set(Some(42)),
        echeance: Set(Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())),
        constate_le: Set(chrono::Utc::now().into()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    model::ActiveModel {
        reference: Set("reference-2".to_owned()),
        sujet: Set("sujet-2".to_owned()),
        detail: Set("detail-2".to_owned()),
        gravite: Set(model::IncidentGravite::Moyenne),
        ouvert: Set(false),
        duree_minutes: Set(Some(43)),
        echeance: Set(Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 2).unwrap())),
        constate_le: Set(chrono::Utc::now().into()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}
