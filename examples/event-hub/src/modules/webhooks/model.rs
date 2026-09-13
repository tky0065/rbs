use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "webhook_subscriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Où POSTer la livraison.
    pub url: String,
    /// Les motifs écoutés : `*`, `user.*` ou `user.created`.
    pub events: Json,
    /// Le secret de signature, propre à cet abonnement et rendu une seule fois.
    pub secret: String,
    /// La révocation, datée. Nulle tant que l'abonnement sert.
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:webhook_subscriptions>
    // </rbs:relations:webhook_subscriptions>
}

// <rbs:related:webhook_subscriptions>
// </rbs:related:webhook_subscriptions>

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
