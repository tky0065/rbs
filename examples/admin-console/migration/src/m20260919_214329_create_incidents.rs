use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Incidents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Incidents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Incidents::Reference)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Incidents::Sujet).string().not_null())
                    .col(ColumnDef::new(Incidents::Detail).text().not_null())
                    .col(
                        ColumnDef::new(Incidents::Gravite)
                            .string_len(7)
                            .not_null()
                            .check(
                                Expr::col(Incidents::Gravite).is_in(["basse", "moyenne", "haute"]),
                            ),
                    )
                    .col(ColumnDef::new(Incidents::Ouvert).boolean().not_null())
                    .col(ColumnDef::new(Incidents::DureeMinutes).integer().null())
                    .col(ColumnDef::new(Incidents::Echeance).date().null())
                    .col(
                        ColumnDef::new(Incidents::ConstateLe)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Incidents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Incidents::UpdatedAt)
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
            .drop_table(Table::drop().table(Incidents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Incidents {
    Table,
    Id,
    Reference,
    Sujet,
    Detail,
    Gravite,
    Ouvert,
    DureeMinutes,
    Echeance,
    ConstateLe,
    CreatedAt,
    UpdatedAt,
}
