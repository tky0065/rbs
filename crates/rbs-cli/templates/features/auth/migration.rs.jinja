use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Users::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(
                        ColumnDef::new(Users::Role)
                            .string()
                            .not_null()
                            .default("user"),
                    )
                    // Nulle tant que l'adresse n'est pas prouvée. La date et non un
                    // booléen : savoir *quand* une adresse a été vérifiée est ce qui
                    // permet, un jour, de redemander une preuve aux plus anciennes.
                    .col(
                        ColumnDef::new(Users::EmailVerifiedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RefreshTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RefreshTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RefreshTokens::UserId).uuid().not_null())
                    // L'empreinte du jeton, jamais le jeton : une base lue par un tiers ne
                    // lui donne rien qu'il puisse présenter.
                    .col(ColumnDef::new(RefreshTokens::TokenHash).string().not_null())
                    .col(
                        ColumnDef::new(RefreshTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    // Nul tant que la session vit. La ligne survit à sa révocation : c'est
                    // elle qui distingue un jeton retiré d'un jeton jamais émis.
                    .col(
                        ColumnDef::new(RefreshTokens::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RefreshTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(RefreshTokens::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refresh_tokens_user_id")
                            .from(RefreshTokens::Table, RefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OneTimeTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OneTimeTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OneTimeTokens::UserId).uuid().not_null())
                    // L'empreinte, jamais le jeton : la même règle que pour les jetons de
                    // rafraîchissement, et pour la même raison.
                    .col(ColumnDef::new(OneTimeTokens::TokenHash).string().not_null())
                    // L'usage fait partie de la recherche : sans lui, un jeton de
                    // vérification vaudrait comme jeton de réinitialisation.
                    .col(ColumnDef::new(OneTimeTokens::Purpose).string().not_null())
                    .col(
                        ColumnDef::new(OneTimeTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    // Nul tant que le jeton vit. La ligne survit à son usage : c'est elle
                    // qui distingue un jeton déjà joué d'un jeton jamais émis.
                    .col(
                        ColumnDef::new(OneTimeTokens::ConsumedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OneTimeTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OneTimeTokens::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_time_tokens_user_id")
                            .from(OneTimeTokens::Table, OneTimeTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_one_time_tokens_token_hash")
                    .table(OneTimeTokens::Table)
                    .col(OneTimeTokens::TokenHash)
                    .to_owned(),
            )
            .await?;

        // Chaque rafraîchissement cherche une ligne par cette colonne : sans index, le
        // coût de l'opération croît avec le nombre de sessions ouvertes.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_refresh_tokens_token_hash")
                    .table(RefreshTokens::Table)
                    .col(RefreshTokens::TokenHash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // La table portant la clé étrangère part la première.
        manager
            .drop_table(Table::drop().table(OneTimeTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RefreshTokens::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
    PasswordHash,
    Role,
    EmailVerifiedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum OneTimeTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    Purpose,
    ExpiresAt,
    ConsumedAt,
    CreatedAt,
    UpdatedAt,
}
