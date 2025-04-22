//! Schema-based migrations using refinery
//! 
//! This module provides the ability to define and run migrations based on schemas
//! instead of the traditional step-by-step approach.

mod directory;
mod postgrest_schema;
#[cfg(test)]
mod tests;

pub use directory::*;
pub use postgrest_schema::*;

use refinery::config::{Config, ConfigDbType};
use refinery::error::Error as RefineryError;
use refinery::{embed_migrations, Runner};
use std::path::Path;
use std::str::FromStr;
use tokio_postgres::{Client, Error as PgError, NoTls};

// Embed migrations from the migrations directory
// This will automatically include all SQL files in the directory
// in the format V{VERSION}__{NAME}.sql
embed_migrations!("src/schema_migrations/sql");

/// Container for schema-based migrations
pub struct SchemaMigrator;

impl SchemaMigrator {
    /// Run all pending migrations to the latest version
    pub async fn run_migrations(&self, conn_str: &str) -> Result<(), RefineryError> {
        let mut config = Config::new(ConfigDbType::Postgres);
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await
            .map_err(|e| RefineryError::ConnectError(Box::new(e)))?;
        
        // The connection needs to be running for the client to work
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });
        
        // Run migrations
        migrations::runner()
            .run_async(&mut ClientWrapper(client))
            .await
            .map_err(|e| RefineryError::ExecutionError(format!("Migration failed: {}", e)))
    }
    
    /// Load migrations from a directory instead of embedded ones
    pub async fn run_from_directory(
        &self, 
        conn_str: &str, 
        dir_path: &str
    ) -> Result<(), RefineryError> {
        let mut config = Config::new(ConfigDbType::Postgres);
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await
            .map_err(|e| RefineryError::ConnectError(Box::new(e)))?;
        
        // The connection needs to be running for the client to work
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });
        
        // Set migrations path
        config.set_migrations_path(Path::new(dir_path));
        
        // Build and run migrations from directory
        let runner = Runner::new(&config).build()
            .map_err(|e| RefineryError::ConfigError(e.to_string()))?;
            
        runner.run_async(&mut ClientWrapper(client))
            .await
            .map_err(|e| RefineryError::ExecutionError(format!("Migration failed: {}", e)))
    }
    
    /// Run migrations generated from table definitions
    pub async fn run_from_table_definitions(
        &self,
        conn_str: &str,
        tables: &[TableDefinition],
        version: i32,
        name: &str,
    ) -> Result<(), RefineryError> {
        // Generate migration from table definitions
        let migration = SchemaGenerator::create_migration(version, name, tables)?;
        
        // Connect to database
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await
            .map_err(|e| RefineryError::ConnectError(Box::new(e)))?;
        
        // The connection needs to be running for the client to work
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });
        
        // Create a runner with just this migration
        let mut migrations = vec![migration];
        let runner = Runner::new_migrations(migrations);
        
        // Run the migration
        runner.run_async(&mut ClientWrapper(client))
            .await
            .map_err(|e| RefineryError::ExecutionError(format!("Migration failed: {}", e)))
    }
    
    /// Run migrations from a JSON schema file
    pub async fn run_from_json_schema(
        &self,
        conn_str: &str,
        json_str: &str,
        version: i32,
        name: &str,
    ) -> Result<(), RefineryError> {
        // Import table definitions from JSON
        let tables = SchemaGenerator::import_from_json(json_str)?;
        
        // Run migrations from table definitions
        self.run_from_table_definitions(conn_str, &tables, version, name).await
    }
}

// Wrapper struct to implement AsyncMigrate trait on tokio_postgres::Client
struct ClientWrapper(Client);

#[refinery::async_trait]
impl refinery::AsyncMigrate for ClientWrapper {
    async fn as_ref_migrate<'a, 'b>(&'a mut self) -> Result<&mut dyn refinery::AsyncMigrate, RefineryError> 
    where 'a: 'b {
        Ok(self)
    }

    async fn get_schema_version(&mut self) -> Result<Option<i32>, RefineryError> {
        match self.0.query_one("SELECT current_setting('refinery.schema_version', true)::integer", &[]).await {
            Ok(row) => {
                let version: Option<i32> = row.get(0);
                Ok(version)
            },
            Err(_) => Ok(None),
        }
    }

    async fn set_schema_version(&mut self, version: i32) -> Result<(), RefineryError> {
        let stmt = format!("SELECT set_config('refinery.schema_version', '{}', false)", version);
        self.0.execute(&stmt, &[]).await.map_err(|e| RefineryError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn verify_migration_table(&mut self) -> Result<(), RefineryError> {
        self.0
            .execute(
                "CREATE TABLE IF NOT EXISTS refinery_schema_history(
                    version INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    applied_on TIMESTAMP NOT NULL,
                    checksum TEXT NOT NULL
                )",
                &[],
            )
            .await
            .map_err(|e| RefineryError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn get_migrations_table(&mut self) -> Result<Vec<refinery::Migration>, RefineryError> {
        let rows = self.0
            .query(
                "SELECT version, name, applied_on, checksum FROM refinery_schema_history ORDER BY version ASC",
                &[],
            )
            .await
            .map_err(|e| RefineryError::ExecutionError(e.to_string()))?;

        let mut migrations = Vec::new();
        for row in rows {
            let version: i32 = row.get(0);
            let name: String = row.get(1);
            let applied_on: chrono::DateTime<chrono::Utc> = row.get(2);
            let checksum: String = row.get(3);

            migrations.push(refinery::Migration {
                version,
                name,
                checksum,
                sql: String::new(),
                applied_on: Some(applied_on.to_rfc3339()),
            });
        }

        Ok(migrations)
    }

    async fn add_migration_to_table(
        &mut self,
        migration: &refinery::Migration,
    ) -> Result<(), RefineryError> {
        self.0
            .execute(
                "INSERT INTO refinery_schema_history(version, name, applied_on, checksum) VALUES ($1, $2, now(), $3)",
                &[&migration.version, &migration.name, &migration.checksum],
            )
            .await
            .map_err(|e| RefineryError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn remove_migration_from_table(
        &mut self,
        migration: &refinery::Migration,
    ) -> Result<(), RefineryError> {
        self.0
            .execute(
                "DELETE FROM refinery_schema_history WHERE version = $1",
                &[&migration.version],
            )
            .await
            .map_err(|e| RefineryError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    async fn execute(&mut self, migration: &refinery::Migration) -> Result<(), RefineryError> {
        self.0
            .execute(&migration.sql, &[])
            .await
            .map_err(|e| RefineryError::ExecutionError(e.to_string()))?;
        Ok(())
    }
} 