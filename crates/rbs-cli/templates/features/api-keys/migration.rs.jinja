use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKeys::UserId).uuid().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).string().not_null())
                    // Les douze premiers caractères de la clé : `rbs_` et huit du tirage.
                    // De quoi reconnaître une clé dans une liste sans jamais la redonner.
                    .col(ColumnDef::new(ApiKeys::Prefix).string().not_null())
                    // L'empreinte, et jamais la clé : une base lue par un tiers ne lui
                    // donne aucune clé présentable.
                    .col(ColumnDef::new(ApiKeys::TokenHash).string().not_null())
                    .col(ColumnDef::new(ApiKeys::Role).string().not_null())
                    .col(
                        ColumnDef::new(ApiKeys::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // Nulle : la clé ne périme pas. C'est le défaut, et la rotation est
                    // un choix que `expires_in_days` rend à la création.
                    .col(
                        ColumnDef::new(ApiKeys::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // Une date plutôt qu'un booléen : elle porte le booléen et le moment.
                    .col(
                        ColumnDef::new(ApiKeys::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_keys_user_id")
                            .from(ApiKeys::Table, ApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Chaque requête authentifiée par clé cherche une ligne par cette colonne : c'est
        // le chemin de lecture le plus chaud du projet. Unique, aussi : deux lignes de
        // même empreinte seraient la même clé, et rien ne dirait laquelle a servi.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_api_keys_token_hash")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::TokenHash)
                    .to_owned(),
            )
            .await?;

        // `GET /api-keys` et la révocation en masse lisent par le porteur.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_api_keys_user_id")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    UserId,
    Name,
    Prefix,
    TokenHash,
    Role,
    LastUsedAt,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
}

/// La table des comptes, que pose le fragment `auth` : la clé référence son porteur.
#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
