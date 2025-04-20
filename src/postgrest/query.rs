//! Query builder for PostgrestClient

use reqwest::{Client, Method};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

use crate::error::Error;
use crate::fetch::Fetch;
use crate::postgrest::filter::*;

/// Base query builder
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    /// Query parameters
    params: HashMap<String, String>,
}

impl QueryBuilder {
    /// Create a new QueryBuilder
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }
    
    /// Add a parameter to the query
    pub fn add_param(&mut self, key: &str, value: &str) {
        self.params.insert(key.to_string(), value.to_string());
    }
    
    /// Get the query parameters
    pub fn get_params(&self) -> &HashMap<String, String> {
        &self.params
    }
}

/// Builder for SELECT queries
pub struct SelectBuilder {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// The columns to select
    columns: String,
    
    /// HTTP client
    client: Client,
    
    /// Query builder
    query: QueryBuilder,
}

impl SelectBuilder {
    /// Create a new SelectBuilder
    pub fn new(url: String, key: String, columns: &str, client: Client) -> Self {
        let mut query = QueryBuilder::new();
        query.add_param("select", columns);
        
        Self {
            url,
            key,
            columns: columns.to_string(),
            client,
            query,
        }
    }
    
    /// Filter rows where column equals a value
    pub fn eq<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("eq.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column does not equal a value
    pub fn neq<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("neq.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column is greater than a value
    pub fn gt<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("gt.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column is greater than or equal to a value
    pub fn gte<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("gte.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column is less than a value
    pub fn lt<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("lt.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column is less than or equal to a value
    pub fn lte<T: ToString>(&mut self, column: &str, value: T) -> &mut Self {
        let filter = format!("lte.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column matches a pattern (case sensitive)
    pub fn like(&mut self, column: &str, pattern: &str) -> &mut Self {
        let filter = format!("like.{}", pattern);
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column matches a pattern (case insensitive)
    pub fn ilike(&mut self, column: &str, pattern: &str) -> &mut Self {
        let filter = format!("ilike.{}", pattern);
        self.query.add_param(column, &filter);
        self
    }
    
    /// Filter rows where column is in a list of values
    pub fn in_list<T: ToString>(&mut self, column: &str, values: &[T]) -> &mut Self {
        let values_str: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        let filter = format!("in.({})", values_str.join(","));
        self.query.add_param(column, &filter);
        self
    }
    
    /// Limit the number of rows returned
    pub fn limit(&mut self, count: i32) -> &mut Self {
        self.query.add_param("limit", &count.to_string());
        self
    }
    
    /// Skip a number of rows
    pub fn offset(&mut self, count: i32) -> &mut Self {
        self.query.add_param("offset", &count.to_string());
        self
    }
    
    /// Order the results by a column
    pub fn order(&mut self, column: &str, ascending: bool) -> &mut Self {
        let direction = if ascending { "asc" } else { "desc" };
        self.query.add_param("order", &format!("{}.{}", column, direction));
        self
    }
    
    /// Retrieve a single row
    pub fn single(&mut self) -> &mut Self {
        self.query.add_param("limit", "1");
        self
    }
    
    /// Execute the query and return the results
    pub async fn execute<T: DeserializeOwned>(&self) -> Result<Vec<T>, Error> {
        let fetch = Fetch::get(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .query(self.query.get_params().clone());
        
        let result = fetch.execute::<Vec<T>>().await?;
        Ok(result)
    }
    
    /// Execute the query and return the first row
    pub async fn execute_one<T: DeserializeOwned>(&mut self) -> Result<Option<T>, Error> {
        self.limit(1);
        
        let results = self.execute::<T>().await?;
        Ok(results.into_iter().next())
    }
}

/// Builder for INSERT queries
pub struct InsertBuilder<T: Serialize> {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// The values to insert
    values: T,
    
    /// HTTP client
    client: Client,
    
    /// Query builder
    query: QueryBuilder,
}

impl<T: Serialize> InsertBuilder<T> {
    /// Create a new InsertBuilder
    pub fn new(url: String, key: String, values: T, client: Client) -> Self {
        Self {
            url,
            key,
            values,
            client,
            query: QueryBuilder::new(),
        }
    }
    
    /// Execute the query and return the results
    pub async fn execute<R: DeserializeOwned>(&self) -> Result<R, Error> {
        let fetch = Fetch::post(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=representation")
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        let result = fetch.execute::<R>().await?;
        Ok(result)
    }
    
    /// Execute the query without returning the inserted data
    pub async fn execute_no_return(&self) -> Result<(), Error> {
        let fetch = Fetch::post(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=minimal")
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        fetch.execute_raw().await?;
        Ok(())
    }
}

/// Builder for UPDATE queries
pub struct UpdateBuilder<T: Serialize> {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// The values to update
    values: T,
    
    /// HTTP client
    client: Client,
    
    /// Query builder
    query: QueryBuilder,
}

impl<T: Serialize> UpdateBuilder<T> {
    /// Create a new UpdateBuilder
    pub fn new(url: String, key: String, values: T, client: Client) -> Self {
        Self {
            url,
            key,
            values,
            client,
            query: QueryBuilder::new(),
        }
    }
    
    /// Filter rows where column equals a value
    pub fn eq<V: ToString>(&mut self, column: &str, value: V) -> &mut Self {
        let filter = format!("eq.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Execute the query and return the results
    pub async fn execute<R: DeserializeOwned>(&self) -> Result<R, Error> {
        let fetch = Fetch::patch(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=representation")
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        let result = fetch.execute::<R>().await?;
        Ok(result)
    }
    
    /// Execute the query without returning the updated data
    pub async fn execute_no_return(&self) -> Result<(), Error> {
        let fetch = Fetch::patch(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=minimal")
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        fetch.execute_raw().await?;
        Ok(())
    }
}

/// Builder for UPSERT queries
pub struct UpsertBuilder<T: Serialize> {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// The values to upsert
    values: T,
    
    /// HTTP client
    client: Client,
    
    /// Query builder
    query: QueryBuilder,
    
    /// On conflict columns
    on_conflict: Option<String>,
}

impl<T: Serialize> UpsertBuilder<T> {
    /// Create a new UpsertBuilder
    pub fn new(url: String, key: String, values: T, client: Client) -> Self {
        Self {
            url,
            key,
            values,
            client,
            query: QueryBuilder::new(),
            on_conflict: None,
        }
    }
    
    /// Specify the column(s) to check for conflicts
    pub fn on_conflict(&mut self, column: &str) -> &mut Self {
        self.on_conflict = Some(column.to_string());
        self
    }
    
    /// Execute the query and return the results
    pub async fn execute<R: DeserializeOwned>(&self) -> Result<R, Error> {
        let mut fetch = Fetch::post(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=representation");
        
        if let Some(ref conflict) = self.on_conflict {
            fetch = fetch.header("Prefer", &format!("resolution=merge-duplicates,on_conflict={}", conflict));
        }
        
        let fetch = fetch
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        let result = fetch.execute::<R>().await?;
        Ok(result)
    }
    
    /// Execute the query without returning the upserted data
    pub async fn execute_no_return(&self) -> Result<(), Error> {
        let mut fetch = Fetch::post(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=minimal");
        
        if let Some(ref conflict) = self.on_conflict {
            fetch = fetch.header("Prefer", &format!("resolution=merge-duplicates,on_conflict={}", conflict));
        }
        
        let fetch = fetch
            .query(self.query.get_params().clone())
            .json(&self.values)?;
        
        fetch.execute_raw().await?;
        Ok(())
    }
}

/// Builder for DELETE queries
pub struct DeleteBuilder {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// HTTP client
    client: Client,
    
    /// Query builder
    query: QueryBuilder,
}

impl DeleteBuilder {
    /// Create a new DeleteBuilder
    pub fn new(url: String, key: String, client: Client) -> Self {
        Self {
            url,
            key,
            client,
            query: QueryBuilder::new(),
        }
    }
    
    /// Filter rows where column equals a value
    pub fn eq<V: ToString>(&mut self, column: &str, value: V) -> &mut Self {
        let filter = format!("eq.{}", value.to_string());
        self.query.add_param(column, &filter);
        self
    }
    
    /// Execute the query and return the deleted data
    pub async fn execute<R: DeserializeOwned>(&self) -> Result<R, Error> {
        let fetch = Fetch::delete(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=representation")
            .query(self.query.get_params().clone());
        
        let result = fetch.execute::<R>().await?;
        Ok(result)
    }
    
    /// Execute the query without returning the deleted data
    pub async fn execute_no_return(&self) -> Result<(), Error> {
        let fetch = Fetch::delete(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .header("Prefer", "return=minimal")
            .query(self.query.get_params().clone());
        
        fetch.execute_raw().await?;
        Ok(())
    }
}

/// Builder for RPC (stored procedure) calls
pub struct RpcBuilder<T: Serialize> {
    /// The base URL for the request
    url: String,
    
    /// The API key
    key: String,
    
    /// The parameters to pass to the function
    params: T,
    
    /// HTTP client
    client: Client,
}

impl<T: Serialize> RpcBuilder<T> {
    /// Create a new RpcBuilder
    pub fn new(url: String, key: String, params: T, client: Client) -> Self {
        Self {
            url,
            key,
            params,
            client,
        }
    }
    
    /// Execute the RPC call and return the results
    pub async fn execute<R: DeserializeOwned>(&self) -> Result<R, Error> {
        let fetch = Fetch::post(&self.client, &self.url)
            .header("apikey", &self.key)
            .header("X-Client-Info", "supabase-rust/0.1.0")
            .json(&self.params)?;
        
        let result = fetch.execute::<R>().await?;
        Ok(result)
    }
}