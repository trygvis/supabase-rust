//! Schema migrations based on Postgrest schema information
//!
//! This module provides functionality to generate migrations from schema information
//! obtained from the Postgrest crate.

use crate::utils::{RlsCommand, RlsPolicy, RlsRole};
use refinery::Migration;
use refinery::error::Error as RefineryError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database table definition in schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDefinition {
    /// Table name
    pub name: String,
    /// Table columns
    pub columns: Vec<ColumnDefinition>,
    /// Table constraints
    pub constraints: Vec<ConstraintDefinition>,
    /// RLS policies
    pub rls_policies: Vec<RlsPolicyDefinition>,
    /// Enable RLS
    pub enable_rls: bool,
    /// Force RLS
    pub force_rls: bool,
}

/// Column definition in a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: String,
    /// Nullable
    pub nullable: bool,
    /// Default value
    pub default_value: Option<String>,
    /// Primary key
    pub primary_key: bool,
}

/// Constraint definition in a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintDefinition {
    /// Primary key constraint
    PrimaryKey {
        /// Constraint name
        name: String,
        /// Columns
        columns: Vec<String>,
    },
    /// Foreign key constraint
    ForeignKey {
        /// Constraint name
        name: String,
        /// Columns
        columns: Vec<String>,
        /// Referenced table
        ref_table: String,
        /// Referenced columns
        ref_columns: Vec<String>,
        /// On delete action
        on_delete: String,
        /// On update action
        on_update: String,
    },
    /// Unique constraint
    Unique {
        /// Constraint name
        name: String,
        /// Columns
        columns: Vec<String>,
    },
    /// Check constraint
    Check {
        /// Constraint name
        name: String,
        /// Check expression
        expression: String,
    },
}

/// RLS policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlsPolicyDefinition {
    /// Policy name
    pub name: String,
    /// Command (SELECT, INSERT, UPDATE, DELETE, ALL)
    pub command: String,
    /// Role (PUBLIC, AUTHENTICATED, ANON, or custom role)
    pub role: String,
    /// Using expression
    pub using: String,
    /// Check expression
    pub check: Option<String>,
}

