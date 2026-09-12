use sea_orm::entity::prelude::*;

/// Rôle applicatif, stocké en texte.
///
/// Un rôle de plus s'ajoute ici, sans migration : c'est le fichier que vous ouvrirez
/// pour le faire, et la garde `require_role` le suit.
///
/// **L'ordre de déclaration porte la hiérarchie** : `require_role` compare un seuil, si
/// bien qu'une variante insérée entre deux autres déplace le seuil de toutes les gardes
/// du projet. Un rôle plus étendu s'ajoute donc en fin d'énumération.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Role {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "admin")]
    Admin,
}

/// Ce à quoi un jeton à usage unique donne droit.
///
/// Stocké en texte comme [`Role`] : un troisième usage s'ajoute ici, sans migration.
/// Aucun ordre n'est porté par cette énumération — les usages ne se contiennent pas, et
/// l'ordre de déclaration n'y a donc pas la portée qu'il a sur les rôles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum TokenPurpose {
    #[sea_orm(string_value = "password_reset")]
    PasswordReset,
    #[sea_orm(string_value = "email_verification")]
    EmailVerification,
}

pub mod user {
    use sea_orm::ActiveValue::Set;
    use sea_orm::entity::prelude::*;

    use super::Role;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(unique)]
        pub email: String,
        pub password_hash: String,
        pub role: Role,
        pub email_verified_at: Option<DateTimeWithTimeZone>,
        pub sessions_revoked_at: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:users>
        // </rbs:relations:users>
    }

    // <rbs:related:users>
    // </rbs:related:users>

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
}

pub mod refresh_token {
    use sea_orm::ActiveValue::Set;
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "refresh_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        #[sea_orm(indexed)]
        pub token_hash: String,
        pub expires_at: DateTimeWithTimeZone,
        pub revoked_at: Option<DateTimeWithTimeZone>,
        pub replaced_at: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:refresh_tokens>
        // </rbs:relations:refresh_tokens>
    }

    // <rbs:related:refresh_tokens>
    // </rbs:related:refresh_tokens>

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
}

pub mod one_time_token {
    use sea_orm::ActiveValue::Set;
    use sea_orm::entity::prelude::*;

    use super::TokenPurpose;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "one_time_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        #[sea_orm(indexed)]
        pub token_hash: String,
        pub purpose: TokenPurpose,
        pub expires_at: DateTimeWithTimeZone,
        pub consumed_at: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:one_time_tokens>
        // </rbs:relations:one_time_tokens>
    }

    // <rbs:related:one_time_tokens>
    // </rbs:related:one_time_tokens>

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
}
