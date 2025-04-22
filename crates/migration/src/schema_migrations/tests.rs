//! Tests for schema migrations

#[cfg(test)]
mod tests {
    use super::super::DirectoryMigrator;
    use std::path::Path;
    
    #[test]
    fn test_extract_version_name() {
        // Call the function directly from parent module
        let result = super::extract_version_name("V1__initial_schema.sql");
        assert_eq!(result, Some((1, "initial_schema".to_string())));
        
        let result = super::extract_version_name("V42__add_new_table.sql");
        assert_eq!(result, Some((42, "add_new_table".to_string())));
        
        // Test with invalid formats
        let result = super::extract_version_name("invalid_filename.sql");
        assert_eq!(result, None);
        
        let result = super::extract_version_name("V1_invalid_separator.sql");
        assert_eq!(result, None);
        
        let result = super::extract_version_name("1__missing_v_prefix.sql");
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_load_migrations_from_directory() {
        // This test assumes the presence of the SQL files we created
        let migrator = DirectoryMigrator;
        
        // Use relative path to the sql directory
        let dir_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/schema_migrations/sql")
            .to_string_lossy()
            .to_string();
        
        // Test if migrations can be loaded
        let result = migrator.load_migrations_from_directory(&dir_path);
        assert!(result.is_ok(), "Failed to load migrations: {:?}", result.err());
        
        let migrations = result.unwrap();
        assert!(!migrations.is_empty(), "No migrations were loaded");
        
        // Verify migrations are sorted by version
        for i in 1..migrations.len() {
            assert!(migrations[i].version > migrations[i-1].version, 
                   "Migrations are not sorted: {} <= {}", 
                   migrations[i].version, migrations[i-1].version);
        }
        
        // Verify migration content
        for migration in migrations {
            assert!(!migration.sql.is_empty(), "Migration SQL is empty");
            assert!(!migration.name.is_empty(), "Migration name is empty");
        }
    }
} 