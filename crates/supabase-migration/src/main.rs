use clap::{Parser, Subcommand};
use sea_orm_migration::prelude::*;
use std::{env, fs, path::PathBuf};
use chrono::Utc;
use anyhow::Context;

// Import the Migrator from the library part of the crate (updated name)
use supabase_rust_migration_lib::Migrator;

#[derive(Parser, Debug)]
#[clap(name = "supabase-migration", version)] // Changed name to match Supabase CLI
#[clap(about = "Supabase Type-Safe Migration Tool (using sea-orm-migration)", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    /// Optional database URL. If not provided, tries to read from DATABASE_URL environment variable.
    #[clap(long)]
    database_url: Option<String>,

    /// Output debug logs to stderr
    #[clap(long)]
    debug: bool, // Added global flag example
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Apply pending migrations
    Up {
        /// Apply specific number of migrations
        #[clap(long)]
        num: Option<u32>
    },
    /// Rollback applied migrations
    Down {
        /// Rollback specific number of migrations. Use "all" to roll back all.
        #[clap(long, default_value = "1")]
        num: String
    },
    /// List local and remote migrations (shows applied vs pending)
    List,
    /// Create a new migration script file
    New {
        /// Descriptive name for the migration (e.g., create_users_table)
        name: String,
    },
    /// Fetch migration files from history table (Not Implemented)
    Fetch,
    /// Repair the migration history table (Not Implemented)
    Repair,
    /// Squash migrations to a single file (Not Implemented)
    Squash,
    /// Apply all pending migrations
    Fresh,
    /// Rollback all applied migrations
    Refresh,
    /// Drop all tables, then reapply all migrations
    Reset,
}

// --- Main application logic ---

async fn run() -> anyhow::Result<()> {
    dotenv::dotenv().ok(); // Load .env file if present
    let cli = Cli::parse();

    // TODO: Implement handling for --debug flag (e.g., initialize a logger)
    if cli.debug {
        eprintln!("Debug mode enabled.");
        // Setup logging level based on debug flag here if using a logger like env_logger or tracing
    }

    // Handle commands that don't need a DB connection first
    if let Commands::New { name } = cli.command {
        return create_new_migration(&name);
    }

    // Get DB connection for other commands
    let db_url = cli.database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow::anyhow!("Database URL not provided via --database-url or DATABASE_URL env var for this command"))?;

    println!("Connecting to database...");
    let conn = sea_orm::Database::connect(&db_url).await
        .with_context(|| format!("Failed to connect to database at {}", db_url))?;
    println!("Connected.");

    match cli.command {
        Commands::Up { num } => {
            println!("Applying migrations...");
            Migrator::up(&conn, num).await?;
        }
        Commands::Down { num } => {
            println!("Rolling back migrations...");
            let steps = match num.as_str() {
                 "all" => None,
                 s => Some(s.parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid number for down command: {}", s))?),
            };
            Migrator::down(&conn, steps).await?;
        }
        Commands::List => { // Was Status
            println!("Checking migration status...");
            Migrator::status(&conn).await?;
        }
        Commands::Fresh => {
            println!("Applying fresh migrations (apply all pending)...");
            Migrator::fresh(&conn).await?;
        }
        Commands::Refresh => {
            println!("Refreshing migrations (rollback all, then apply all)...");
            Migrator::refresh(&conn).await?;
        }
        Commands::Reset => {
            println!("Resetting database (drop all, then apply all)...");
            Migrator::reset(&conn).await?;
        }
        Commands::Fetch | Commands::Repair | Commands::Squash => {
            println!("Command not implemented in this tool.");
            // Or return an error:
            // return Err(anyhow::anyhow!("Command not implemented"));
        }
        Commands::New { .. } => unreachable!(), // Handled above
    }

    println!("Operation completed successfully.");
    Ok(())
}

// --- Helper function for 'new' command ---

fn create_new_migration(name: &str) -> anyhow::Result<()> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("m{}_{}.rs", timestamp, name);
    // Assume migrations are created directly in the src directory for simplicity
    // A real implementation might place them in a dedicated `src/migrations/` folder
    let migration_dir = PathBuf::from("src");
    let file_path = migration_dir.join(&filename);

    if !migration_dir.exists() {
        fs::create_dir_all(&migration_dir)
            .with_context(|| format!("Failed to create directory: {:?}", migration_dir))?;
    }

    let module_name = format!("m{}_{}", timestamp, name);

    let boilerplate = format!(r#"
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {{
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        // Replace the following example implementation with your actual migration logic
        // manager
        //     .create_table(
        //         Table::create()
        //             .table(Alias::new("your_table_name"))
        //             .if_not_exists()
        //             .col(
        //                 ColumnDef::new(Alias::new("id"))
        //                     .integer()
        //                     .not_null()
        //                     .auto_increment()
        //                     .primary_key(),
        //             )
        //             .col(ColumnDef::new(Alias::new("created_at")).timestamp_with_time_zone().default(Expr::current_timestamp()))
        //             .to_owned(),
        //     )
        //     .await
        println!("Apply migration: {}");
        Ok(())
    }}

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        // Replace the following example implementation with your actual rollback logic
        // manager
        //     .drop_table(Table::drop().table(Alias::new("your_table_name")).to_owned())
        //     .await
        println!("Rollback migration: {}");
        Ok(())
    }}
}}
"#, module_name, module_name);

    println!("Creating migration file: {:?}", file_path);
    fs::write(&file_path, boilerplate)
        .with_context(|| format!("Failed to write migration file: {:?}", file_path))?;

    println!("\nMigration file created successfully!");
    println!("Next steps:");
    println!("1. Implement the `up` and `down` methods in {:?}.", file_path);
    println!("2. Add the following lines to `src/migration.rs` inside the `Migrator::migrations()` vec:");
    println!("   mod {};", module_name);
    println!("   Box::new({}::Migration),", module_name);

    Ok(())
}

// --- Entry point ---

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {:#?}", err);
        std::process::exit(1);
    }
} 