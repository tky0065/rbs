pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260828_093230_create_uploads;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260828_093230_create_uploads::Migration),
            // </rbs:migrations>
        ]
    }
}
