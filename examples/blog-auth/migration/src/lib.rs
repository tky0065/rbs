pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260827_134719_create_auth_tables;
mod m20260827_134721_create_posts;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260827_134719_create_auth_tables::Migration),
            Box::new(m20260827_134721_create_posts::Migration),
            // </rbs:migrations>
        ]
    }
}
