use sea_orm_migration::prelude::*;
use std::env;

// Make sure the lib module is accessible
use supabase_rust_migration::{Migrator, SchemaMigrator, DirectoryMigrator};

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok(); // Use .ok() to ignore errors if .env is not present

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("cli");
    
    // Check for schema-based migration flag
    let use_schema_migration = args.iter().any(|arg| arg == "--schema");
    let schema_dir = args.iter()
        .position(|arg| arg == "--schema-dir")
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str());

    match command {
        "up" => {
            println!("Running migrations up...");
            
            if use_schema_migration {
                // Use schema-based migrations
                let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
                
                if let Some(dir_path) = schema_dir {
                    // Run migrations from specified directory
                    println!("Running schema-based migrations from directory: {}", dir_path);
                    let migrator = DirectoryMigrator;
                    migrator.run_from_directory(&db_url, dir_path).await
                        .expect("Failed to run schema-based migrations from directory");
                } else {
                    // Run embedded schema-based migrations
                    println!("Running embedded schema-based migrations");
                    let migrator = SchemaMigrator;
                    migrator.run_migrations(&db_url).await
                        .expect("Failed to run schema-based migrations");
                }
            } else {
                // Use traditional sea-orm migrations
                let db = get_database_connection().await;
                Migrator::up(&db, None).await.unwrap();
            }
            
            println!("Completed migrations up.");
        }
        "down" => {
            println!("Running migrations down...");
            
            if use_schema_migration {
                println!("Schema-based migrations don't support 'down' directly.");
                println!("To revert, create a new migration that undoes the changes.");
            } else {
                let db = get_database_connection().await;
                Migrator::down(&db, None).await.unwrap();
            }
            
            println!("Completed migrations down.");
        }
        "fresh" => {
            println!("Refreshing database (down then up)...");
            
            if use_schema_migration {
                println!("Running schema refresh...");
                let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
                
                // For schema migrations, we'll recreate all tables from scratch
                // First run down migrations with traditional migrations to clean up
                let db = get_database_connection().await;
                Migrator::down(&db, None).await.unwrap();
                
                // Then apply schema migrations
                if let Some(dir_path) = schema_dir {
                    let migrator = DirectoryMigrator;
                    migrator.run_from_directory(&db_url, dir_path).await
                        .expect("Failed to run schema-based migrations from directory");
                } else {
                    let migrator = SchemaMigrator;
                    migrator.run_migrations(&db_url).await
                        .expect("Failed to run schema-based migrations");
                }
            } else {
                let db = get_database_connection().await;
                Migrator::down(&db, None).await.unwrap();
                Migrator::up(&db, None).await.unwrap();
            }
            
            println!("Database refresh completed.");
        }
        "status" => {
            println!("Checking migration status...");
            let db = get_database_connection().await;
            let migration_infos = Migrator::get_migration_infos(&db).await.unwrap();
            
            for info in migration_infos.iter() {
                match info.status {
                    MigrationStatus::Applied => {
                        println!("[✓] {} ({})", info.name, info.version);
                    }
                    MigrationStatus::Pending => {
                        println!("[ ] {} ({})", info.name, info.version);
                    }
                    MigrationStatus::Error => {
                        println!("[!] {} ({})", info.name, info.version);
                    }
                    _ => {
                        println!("[?] {} ({})", info.name, info.version);
                    }
                }
            }
            
            if use_schema_migration {
                println!("\nSchema-based migration status:");
                // This would require a custom implementation to check schema version
                println!("See refinery_schema_history table for details.");
            }
        }
        "cli" => {
            println!("Running migration CLI...");
            if use_schema_migration {
                println!("Schema-based migrations selected.");
            }
            
            let db = get_database_connection().await;
            sea_orm_migration::cli::run_cli(db).await;
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Available commands: up, down, fresh, status, cli");
            println!("Options:");
            println!("  --schema        Use schema-based migrations");
            println!("  --schema-dir    Specify directory for schema migrations");
        }
    }
}

/// Get a database connection pool
async fn get_database_connection() -> sea_orm::DatabaseConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    sea_orm::Database::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}
