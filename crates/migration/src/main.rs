use sea_orm_migration::prelude::*;
use std::env;

// Make sure the lib module is accessible
// The crate name is supabase-rust-migration, so we use that.
use supabase_rust_migration::Migrator;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok(); // Use .ok() to ignore errors if .env is not present
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("cli");
    
    match command {
        "up" => {
            println!("Running migrations up...");
            let schema_manager = SchemaManager::new(get_database_connection().await);
            Migrator::up(&schema_manager, None).await.unwrap();
            println!("Completed migrations up.");
        }
        "down" => {
            println!("Running migrations down...");
            let schema_manager = SchemaManager::new(get_database_connection().await);
            Migrator::down(&schema_manager, None).await.unwrap();
            println!("Completed migrations down.");
        }
        "fresh" => {
            println!("Refreshing database (down then up)...");
            let schema_manager = SchemaManager::new(get_database_connection().await);
            Migrator::down(&schema_manager, None).await.unwrap();
            Migrator::up(&schema_manager, None).await.unwrap();
            println!("Database refresh completed.");
        }
        "status" => {
            println!("Checking migration status...");
            let schema_manager = SchemaManager::new(get_database_connection().await);
            let status = Migrator::status(&schema_manager).await.unwrap();
            
            println!("Migration Status:");
            for migration in status {
                println!(
                    "{}: {}",
                    migration.name,
                    if migration.applied { "Applied" } else { "Pending" }
                );
            }
        }
        _ => {
            // Default to CLI mode
            println!("Running SeaORM migration CLI...");
            cli::run_cli(Migrator).await;
            println!("SeaORM migration CLI finished.");
        }
    }
}

async fn get_database_connection() -> sea_orm::DatabaseConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
    
    sea_orm::Database::connect(&database_url)
        .await
        .expect("Failed to connect to the database")
} 