use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Jobs::Id).uuid().not_null().primary_key())
                    // Le nom du job, non son code : c'est par lui que le registre du
                    // projet retrouve quoi exécuter.
                    .col(ColumnDef::new(Jobs::Kind).string().not_null())
                    .col(ColumnDef::new(Jobs::Payload).json().not_null())
                    .col(
                        ColumnDef::new(Jobs::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(Jobs::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    // Ce qui rend un réessai différé possible : un job raté revient en
                    // file avec cette date repoussée, plutôt que d'être redépilé aussitôt.
                    .col(
                        ColumnDef::new(Jobs::AvailableAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Jobs::LastError).text().null())
                    .col(
                        ColumnDef::new(Jobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Jobs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Le worker interroge la table à chaque tour de boucle sur ces deux colonnes :
        // sans index, le coût du dépilage croît avec l'historique des jobs exécutés.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_jobs_status_available_at")
                    .table(Jobs::Table)
                    .col(Jobs::Status)
                    .col(Jobs::AvailableAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Jobs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Id,
    Kind,
    Payload,
    Status,
    Attempts,
    AvailableAt,
    LastError,
    CreatedAt,
    UpdatedAt,
}
