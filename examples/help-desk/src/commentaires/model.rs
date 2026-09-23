use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "commentaires")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text")]
    pub corps: String,
    #[sea_orm(indexed)]
    pub ticket_id: Uuid,
    #[sea_orm(indexed)]
    pub auteur_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::tickets::model::Entity",
        from = "Column::TicketId",
        to = "crate::tickets::model::Column::Id",
        on_delete = "Cascade"
    )]
    Ticket,
    #[sea_orm(
        belongs_to = "crate::auth::model::user::Entity",
        from = "Column::AuteurId",
        to = "crate::auth::model::user::Column::Id",
        on_delete = "Restrict"
    )]
    Auteur,
    // <rbs:relations:commentaires>
    // </rbs:relations:commentaires>
}

impl Related<crate::tickets::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ticket.def()
    }
}

impl Related<crate::auth::model::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Auteur.def()
    }
}

// <rbs:related:commentaires>
// </rbs:related:commentaires>

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
