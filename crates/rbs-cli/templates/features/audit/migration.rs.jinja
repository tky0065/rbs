use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AuditLog::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(AuditLog::ActorId).text().null())
                    .col(ColumnDef::new(AuditLog::Action).text().not_null())
                    .col(ColumnDef::new(AuditLog::Entity).text().not_null())
                    .col(ColumnDef::new(AuditLog::EntityId).text().not_null())
                    .col(ColumnDef::new(AuditLog::Changes).json().not_null())
                    .col(
                        ColumnDef::new(AuditLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // L'histoire d'une ligne se lit par le premier, celle d'une journée par le
        // second : sans eux, le coût d'une lecture croît avec le journal entier, et un
        // journal est fait pour grossir.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audit_log_entity")
                    .table(AuditLog::Table)
                    .col(AuditLog::Entity)
                    .col(AuditLog::EntityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audit_log_created_at")
                    .table(AuditLog::Table)
                    .col(AuditLog::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    ActorId,
    Action,
    Entity,
    EntityId,
    Changes,
    CreatedAt,
}
