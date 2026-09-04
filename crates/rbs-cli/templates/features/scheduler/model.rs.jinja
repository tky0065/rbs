use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "schedules")]
pub struct Model {
    /// Le `KIND` du job déclenché. La clé primaire *est* l'unicité de l'échéance.
    #[sea_orm(primary_key, auto_increment = false)]
    pub kind: String,
    /// L'échéance. La réservation la compare, puis l'avance à l'occurrence suivante.
    pub next_run_at: DateTimeWithTimeZone,
    /// Le dernier déclenchement, ou rien tant qu'il n'y en a pas eu.
    pub last_run_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:schedules>
    // </rbs:relations:schedules>
}

// <rbs:related:schedules>
// </rbs:related:schedules>

impl ActiveModelBehavior for ActiveModel {}
