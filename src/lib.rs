//! Supabase Rust Client Library
//!
//! A Rust client library for Supabase, providing access to Supabase services including
//! database, auth, storage, and realtime subscriptions.
//!
//! This is a wrapper around individual Supabase component crates that provides
//! a unified and convenient API that matches the JavaScript supabase-js client.

// 各コンポーネントクレートの再エクスポート
pub use supabase_auth as auth;
pub use supabase_postgrest as postgrest;
pub use supabase_storage as storage;
pub use supabase_realtime as realtime;
pub use supabase_functions as functions;

// 内部モジュール
mod config;
mod error;

// 公開エクスポート
pub use config::ClientOptions;
pub use error::{Error, Result};

use reqwest::Client;

/// The main entry point for the Supabase Rust client
pub struct Supabase {
    /// The base URL for the Supabase project
    pub url: String,
    /// The anonymous API key for the Supabase project
    pub key: String,
    /// HTTP client used for requests
    pub http_client: Client,
    /// Auth client for user management and authentication
    pub auth: auth::Auth,
    /// Client options
    pub options: config::ClientOptions,
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
    /// use supabase_rust::{Supabase, ClientOptions};
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
        
        let auth_options = auth::AuthOptions {
            auto_refresh_token: options.auto_refresh_token,
            persist_session: options.persist_session,
            detect_session_in_url: options.detect_session_in_url,
        };
        
        let auth = auth::Auth::new(supabase_url, supabase_key, http_client.clone(), auth_options);
        
        Self {
            url: supabase_url.to_string(),
            key: supabase_key.to_string(),
            http_client,
            auth,
            options,
        }
    }
    
    /// Get a reference to the auth client for user management and authentication
    pub fn auth(&self) -> &auth::Auth {
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
    pub fn from(&self, table: &str) -> postgrest::PostgrestClient {
        postgrest::PostgrestClient::new(
            &self.url,
            &self.key,
            table,
            self.http_client.clone(),
        )
    }
    
    /// Get a storage client for file operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let storage = supabase.storage();
    /// ```
    pub fn storage(&self) -> storage::StorageClient {
        storage::StorageClient::new(&self.url, &self.key, self.http_client.clone())
    }
    
    /// Get a realtime client for subscription operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let realtime = supabase.realtime();
    /// ```
    pub fn realtime(&self) -> realtime::RealtimeClient {
        realtime::RealtimeClient::new(&self.url, &self.key)
    }
    
    /// Get a functions client for edge function operations
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let functions = supabase.functions();
    /// ```
    pub fn functions(&self) -> functions::FunctionsClient {
        functions::FunctionsClient::new(&self.url, &self.key, self.http_client.clone())
    }
    
    /// Execute a Postgres function via RPC
    ///
    /// # Arguments
    ///
    /// * `function_name` - The name of the function to call
    /// * `params` - Parameters to pass to the function
    ///
    /// # Example
    ///
    /// ```
    /// use supabase_rust::Supabase;
    /// use serde_json::json;
    ///
    /// let supabase = Supabase::new("https://your-project-url.supabase.co", "your-anon-key");
    /// let result = supabase.rpc("calculate_total", json!({"user_id": 123}));
    /// ```
    pub fn rpc(&self, function_name: &str, params: serde_json::Value) -> postgrest::PostgrestClient {
        postgrest::PostgrestClient::rpc(
            &self.url,
            &self.key,
            function_name,
            params,
            self.http_client.clone()
        )
    }
}

/// A convenience module for common imports
pub mod prelude {
    pub use crate::Supabase;
    pub use crate::error::{Error, Result};
    pub use crate::config::ClientOptions;
    pub use crate::postgrest::{IsolationLevel, TransactionMode};
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;
    
    #[tokio::test]
    async fn test_integration() {
        let mock_server = MockServer::start().await;
        
        // Auth モックエンドポイント
        Mock::given(method("POST"))
            .and(path("/auth/v1/token"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "access_token": "test_token",
                    "refresh_token": "test_refresh",
                    "expires_in": 3600,
                    "token_type": "bearer",
                    "user": {
                        "id": "1234",
                        "email": "test@example.com"
                    }
                }))
            )
            .mount(&mock_server)
            .await;
        
        // Database モックエンドポイント
        Mock::given(method("GET"))
            .and(path("/rest/v1/users"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([
                    { "id": 1, "name": "Test User" }
                ]))
            )
            .mount(&mock_server)
            .await;
        
        // Storage モックエンドポイント
        Mock::given(method("GET"))
            .and(path("/storage/v1/bucket"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!([
                    { "id": "test-bucket", "name": "test-bucket", "public": true, "owner": "owner", "created_at": "2023-01-01", "updated_at": "2023-01-01" }
                ]))
            )
            .mount(&mock_server)
            .await;
        
        let supabase = Supabase::new(&mock_server.uri(), "test_key");
        
        // Database操作のテスト
        let users: Vec<serde_json::Value> = supabase
            .from("users")
            .select("*")
            .execute()
            .await
            .unwrap();
        
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["name"], "Test User");
        
        // Storageバケット一覧取得のテスト
        let buckets = supabase.storage().list_buckets().await.unwrap();
        
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, "test-bucket");
    }
}