use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Subscribers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Subscribers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Subscribers::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Subscribers::Name).string().not_null())
                    .col(ColumnDef::new(Subscribers::Confirmed).boolean().not_null())
                    .col(
                        ColumnDef::new(Subscribers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Subscribers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Subscribers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Subscribers {
    Table,
    Id,
    Email,
    Name,
    Confirmed,
    CreatedAt,
    UpdatedAt,
}
