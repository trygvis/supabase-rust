//! Database operations through the PostgREST API

mod query;
mod types;
mod filter;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Error;
use crate::fetch::Fetch;

pub use query::*;
pub use types::*;
pub use filter::*;

/// Client for database operations
pub struct PostgrestClient {
    /// The base URL for the Supabase project
    url: String,
    
    /// The anonymous API key for the Supabase project
    key: String,
    
    /// The table or view name
    table: String,
    
    /// HTTP client
    client: Client,
    
    /// The current query builder
    query: QueryBuilder,
}

impl PostgrestClient {
    /// Create a new PostgrestClient
    pub(crate) fn new(url: &str, key: &str, table: &str, client: Client) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
            table: table.to_string(),
            client,
            query: QueryBuilder::new(),
        }
    }
    
    /// Get the base URL for REST API requests
    fn get_url(&self) -> String {
        format!("{}/rest/v1/{}", self.url, self.table)
    }
    
    /// Select specific columns from the table
    pub fn select(&self, columns: &str) -> SelectBuilder {
        let url = self.get_url();
        SelectBuilder::new(
            url, 
            self.key.clone(), 
            columns, 
            self.client.clone()
        )
    }
    
    /// Insert data into the table
    pub fn insert<T: Serialize>(&self, values: T) -> InsertBuilder<T> {
        let url = self.get_url();
        InsertBuilder::new(
            url, 
            self.key.clone(), 
            values, 
            self.client.clone()
        )
    }
    
    /// Update data in the table
    pub fn update<T: Serialize>(&self, values: T) -> UpdateBuilder<T> {
        let url = self.get_url();
        UpdateBuilder::new(
            url, 
            self.key.clone(), 
            values, 
            self.client.clone()
        )
    }
    
    /// Upsert data in the table (insert or update if it exists)
    pub fn upsert<T: Serialize>(&self, values: T) -> UpsertBuilder<T> {
        let url = self.get_url();
        UpsertBuilder::new(
            url, 
            self.key.clone(), 
            values, 
            self.client.clone()
        )
    }
    
    /// Delete data from the table
    pub fn delete(&self) -> DeleteBuilder {
        let url = self.get_url();
        DeleteBuilder::new(
            url, 
            self.key.clone(), 
            self.client.clone()
        )
    }
    
    /// Call a stored procedure or function
    pub fn rpc<T: Serialize>(&self, function: &str, params: T) -> RpcBuilder<T> {
        let url = format!("{}/rest/v1/rpc/{}", self.url, function);
        RpcBuilder::new(
            url, 
            self.key.clone(), 
            params, 
            self.client.clone()
        )
    }
}