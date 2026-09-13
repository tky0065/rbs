use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// L'auteur de l'écriture, ou rien : un job, un seed et une commande n'ont pas
    /// d'identité HTTP, et ce sont précisément les écritures qu'on cherche à expliquer.
    pub actor_id: Option<String>,
    pub action: String,
    /// La table visée.
    pub entity: String,
    /// La clé de la ligne visée, telle qu'elle s'écrit — une clé entière ou composite
    /// doit rester citable.
    pub entity_id: String,
    /// Ce qui a changé. Le fragment n'impose aucun schéma.
    pub changes: Json,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:audit_log>
    // </rbs:relations:audit_log>
}

// <rbs:related:audit_log>
// </rbs:related:audit_log>

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
