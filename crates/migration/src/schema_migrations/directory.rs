//! Directory-based migration loading

use refinery::{Migration, Runner};
use refinery::config::{Config, ConfigDbType};
use refinery::error::Error as RefineryError;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio_postgres::{Client, NoTls};

use super::ClientWrapper;

/// Load migrations from a directory
pub struct DirectoryMigrator;

impl DirectoryMigrator {
    /// Run migrations from a specific directory
    pub async fn run_from_directory(&self, conn_str: &str, dir_path: &str) -> Result<(), RefineryError> {
        // Connect to the database
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await
            .map_err(|e| RefineryError::ConnectError(Box::new(e)))?;
        
        // The connection needs to be running for the client to work
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Create configuration
        let mut config = Config::new(ConfigDbType::Postgres);
        config.set_migrations_path(Path::new(dir_path));
        
        // Create runner
        let runner = Runner::new(&config).build()
            .map_err(|e| RefineryError::ConfigError(e.to_string()))?;
            
        // Run migrations
        runner.run_async(&mut ClientWrapper(client))
            .await
            .map_err(|e| RefineryError::ExecutionError(format!("Migration failed: {}", e)))
    }
    
    /// Load manually created migrations from a directory
    pub fn load_migrations_from_directory(&self, dir_path: &str) -> Result<Vec<Migration>, RefineryError> {
        let path = PathBuf::from(dir_path);
        
        if !path.exists() || !path.is_dir() {
            return Err(RefineryError::ConfigError(format!("Invalid directory path: {}", dir_path)));
        }
        
        let mut migrations = Vec::new();
        
        for entry in fs::read_dir(path)
            .map_err(|e| RefineryError::ConfigError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry.map_err(|e| RefineryError::ConfigError(format!("Failed to read directory entry: {}", e)))?;
            let file_path = entry.path();
            
            if file_path.is_file() && file_path.extension().map_or(false, |ext| ext == "sql") {
                let file_name = file_path.file_name().unwrap().to_string_lossy();
                
                // Extract version and name from the filename (V1__name.sql)
                if let Some(version_name) = extract_version_name(&file_name) {
                    let (version, name) = version_name;
                    
                    // Read the SQL content
                    let sql = fs::read_to_string(&file_path)
                        .map_err(|e| RefineryError::ConfigError(format!("Failed to read file {}: {}", file_path.display(), e)))?;
                    
                    let migration = Migration::new(version, name, &sql)
                        .map_err(|e| RefineryError::ConfigError(format!("Failed to create migration: {}", e)))?;
                    
                    migrations.push(migration);
                }
            }
        }
        
        // Sort migrations by version
        migrations.sort_by_key(|m| m.version);
        
        Ok(migrations)
    }
}

/// Extract version and name from a migration filename
fn extract_version_name(filename: &str) -> Option<(i32, String)> {
    // Format: V1__name.sql
    let parts: Vec<&str> = filename.split("__").collect();
    
    if parts.len() != 2 {
        return None;
    }
    
    // Extract version number without the V prefix
    let version_str = parts[0].strip_prefix('V')?;
    let version = version_str.parse::<i32>().ok()?;
    
    // Extract name without the .sql suffix
    let name_with_ext = parts[1];
    let name = name_with_ext.strip_suffix(".sql")?.to_string();
    
    Some((version, name))
} 