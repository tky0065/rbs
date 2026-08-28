//! Données de démonstration de `subscribers`.

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};

// Le binaire des seeds est une racine de crate distincte de celle de l'application : il
// rejoint l'entité par son chemin, et n'en appelle qu'une part.
#[path = "../subscribers/model.rs"]
mod model;

// region: seed
/// Insère les abonnés de démonstration.
///
/// Le dernier n'a pas confirmé : `POST /subscribers/broadcast` enfile trois lettres et
/// non quatre, ce qui est la seule façon de voir que le filtre existe.
pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    // `id`, `created_at` et `updated_at` tiennent leur valeur du défaut de la colonne.
    for (email, name, confirmed) in [
        ("ada@example.com", "Ada", true),
        ("grace@example.com", "Grace", true),
        ("alan@example.com", "Alan", true),
        ("edsger@example.com", "Edsger", false),
    ] {
        model::ActiveModel {
            email: Set(email.to_owned()),
            name: Set(name.to_owned()),
            confirmed: Set(confirmed),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}
// endregion: seed
