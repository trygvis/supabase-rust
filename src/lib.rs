//! Supabase Rust Client Library
//!
//! A Rust client library for Supabase, providing access to Supabase services including
//! database, auth, storage, and realtime subscriptions.
//!
//! This is a wrapper around individual Supabase component crates that provides
//! a unified and convenient API that matches the JavaScript supabase-js client.

// 各コンポーネントクレートの再エクスポート
pub use supabase_rust_auth as auth;
pub use supabase_rust_postgrest as postgrest;
// pub use supabase_rust_storage as storage;
// pub use supabase_rust_realtime as realtime;
// pub use supabase_rust_functions as functions;

// 内部モジュール
mod config;
mod error;

// 公開エクスポート
pub use config::ClientOptions;
pub use error::{Error, Result};

use reqwest::Client;
// 使われていないインポートを削除
// use reqwest::header::{HeaderMap, HeaderValue, HeaderName};
// use serde::{Serialize, Deserialize};
// use serde_json::Value;
// use std::collections::HashMap;
// use thiserror::Error;
// use url::Url;
// use serde_json::json;
// use std::sync::Arc;
// use std::sync::atomic::{AtomicBool, Ordering};
use serde::Serialize;

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
    
    /// Create a client for the Storage API
    pub fn storage(&self) -> /* storage::StorageClient */ () {
        /* storage::StorageClient::new(&self.url, &self.key, self.http_client.clone()) */
        // Storage client is temporarily disabled until the crate is published
        panic!("Storage client is not available in this version")
    }
    
    /// Create a client for the Realtime API
    pub fn realtime(&self) -> /* realtime::RealtimeClient */ () {
        /* realtime::RealtimeClient::new(&self.url, &self.key) */
        // Realtime client is temporarily disabled until the crate is published
        panic!("Realtime client is not available in this version")
    }
    
    /// Create a client for the Edge Functions API
    pub fn functions(&self) -> /* functions::FunctionsClient */ () {
        /* functions::FunctionsClient::new(&self.url, &self.key, self.http_client.clone()) */
        // Functions client is temporarily disabled until the crate is published
        panic!("Functions client is not available in this version")
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

    /// eq演算子による簡便なフィルター追加メソッド
    pub fn eq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Eq,
            value.into()
        ))
    }

    /// neq演算子による簡便なフィルター追加メソッド
    pub fn neq<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Neq,
            value.into()
        ))
    }

    /// gt演算子による簡便なフィルター追加メソッド
    pub fn gt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(), 
            FilterOperator::Gt,
            value.into()
        ))
    }

    /// gte演算子による簡便なフィルター追加メソッド
    pub fn gte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Gte,
            value.into()
        ))
    }

    /// lt演算子による簡便なフィルター追加メソッド
    pub fn lt<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Lt,
            value.into()
        ))
    }

    /// lte演算子による簡便なフィルター追加メソッド
    pub fn lte<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Lte,
            value.into()
        ))
    }

    /// in演算子による簡便なフィルター追加メソッド（配列の中に含まれるか）
    pub fn in_values<T: Into<serde_json::Value>>(self, column: &str, values: Vec<T>) -> Self {
        let json_values = values.into_iter().map(|v| v.into()).collect();
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::In,
            serde_json::Value::Array(json_values)
        ))
    }

    /// contains演算子による簡便なフィルター追加メソッド（配列が値を含むか）
    pub fn contains<T: Into<serde_json::Value>>(self, column: &str, value: T) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Contains,
            value.into()
        ))
    }

    /// like演算子による簡便なフィルター追加メソッド（ワイルドカード検索）
    pub fn like(self, column: &str, pattern: &str) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::Like,
            serde_json::Value::String(pattern.to_string())
        ))
    }

    /// ilike演算子による簡便なフィルター追加メソッド（大文字小文字を区別しないワイルドカード検索）
    pub fn ilike(self, column: &str, pattern: &str) -> Self {
        self.filter(DatabaseFilter::new(
            column.to_string(),
            FilterOperator::ILike,
            serde_json::Value::String(pattern.to_string())
        ))
    }
}

/// A convenience module for common imports
pub mod prelude {
    pub use crate::Supabase;
    pub use crate::error::{Error, Result};
    pub use crate::config::ClientOptions;
    pub use crate::postgrest::{IsolationLevel, TransactionMode};
}

/// フィルター演算子
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperator {
    /// 等しい
    Eq,
    /// 等しくない
    Neq,
    /// より大きい
    Gt,
    /// より大きいか等しい
    Gte,
    /// より小さい
    Lt,
    /// より小さいか等しい
    Lte,
    /// 含む
    In,
    /// 含まない
    NotIn,
    /// 近い値（配列内の値に対して）
    ContainedBy,
    /// 含む（配列が対象の値を含む）
    Contains,
    /// 完全に含む（配列が対象の配列のすべての要素を含む）
    ContainedByArray,
    /// LIKE演算子（ワイルドカード検索）
    Like,
    /// ILIKE演算子（大文字小文字を区別しないワイルドカード検索）
    ILike,
}

impl ToString for FilterOperator {
    fn to_string(&self) -> String {
        match self {
            FilterOperator::Eq => "eq".to_string(),
            FilterOperator::Neq => "neq".to_string(),
            FilterOperator::Gt => "gt".to_string(),
            FilterOperator::Gte => "gte".to_string(),
            FilterOperator::Lt => "lt".to_string(),
            FilterOperator::Lte => "lte".to_string(),
            FilterOperator::In => "in".to_string(),
            FilterOperator::NotIn => "not.in".to_string(),
            FilterOperator::ContainedBy => "contained_by".to_string(),
            FilterOperator::Contains => "contains".to_string(),
            FilterOperator::ContainedByArray => "contained_by_array".to_string(),
            FilterOperator::Like => "like".to_string(),
            FilterOperator::ILike => "ilike".to_string(),
        }
    }
}

/// データベース変更に対するフィルター条件
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseFilter {
    /// フィルター対象のカラム名
    pub column: String,
    /// 比較演算子
    #[serde(rename = "operator")]
    pub operator_str: String,
    /// 比較する値
    pub value: serde_json::Value,
}

impl DatabaseFilter {
    pub fn new(column: String, operator: FilterOperator, value: serde_json::Value) -> Self {
        Self {
            column,
            operator_str: operator.to_string(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, query_param};
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