use chrono::Utc;
use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::super::model::{TokenPurpose, one_time_token};

pub use one_time_token::Model;

// L'instant vient de Rust et se lie en paramètre, jamais de `CURRENT_TIMESTAMP` : sqlx
// écrit la colonne en RFC 3339 (`2026-01-01T…+00:00`) là où l'horloge de SQLite rend
// `2026-01-01 23:59:59`, et SQLite compare le texte — `'T' > ' '`, toute échéance du jour
// passait pour future. L'écriture de `consumed_at` suit la même règle pour que la colonne
// ne mélange pas deux formats.

/// Ouvre un jeton à usage unique.
///
/// `fingerprint` et non le jeton : une base lue par un tiers ne lui donne aucun lien
/// qu'il puisse jouer.
pub async fn issue(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    purpose: TokenPurpose,
    fingerprint: String,
    expires_at: DateTimeWithTimeZone,
) -> Result<()> {
    one_time_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(fingerprint),
        purpose: Set(purpose),
        expires_at: Set(expires_at),
        created_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

/// Retrouve un jeton par son empreinte **et son usage**.
///
/// L'usage fait partie de la recherche : sans lui, un jeton de vérification — plus long à
/// périmer, et envoyé à toute inscription — vaudrait comme jeton de réinitialisation.
pub async fn find(
    db: &impl ConnectionTrait,
    fingerprint: &str,
    purpose: TokenPurpose,
) -> Result<Option<Model>> {
    Ok(one_time_token::Entity::find()
        .filter(one_time_token::Column::TokenHash.eq(fingerprint))
        .filter(one_time_token::Column::Purpose.eq(purpose))
        .one(db)
        .await?)
}

/// Consomme un jeton, et dit si c'est bien cet appel qui l'a fait.
///
/// La péremption est **dans** la condition de l'`UPDATE` et non dans la lecture qui le
/// précède : deux réinitialisations concurrentes du même jeton franchiraient sinon toutes
/// deux la lecture, et poseraient chacune leur mot de passe — la seconde gagnant sans que
/// la première le sache.
pub async fn consume(db: &impl ConnectionTrait, id: Uuid) -> Result<bool> {
    let maintenant = Utc::now().fixed_offset();

    let touchees = one_time_token::Entity::update_many()
        .col_expr(one_time_token::Column::ConsumedAt, Expr::value(maintenant))
        .filter(one_time_token::Column::Id.eq(id))
        .filter(one_time_token::Column::ConsumedAt.is_null())
        .filter(one_time_token::Column::ExpiresAt.gt(maintenant))
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}

/// Ferme les jetons encore vivants du même compte et du même usage.
///
/// Appelée avant chaque émission. Sans elle, l'utilisateur qui redemande un lien parce que
/// le premier est parti dans une boîte qu'il ne contrôle plus laisse ce premier lien
/// valide jusqu'à son terme.
pub async fn invalidate_pending(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    purpose: TokenPurpose,
) -> Result<u64> {
    let touchees = one_time_token::Entity::update_many()
        .col_expr(
            one_time_token::Column::ConsumedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(one_time_token::Column::UserId.eq(user_id))
        .filter(one_time_token::Column::Purpose.eq(purpose))
        .filter(one_time_token::Column::ConsumedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}

/// Supprime les jetons périmés, de tous les comptes, et dit combien.
///
/// La table croît d'une ligne par demande, au rythme du trafic anonyme : chaque émission
/// appelle cette purge avant d'écrire, ce qui la borne aux jetons encore vivants sans
/// réclamer de tâche périodique. L'index sur `expires_at` la garde bon marché.
pub async fn purge_expired(db: &impl ConnectionTrait) -> Result<u64> {
    let supprimees = one_time_token::Entity::delete_many()
        .filter(one_time_token::Column::ExpiresAt.lt(Utc::now().fixed_offset()))
        .exec(db)
        .await?;

    Ok(supprimees.rows_affected)
}
