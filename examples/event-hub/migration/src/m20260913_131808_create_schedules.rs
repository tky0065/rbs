use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Schedules::Table)
                    .if_not_exists()
                    // Le `KIND` du job déclenché, et non un identifiant de plus : c'est
                    // la clé primaire qui porte l'unicité d'une échéance. Une longueur
                    // bornée et non `text()` — MySQL refuse un `TEXT` en clé primaire
                    // sans elle, et 191 est ce qu'un index `utf8mb4` y admet.
                    .col(
                        ColumnDef::new(Schedules::Kind)
                            .string_len(191)
                            .not_null()
                            .primary_key(),
                    )
                    // L'échéance. La réservation la compare et l'avance d'un seul
                    // `UPDATE` : c'est cette colonne qui départage les réplicas.
                    .col(
                        ColumnDef::new(Schedules::NextRunAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Schedules::LastRunAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Schedules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Schedules::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await

        // Aucun index de plus, et ce n'est pas un oubli : toute lecture passe par la clé
        // primaire ou balaie une table qui compte autant de lignes que le projet a
        // d'échéances déclarées.
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Schedules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Schedules {
    Table,
    Kind,
    NextRunAt,
    LastRunAt,
    CreatedAt,
    UpdatedAt,
}
