use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Uploads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Uploads::Id)
                            .uuid()
                            .not_null()
                            .default(Expr::cust("uuidv7()"))
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Uploads::Title).string().not_null())
                    .col(ColumnDef::new(Uploads::OwnerEmail).string().not_null())
                    .col(ColumnDef::new(Uploads::ContentType).string().not_null())
                    .col(ColumnDef::new(Uploads::Size).integer().not_null())
                    .col(
                        ColumnDef::new(Uploads::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Uploads::UpdatedAt)
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
            .drop_table(Table::drop().table(Uploads::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Uploads {
    Table,
    Id,
    Title,
    OwnerEmail,
    ContentType,
    Size,
    CreatedAt,
    UpdatedAt,
}
