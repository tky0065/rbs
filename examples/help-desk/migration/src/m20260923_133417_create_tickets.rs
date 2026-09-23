use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tickets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tickets::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Tickets::Sujet).string().not_null())
                    .col(ColumnDef::new(Tickets::Detail).text().not_null())
                    .col(
                        ColumnDef::new(Tickets::Statut)
                            .string_len(8)
                            .not_null()
                            .check(
                                Expr::col(Tickets::Statut)
                                    .is_in(["ouvert", "en_cours", "resolu", "ferme"]),
                            ),
                    )
                    .col(
                        ColumnDef::new(Tickets::Priorite)
                            .string_len(7)
                            .not_null()
                            .check(
                                Expr::col(Tickets::Priorite).is_in(["basse", "normale", "haute"]),
                            ),
                    )
                    .col(ColumnDef::new(Tickets::AuteurId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tickets_auteur_id")
                            .from(Tickets::Table, Tickets::AuteurId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .col(
                        ColumnDef::new(Tickets::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tickets::UpdatedAt)
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
                    .name("idx_tickets_auteur_id")
                    .table(Tickets::Table)
                    .col(Tickets::AuteurId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tickets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tickets {
    Table,
    Id,
    Sujet,
    Detail,
    Statut,
    Priorite,
    AuteurId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
