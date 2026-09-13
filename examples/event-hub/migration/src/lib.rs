pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260913_131801_create_auth_tables;
mod m20260913_131801_create_jobs;
mod m20260913_131801_create_webhook_subscriptions;
mod m20260913_131808_create_schedules;
mod m20260913_131820_create_audit_log;
mod m20260913_131916_create_orders;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260913_131801_create_jobs::Migration),
            Box::new(m20260913_131801_create_auth_tables::Migration),
            Box::new(m20260913_131801_create_webhook_subscriptions::Migration),
            Box::new(m20260913_131808_create_schedules::Migration),
            Box::new(m20260913_131820_create_audit_log::Migration),
            Box::new(m20260913_131916_create_orders::Migration),
            // </rbs:migrations>
        ]
    }
}
