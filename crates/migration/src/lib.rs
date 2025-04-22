use sea_orm_migration::prelude::*;

// Declare the examples module (which contains its own mod.rs)
mod examples;

// Declare real migration modules here, directly under src/
// mod m2024XXXX_YYYYYY_create_my_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations: Vec<Box<dyn MigrationTrait>> = vec![];

        // Add migrations from the examples module
        migrations.push(Box::new(examples::m20240101_000000_initial_setup::Migration));
        migrations.push(Box::new(examples::m20240101_000001_create_test_rls_table::Migration));
        
        // Add real migrations here
        // migrations.push(Box::new(m2024XXXX_YYYYYY_create_my_table::Migration));

        migrations
    }
} 