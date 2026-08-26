use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // region: up
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Articles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Articles::Id)
                            .uuid()
                            .not_null()
                            .default(Expr::cust("uuidv7()"))
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Articles::Title).string().not_null())
                    .col(ColumnDef::new(Articles::Body).text().not_null())
                    .col(ColumnDef::new(Articles::Published).boolean().not_null())
                    .col(
                        ColumnDef::new(Articles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Articles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
    // endregion: up

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Articles::Table).to_owned())
            .await
    }
}

// region: colonnes
#[derive(DeriveIden)]
enum Articles {
    Table,
    Id,
    Title,
    Body,
    Published,
    CreatedAt,
    UpdatedAt,
}
// endregion: colonnes
