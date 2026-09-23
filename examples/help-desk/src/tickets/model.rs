use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Valeurs acceptées par la colonne « statut ».
///
/// Une valeur de plus s'ajoute ici et dans le `CHECK` que porte une migration nouvelle :
/// la base refuse d'elle-même celles qu'elle ne connaît pas. Plus longue que toutes les
/// actuelles, elle demande en troisième lieu d'élargir le `StringLen::N` ci-dessous, et
/// avec lui le `string_len` de cette migration.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(8))")]
pub enum TicketStatut {
    #[sea_orm(string_value = "ouvert")]
    #[serde(rename = "ouvert")]
    Ouvert,
    #[sea_orm(string_value = "en_cours")]
    #[serde(rename = "en_cours")]
    EnCours,
    #[sea_orm(string_value = "resolu")]
    #[serde(rename = "resolu")]
    Resolu,
    #[sea_orm(string_value = "ferme")]
    #[serde(rename = "ferme")]
    Ferme,
}

/// Valeurs acceptées par la colonne « priorite ».
///
/// Une valeur de plus s'ajoute ici et dans le `CHECK` que porte une migration nouvelle :
/// la base refuse d'elle-même celles qu'elle ne connaît pas. Plus longue que toutes les
/// actuelles, elle demande en troisième lieu d'élargir le `StringLen::N` ci-dessous, et
/// avec lui le `string_len` de cette migration.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(7))")]
pub enum TicketPriorite {
    #[sea_orm(string_value = "basse")]
    #[serde(rename = "basse")]
    Basse,
    #[sea_orm(string_value = "normale")]
    #[serde(rename = "normale")]
    Normale,
    #[sea_orm(string_value = "haute")]
    #[serde(rename = "haute")]
    Haute,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tickets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub sujet: String,
    #[sea_orm(column_type = "Text")]
    pub detail: String,
    pub statut: TicketStatut,
    pub priorite: TicketPriorite,
    #[sea_orm(indexed)]
    pub auteur_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::auth::model::user::Entity",
        from = "Column::AuteurId",
        to = "crate::auth::model::user::Column::Id",
        on_delete = "Restrict"
    )]
    Auteur,
    // <rbs:relations:tickets>
    #[sea_orm(has_many = "crate::commentaires::model::Entity")]
    Commentaires,
    // </rbs:relations:tickets>
}

impl Related<crate::auth::model::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Auteur.def()
    }
}

// <rbs:related:tickets>
impl Related<crate::commentaires::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Commentaires.def()
    }
}
// </rbs:related:tickets>

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
