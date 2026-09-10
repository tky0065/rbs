use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::super::model::refresh_token;

/// Ouvre une session de rafraîchissement.
///
/// `fingerprint` et non le jeton : c'est ce que la table doit porter pour qu'une base lue
/// par un tiers ne lui donne aucune session utilisable.
pub async fn create_refresh_token(
    db: &DatabaseConnection,
    user_id: Uuid,
    fingerprint: String,
    expire_a: DateTimeWithTimeZone,
) -> Result<()> {
    refresh_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(fingerprint),
        expires_at: Set(expire_a),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn find_refresh_token(
    db: &DatabaseConnection,
    fingerprint: &str,
) -> Result<Option<refresh_token::Model>> {
    Ok(refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(fingerprint))
        .one(db)
        .await?)
}

/// Consomme une session, et dit si c'est bien cet appel qui l'a fait.
///
/// L'`UPDATE` porte sa propre condition plutôt que de suivre une lecture : deux
/// rafraîchissements simultanés du même jeton franchiraient tous deux la lecture avant
/// que l'un ait écrit, et repartiraient chacun avec une paire valide.
pub async fn consume(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(refresh_token::Column::RevokedAt, Expr::current_timestamp())
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}

/// Ferme toutes les sessions encore ouvertes d'un compte, et dit combien l'étaient.
///
/// La granularité est le compte et non la chaîne de rotation : distinguer les chaînes
/// demanderait à la table une colonne de famille, qu'un projet déjà migré ne recevrait
/// jamais. Fermer trop large coûte une reconnexion ; fermer trop étroit laisse une paire
/// volée en circulation.
pub async fn revoke_sessions_of(db: &DatabaseConnection, user_id: Uuid) -> Result<u64> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(refresh_token::Column::RevokedAt, Expr::current_timestamp())
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}
