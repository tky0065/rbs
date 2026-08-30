use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

/// Où en est un job dans la file.
///
/// `Running` distingue la ligne qu'un worker a réservée de celle qui attend encore : sans
/// elle, deux workers dépileraient la même.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Status {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "done")]
    Done,
    #[sea_orm(string_value = "failed")]
    Failed,
}

impl Status {
    /// La valeur telle que la colonne la porte.
    ///
    /// Le dépilage passe une requête écrite à la main, où le statut est un paramètre et
    /// non une valeur de l'ORM.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pending => "pending",
            Status::Running => "running",
            Status::Done => "done",
            Status::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Le `KIND` du job, par lequel le registre retrouve quoi exécuter.
    pub kind: String,
    /// Le job sérialisé, tel que `enqueue` l'a écrit.
    pub payload: Json,
    pub status: Status,
    /// Tentatives déjà consommées, réservation comprise.
    pub attempts: i32,
    /// Instant à partir duquel la ligne est dépilable. Un réessai le repousse.
    pub available_at: DateTimeWithTimeZone,
    /// Le message de la dernière tentative ratée. Nul tant qu'aucune n'a échoué.
    pub last_error: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:jobs>
    // </rbs:relations:jobs>
}

// <rbs:related:jobs>
// </rbs:related:jobs>

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
