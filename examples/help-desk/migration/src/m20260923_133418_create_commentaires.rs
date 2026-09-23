use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Commentaires::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Commentaires::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Commentaires::Corps).text().not_null())
                    .col(ColumnDef::new(Commentaires::TicketId).uuid().not_null())
                    .col(ColumnDef::new(Commentaires::AuteurId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_commentaires_ticket_id")
                            .from(Commentaires::Table, Commentaires::TicketId)
                            .to(Tickets::Table, Tickets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_commentaires_auteur_id")
                            .from(Commentaires::Table, Commentaires::AuteurId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .col(
                        ColumnDef::new(Commentaires::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Commentaires::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_commentaires_ticket_id")
                    .table(Commentaires::Table)
                    .col(Commentaires::TicketId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_commentaires_auteur_id")
                    .table(Commentaires::Table)
                    .col(Commentaires::AuteurId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Commentaires::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Commentaires {
    Table,
    Id,
    Corps,
    TicketId,
    AuteurId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Tickets {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
