use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Valeurs acceptées par la colonne « gravite ».
///
/// Une valeur de plus s'ajoute ici et dans le `CHECK` que porte une migration nouvelle :
/// la base refuse d'elle-même celles qu'elle ne connaît pas. Plus longue que toutes les
/// actuelles, elle demande en troisième lieu d'élargir le `StringLen::N` ci-dessous, et
/// avec lui le `string_len` de cette migration.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(7))")]
pub enum IncidentGravite {
    #[sea_orm(string_value = "basse")]
    #[serde(rename = "basse")]
    Basse,
    #[sea_orm(string_value = "moyenne")]
    #[serde(rename = "moyenne")]
    Moyenne,
    #[sea_orm(string_value = "haute")]
    #[serde(rename = "haute")]
    Haute,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "incidents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub reference: String,
    pub sujet: String,
    #[sea_orm(column_type = "Text")]
    pub detail: String,
    pub gravite: IncidentGravite,
    pub ouvert: bool,
    pub duree_minutes: Option<i32>,
    pub echeance: Option<Date>,
    pub constate_le: DateTimeWithTimeZone,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:incidents>
    // </rbs:relations:incidents>
}

// <rbs:related:incidents>
// </rbs:related:incidents>

/// L'identifiant est posé ici, et non par un défaut de colonne : `uuidv7()` n'a
/// d'équivalent à écrire ni en MySQL ni en SQLite.
///
/// `new()` est le seul point à écrire — la macro fait déléguer `Default::default()` ici,
/// et tout ce que le projet insère passe par `..Default::default()`. La monotonie est
/// garantie par processus, là où celle de PostgreSQL l'était par serveur.
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            ..ActiveModelTrait::default()
        }
    }
}