/// Schema migration generator
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// Generate migration SQL from table definitions
    pub fn generate_migration_sql(tables: &[TableDefinition]) -> String {
        let mut sql = String::new();
        
        // Generate table creation SQL
        for table in tables {
            sql.push_str(&Self::generate_table_sql(table));
            sql.push_str("\n\n");
        }
        
        // Generate constraints SQL (separate to avoid circular dependencies)
        for table in tables {
            sql.push_str(&Self::generate_constraints_sql(table));
            sql.push_str("\n\n");
        }
        
        // Generate RLS policy SQL
        for table in tables {
            sql.push_str(&Self::generate_rls_sql(table));
            sql.push_str("\n\n");
        }
        
        sql
    }
    
    /// Generate SQL for creating a table
    fn generate_table_sql(table: &TableDefinition) -> String {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table.name);
        
        // Add columns
        let column_definitions: Vec<String> = table.columns.iter()
            .map(|col| Self::generate_column_sql(col))
            .collect();
        
        sql.push_str(&column_definitions.join(",\n"));
        
        // Add primary key constraint inline if present
        for constraint in &table.constraints {
            if let ConstraintDefinition::PrimaryKey { name, columns } = constraint {
                if !columns.is_empty() {
                    sql.push_str(",\n");
                    sql.push_str(&format!("    CONSTRAINT {} PRIMARY KEY ({})",
                                       name, columns.join(", ")));
                }
            }
        }
        
        sql.push_str("\n);");
        sql
    }
    
    /// Generate SQL for a column
    fn generate_column_sql(column: &ColumnDefinition) -> String {
        let mut sql = format!("    {} {}", column.name, column.data_type);
        
        if !column.nullable {
            sql.push_str(" NOT NULL");
        }
        
        if column.primary_key {
            sql.push_str(" PRIMARY KEY");
        }
        
        if let Some(default) = &column.default_value {
            sql.push_str(&format!(" DEFAULT {}", default));
        }
        
        sql
    }
    
    /// Generate SQL for constraints
    fn generate_constraints_sql(table: &TableDefinition) -> String {
        let mut sql = String::new();
        
        for constraint in &table.constraints {
            match constraint {
                // Skip primary key as it's already added inline
                ConstraintDefinition::PrimaryKey { .. } => {},
                
                ConstraintDefinition::ForeignKey { name, columns, ref_table, ref_columns, on_delete, on_update } => {
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) \
                         REFERENCES {} ({}) ON DELETE {} ON UPDATE {};\n",
                        table.name, name, columns.join(", "), 
                        ref_table, ref_columns.join(", "), on_delete, on_update
                    ));
                },
                
                ConstraintDefinition::Unique { name, columns } => {
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});\n",
                        table.name, name, columns.join(", ")
                    ));
                },
                
                ConstraintDefinition::Check { name, expression } => {
                    sql.push_str(&format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} CHECK ({});\n",
                        table.name, name, expression
                    ));
                },
            }
        }
        
        sql
    }
    
    /// Generate SQL for RLS policies
    fn generate_rls_sql(table: &TableDefinition) -> String {
        let mut sql = String::new();
        
        if table.enable_rls {
            sql.push_str(&format!("ALTER TABLE {} ENABLE ROW LEVEL SECURITY;\n", table.name));
            
            if table.force_rls {
                sql.push_str(&format!("ALTER TABLE {} FORCE ROW LEVEL SECURITY;\n", table.name));
            }
            
            for policy in &table.rls_policies {
                // Convert from RlsPolicyDefinition to RlsPolicy
                let command = match policy.command.as_str() {
                    "SELECT" => RlsCommand::Select,
                    "INSERT" => RlsCommand::Insert,
                    "UPDATE" => RlsCommand::Update,
                    "DELETE" => RlsCommand::Delete,
                    _ => RlsCommand::All,
                };
                
                let role = match policy.role.as_str() {
                    "PUBLIC" => RlsRole::Public,
                    "AUTHENTICATED" => RlsRole::Authenticated,
                    "ANON" => RlsRole::Anon,
                    custom => RlsRole::CustomRole(custom.to_string()),
                };
                
                let rls_policy = RlsPolicy {
                    name: policy.name.clone(),
                    table: table.name.clone(),
                    schema: None,
                    command,
                    role,
                    using: policy.using.clone(),
                    check: policy.check.clone(),
                };
                
                sql.push_str(&rls_policy.create_policy_sql());
                sql.push_str("\n");
            }
        }
        
        sql
    }
    
    /// Create a refinery Migration from table definitions
    pub fn create_migration(
        version: i32,
        name: &str,
        tables: &[TableDefinition],
    ) -> Result<Migration, RefineryError> {
        let sql = Self::generate_migration_sql(tables);
        Migration::new(version, name.to_string(), &sql)
            .map_err(|e| RefineryError::ConfigError(format!("Failed to create migration: {}", e)))
    }
    
    /// Import tables from a schema file (in JSON format)
    pub fn import_from_json(json_str: &str) -> Result<Vec<TableDefinition>, RefineryError> {
        serde_json::from_str(json_str)
            .map_err(|e| RefineryError::ConfigError(format!("Failed to parse schema JSON: {}", e)))
    }
    
    /// Generate table definitions from Postgrest schema
    #[cfg(feature = "postgrest-schema")]
    pub fn from_postgrest_schema(schema: &crate::postgrest::schema::PostgrestSchema) -> Vec<TableDefinition> {
        // This would require implementation based on the actual structure of the Postgrest schema
        // This is just a placeholder to show how it would be structured
        unimplemented!("Postgrest schema conversion is not implemented yet")
    }
} 