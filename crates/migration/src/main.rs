use sea_orm_migration::prelude::*;
use std::env;

// Make sure the lib module is accessible
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
            let db = get_database_connection().await;
            Migrator::up(&db, None).await.unwrap();
            println!("Completed migrations up.");
        }
        "down" => {
            println!("Running migrations down...");
            let db = get_database_connection().await;
            Migrator::down(&db, None).await.unwrap();
            println!("Completed migrations down.");
        }
        "fresh" => {
            println!("Refreshing database (down then up)...");
            let db = get_database_connection().await;
            Migrator::down(&db, None).await.unwrap();
            Migrator::up(&db, None).await.unwrap();
            println!("Database refresh completed.");
        }
        "status" => {
            println!("Checking migration status...");
            let db = get_database_connection().await;
            
            // 実際にどんな値を返すか確かめるためにデバッグ出力
            println!("Migration status_list debugging:");
            let result = Migrator::status(&db).await;
            match result {
                Ok(()) => {
                    println!("Status command executed successfully (returned unit type)");
                },
                Err(e) => {
                    println!("Error checking migration status: {}", e);
                }
            }
            
            // 代わりにMigratorのmigrationsリストを取得して表示
            println!("\nConfigured migrations:");
            for migration in Migrator::migrations() {
                println!("- {}", migration.name());
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
