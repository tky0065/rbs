pub use sea_orm_migration::prelude::*;

// <rbs:migration_modules>
mod m20260923_133416_create_auth_tables;
mod m20260923_133417_create_tickets;
mod m20260923_133418_create_commentaires;
// </rbs:migration_modules>

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // <rbs:migrations>
            Box::new(m20260923_133416_create_auth_tables::Migration),
            Box::new(m20260923_133417_create_tickets::Migration),
            Box::new(m20260923_133418_create_commentaires::Migration),
            // </rbs:migrations>
        ]
    }
}
