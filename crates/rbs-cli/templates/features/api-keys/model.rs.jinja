use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

use crate::auth::model::Role;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Le compte auquel la clé appartient : elle ne vaut jamais plus que lui.
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    /// Ce que la clé sert — « ci », « script de facturation ». Pour la reconnaître.
    pub name: String,
    /// Les douze premiers caractères de la clé, seuls à reparaître après la création.
    pub prefix: String,
    /// Empreinte SHA-256 de la clé entière. La clé elle-même n'est jamais stockée.
    #[sea_orm(unique, indexed)]
    pub token_hash: String,
    /// Le rôle propre à la clé, plafonné à la lecture par celui de son porteur.
    pub role: Role,
    /// Dernier usage constaté, à la minute près. Nul tant que la clé n'a jamais servi.
    pub last_used_at: Option<DateTimeWithTimeZone>,
    /// L'échéance, si on lui en a donné une. Nulle : la clé ne périme pas.
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// La révocation, datée. Nulle tant que la clé sert.
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:api_keys>
    // </rbs:relations:api_keys>
}

// <rbs:related:api_keys>
// </rbs:related:api_keys>

/// L'identifiant est posé ici, et non par un défaut de colonne : `uuidv7()` n'a
/// d'équivalent à écrire ni en MySQL ni en SQLite.
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            ..ActiveModelTrait::default()
        }
    }
}
