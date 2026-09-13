use chrono::Utc;
use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use super::super::model::refresh_token;

// La couche service a besoin du type pour rendre `SessionResponse` sans construire de
// requête : c'est elle qui appelle `open_sessions_of`, jamais le modèle directement.
pub use super::super::model::refresh_token::Model;

// L'instant vient de Rust et se lie en paramètre, jamais de `CURRENT_TIMESTAMP` : sqlx
// écrit la colonne en RFC 3339 (`2026-01-01T…+00:00`) là où l'horloge de SQLite rend
// `2026-01-01 23:59:59`, et SQLite compare le texte — `'T' > ' '`, toute échéance du jour
// passait pour future. L'écriture de `revoked_at` et de `replaced_at` suit la même règle
// pour qu'aucune des deux colonnes ne mélange deux formats.

/// Ouvre une session de rafraîchissement.
///
/// `fingerprint` et non le jeton : c'est ce que la table doit porter pour qu'une base lue
/// par un tiers ne lui donne aucune session utilisable.
pub async fn create_refresh_token(
    db: &impl ConnectionTrait,
    user_id: Uuid,
    fingerprint: String,
    expire_a: DateTimeWithTimeZone,
) -> Result<()> {
    refresh_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(fingerprint),
        expires_at: Set(expire_a),
        // Posé ici et non par le défaut de colonne : `CURRENT_TIMESTAMP` est à la seconde
        // sur SQLite et MySQL, et deux sessions ouvertes dans la même seconde n'auraient
        // plus d'ordre — `open_sessions_of` promet la plus récente en tête.
        created_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn find_refresh_token(
    db: &impl ConnectionTrait,
    fingerprint: &str,
) -> Result<Option<refresh_token::Model>> {
    Ok(refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(fingerprint))
        .one(db)
        .await?)
}

/// Ce qu'une rotation a trouvé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// La ligne était ouverte : c'est cet appel qui l'a tournée.
    Done,
    /// La ligne avait déjà tourné et n'avait pas encore été fermée : c'est cet appel qui
    /// l'a fait, sur ce rejeu.
    Replayed,
    /// La ligne avait été fermée — un rejeu déjà instruit, une déconnexion, une
    /// révocation, un mot de passe changé.
    Closed,
}

/// Tourne une session, et dit si c'est bien cet appel qui l'a fait.
///
/// L'`UPDATE` porte sa propre condition plutôt que de suivre une lecture : deux
/// rafraîchissements simultanés du même jeton franchiraient tous deux la lecture avant
/// que l'un ait écrit, et repartiraient chacun avec une paire valide. La lecture qui suit
/// un `UPDATE` sans effet ne décide de rien : elle nomme l'état déjà écrit.
pub async fn rotate(db: &impl ConnectionTrait, id: Uuid) -> Result<Rotation> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::ReplacedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    if touchees.rows_affected == 1 {
        return Ok(Rotation::Done);
    }

    let ligne = refresh_token::Entity::find_by_id(id).one(db).await?;

    // Le rejeu d'une ligne tournée est instruit une fois — la famille tombe — et la ligne
    // est fermée à son tour ; un rejeu répété de la même ligne n'apprend rien de plus et
    // ne ferait qu'offrir à qui tient un jeton mort un bouton pour déconnecter le
    // titulaire à chaque reconnexion. L'ordre des gardes met donc `revoked_at` avant
    // `replaced_at` : les deux colonnes posées signifient « rejeu déjà instruit ».
    match ligne {
        Some(ligne) if ligne.revoked_at.is_some() => Ok(Rotation::Closed),
        Some(ligne) if ligne.replaced_at.is_some() => {
            refresh_token::Entity::update_many()
                .col_expr(
                    refresh_token::Column::RevokedAt,
                    Expr::value(Utc::now().fixed_offset()),
                )
                .filter(refresh_token::Column::Id.eq(id))
                .filter(refresh_token::Column::RevokedAt.is_null())
                .exec(db)
                .await?;

            Ok(Rotation::Replayed)
        }
        _ => Ok(Rotation::Closed),
    }
}

/// Ferme une session présentée, et dit si c'est bien cet appel qui l'a fait.
pub async fn close(db: &impl ConnectionTrait, id: Uuid) -> Result<bool> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::RevokedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
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
pub async fn revoke_sessions_of(db: &impl ConnectionTrait, user_id: Uuid) -> Result<u64> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::RevokedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}

/// Les sessions encore ouvertes d'un compte, la plus récente d'abord.
///
/// Ni tournées, ni révoquées, ni périmées : ce que la liste montre est ce qu'une
/// révocation fermerait.
pub async fn open_sessions_of(
    db: &impl ConnectionTrait,
    user_id: Uuid,
) -> Result<Vec<refresh_token::Model>> {
    Ok(refresh_token::Entity::find()
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .filter(refresh_token::Column::ExpiresAt.gt(Utc::now().fixed_offset()))
        .order_by_desc(refresh_token::Column::CreatedAt)
        .all(db)
        .await?)
}

/// Ferme une session nommée, et dit si elle appartenait bien au compte.
///
/// Le propriétaire est dans la condition de l'`UPDATE` et non dans une lecture qui le
/// précède : comparer après avoir lu laisserait la révocation de la session d'autrui à
/// portée d'une course.
pub async fn revoke_session(db: &impl ConnectionTrait, id: Uuid, user_id: Uuid) -> Result<bool> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::RevokedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
