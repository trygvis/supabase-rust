//! Supabase Rust Client Library
//!
//! A Rust client library for Supabase, providing access to Supabase services including
//! database, auth, storage, and realtime subscriptions.

pub mod auth;
pub mod postgrest;
pub mod storage;
pub mod realtime;
pub mod functions;
pub mod error;
pub mod config;
pub mod fetch;

use std::sync::Arc;
use reqwest::Client;
use url::Url;

use crate::auth::Auth;
use crate::postgrest::PostgrestClient;
use crate::storage::StorageClient;
use crate::realtime::RealtimeClient;
use crate::functions::FunctionsClient;
use crate::config::ClientOptions;

/// The main entry point for the Supabase Rust client
pub struct Supabase {
    /// The base URL for the Supabase project
    pub url: String,
    /// The anonymous API key for the Supabase project
    pub key: String,
    /// HTTP client used for requests
    pub http_client: Client,
    /// Auth client for user management and authentication
    pub auth: Auth,
    /// Client options
    pub options: ClientOptions,
}

impl Supabase {
    /// Create a new Supabase client
    ///
    /// # Arguments
    ///
    /// * `supabase_url` - The base URL for your Supabase project
    /// * `supabase_key` - The anonymous API key for your Supabase project
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// ```
    pub fn new(supabase_url: &str, supabase_key: &str) -> Self {
        Self::new_with_options(supabase_url, supabase_key, ClientOptions::default())
    }

    /// Create a new Supabase client with custom options
    ///
    /// # Arguments
    ///
    /// * `supabase_url` - The base URL for your Supabase project
    /// * `supabase_key` - The anonymous API key for your Supabase project
    /// * `options` - Custom client options
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::{Supabase, config::ClientOptions};
    ///
    /// let options = ClientOptions::default().with_auto_refresh_token(true);
    /// let supabase = Supabase::new_with_options(
    ///     "https://your-project-url.supabase.co",
    ///     "your-anon-key",
    ///     options
    /// );
    /// ```
    pub fn new_with_options(supabase_url: &str, supabase_key: &str, options: ClientOptions) -> Self {
        let http_client = Client::new();
        
        let auth = Auth::new(supabase_url, supabase_key, http_client.clone(), options.clone());
        
        Self {
            url: supabase_url.to_string(),
            key: supabase_key.to_string(),
            http_client,
            auth,
            options,
        }
    }
    
    /// Get a reference to the auth client for user management and authentication
    pub fn auth(&self) -> &Auth {
        &self.auth
    }
    
    /// Create a new PostgrestClient for database operations on a specific table or view
    ///
    /// # Arguments
    ///
    /// * `table` - The name of the table or view
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let query = supabase.from("users");
    /// ```
    pub fn from(&self, table: &str) -> PostgrestClient {
        PostgrestClient::new(
            &self.url,
            &self.key,
            table,
            self.http_client.clone(),
        )
    }
    
    /// Get a reference to the storage client for file operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let storage = supabase.storage();
    /// ```
    pub fn storage(&self) -> StorageClient {
        StorageClient::new(&self.url, &self.key, self.http_client.clone())
    }
    
    /// Get a reference to the realtime client for subscription operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let realtime = supabase.realtime();
    /// ```
    pub fn realtime(&self) -> RealtimeClient {
        RealtimeClient::new(&self.url, &self.key)
    }
    
    /// Get a reference to the functions client for edge function operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let functions = supabase.functions();
    /// ```
    pub fn functions(&self) -> FunctionsClient {
        FunctionsClient::new(&self.url, &self.key, self.http_client.clone())
    }
}

/// A convenience module for common imports
pub mod prelude {
    pub use crate::Supabase;
    pub use crate::error::Error;
    pub use crate::config::ClientOptions;
}