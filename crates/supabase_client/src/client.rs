// src/client.rs

// Reverting to original structure with v0.2.0 path dependencies
// and stubbing out problematic implementations.

use crate::error::{Result, SupabaseError};
use crate::models::{AuthCredentials, Item, User};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// Correct imports based on crate structure
use reqwest::Client as ReqwestClient;
use supabase_rust_auth::AuthOptions;
use supabase_rust_auth::{Auth, AuthError, Session as AuthSession};
use supabase_rust_postgrest::PostgrestClient;
use supabase_rust_realtime::RealtimeClient;

use tokio::sync::{mpsc, Mutex};
use url::Url;
use uuid::Uuid;

/// Configuration for the Supabase client.
/// It's recommended to load these values from environment variables or a secure config source.
#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub url: Url,
    pub anon_key: String,
}

impl SupabaseConfig {
    /// Creates a new configuration, validating the URL.
    pub fn new(url_str: &str, anon_key: String) -> Result<Self> {
        let url = Url::parse(url_str).map_err(SupabaseError::UrlParse)?;
        if anon_key.is_empty() {
            return Err(SupabaseError::Config(
                "anon_key cannot be empty".to_string(),
            ));
        }
        Ok(Self { url, anon_key })
    }

    /// Attempts to create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let url_str = std::env::var("SUPABASE_URL").map_err(|_| {
            SupabaseError::Config("SUPABASE_URL environment variable not found".to_string())
        })?;
        let anon_key = std::env::var("SUPABASE_ANON_KEY").map_err(|_| {
            SupabaseError::Config("SUPABASE_ANON_KEY environment variable not found".to_string())
        })?;
        Self::new(&url_str, anon_key)
    }
}

/// Represents the different types of changes received from a realtime subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemChange {
    Insert(Item),
    Update(Item),
    Delete(HashMap<String, Value>),
    Error(String),
}

/// The main Supabase client wrapper, providing access to different Supabase features.
/// This struct holds initialized clients for Auth, Postgrest, etc., and manages the current session.
#[derive(Clone)]
pub struct SupabaseClientWrapper {
    pub auth: Arc<Auth>,
    pub db: Arc<PostgrestClient>,
    pub realtime: Arc<RealtimeClient>,
    #[allow(dead_code)] // Allowed because methods using it are stubbed
    current_session: Arc<Mutex<Option<AuthSession>>>,
}

impl SupabaseClientWrapper {
    /// Creates a new Supabase client wrapper.
    /// Initializes Auth and Postgrest clients based on the provided config.
    /// Corresponds to the `initializeSupabaseClient` step in the SSOT.
    pub fn new(config: SupabaseConfig) -> Result<Self> {
        let http_client_auth = ReqwestClient::new();
        let auth_client = Auth::new(
            config.url.as_ref(), // Use as_ref()
            &config.anon_key,
            http_client_auth,
            AuthOptions::default(),
        );

        let rest_url = config.url.join("rest/v1/").map_err(|e| {
            SupabaseError::Initialization(format!("Failed to construct PostgREST URL: {}", e))
        })?;

        let http_client_db = ReqwestClient::new();
        let db_client = PostgrestClient::new(
            rest_url.as_ref(), // Use as_ref()
            &config.anon_key,
            "unknown",
            http_client_db,
        );

        // Realtime init
        let mut rt_url_builder = config.url.clone();
        let scheme = if config.url.scheme() == "https" {
            "wss"
        } else {
            "ws"
        };
        rt_url_builder.set_scheme(scheme).map_err(|_| {
            SupabaseError::Initialization("Failed to set scheme for Realtime URL".to_string())
        })?;
        let rt_url = rt_url_builder.join("realtime/v1").map_err(|e| {
            SupabaseError::Initialization(format!("Failed to construct Realtime URL: {}", e))
        })?;
        let realtime_client = RealtimeClient::new(rt_url.as_ref(), &config.anon_key); // Use as_ref()

        println!("Supabase client initialized (Auth & Postgrest & Realtime - v0.2.0 attempt 2).");

        Ok(Self {
            auth: Arc::new(auth_client),
            db: Arc::new(db_client),
            realtime: Arc::new(realtime_client),
            current_session: Arc::new(Mutex::new(None)),
        })
    }

    /// Convenience function to create a client directly from environment variables.
    pub fn from_env() -> Result<Self> {
        let config = SupabaseConfig::from_env()?;
        Self::new(config)
    }

    // --- Stub out methods causing compilation errors ---

    /// Authenticates a user using email and password.
    /// Corresponds to `authenticateUser` in the SSOT.
    /// Returns the Supabase User details on success.
    pub async fn authenticate(&self, _credentials: AuthCredentials) -> Result<User> {
        println!("[STUB] Attempting to authenticate user");
        unimplemented!("Authentication logic needs fixing for v0.2.0 API");
    }

    /// Logs out the currently authenticated user by invalidating the session/token.
    /// Corresponds to `logoutUser` in the SSOT.
    pub async fn logout(&self) -> Result<()> {
        println!("[STUB] Attempting to log out user");
        unimplemented!("Logout logic needs fixing for v0.2.0 API");
    }

    /// Fetches 'items' from the database.
    /// Assumes the user is authenticated and uses the stored session token.
    /// Relies on RLS being configured in Supabase for the 'items' table.
    /// Corresponds to `fetchItemsFromSupabase` in the SSOT.
    pub async fn fetch_items(&self) -> Result<Vec<Item>> {
        println!("[STUB] Attempting to fetch items");
        unimplemented!("Postgrest fetch logic needs fixing for v0.2.0 API");
    }

    /// Subscribes to item changes.
    /// Corresponds to `subscribeToItemChanges` in the SSOT.
    pub async fn subscribe_to_item_changes(&self) -> Result<mpsc::UnboundedReceiver<ItemChange>> {
        println!("[STUB] Attempting to subscribe to item changes");
        unimplemented!("Realtime subscription logic needs fixing for v0.2.0 API");
    }

    // --- CRUD Operations for Items ---

    /// Creates a new item in the database.
    /// Requires authentication.
    pub async fn create_item(&self, _new_item: Item) -> Result<Item> {
        println!("[STUB] Attempting to create item");
        unimplemented!("Postgrest insert logic needs fixing for v0.2.0 API");
    }

    /// Fetches a single 'item' by its ID.
    pub async fn fetch_item_by_id(&self, _item_id: Uuid) -> Result<Option<Item>> {
        println!("[STUB] Attempting to fetch item by ID");
        unimplemented!("Postgrest fetch single logic needs fixing for v0.2.0 API");
    }

    /// Updates an existing 'item' by its ID.
    pub async fn update_item(&self, _item_id: Uuid, _item_update: Item) -> Result<Item> {
        println!("[STUB] Attempting to update item");
        unimplemented!("Postgrest update logic needs fixing for v0.2.0 API");
    }

    /// Deletes an 'item' by its ID.
    pub async fn delete_item(&self, _item_id: Uuid) -> Result<()> {
        println!("[STUB] Attempting to delete item");
        unimplemented!("Postgrest delete logic needs fixing for v0.2.0 API");
    }

    #[allow(dead_code)] // Allowed because methods using it are stubbed
    async fn get_auth_token(&self) -> Result<String> {
        let session_guard = self.current_session.lock().await;
        session_guard
            .as_ref()
            .map(|s| s.access_token.clone()) // Use map() instead of and_then(Some())
            .ok_or_else(|| {
                SupabaseError::Auth(AuthError::ApiError("Missing session token".to_string()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module
    use dotenv::dotenv;

    #[test]
    fn config_new_valid() {
        dotenv().ok(); // Load .env file for testing
        let url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set for tests");
        let key =
            std::env::var("SUPABASE_ANON_KEY").expect("SUPABASE_ANON_KEY must be set for tests");
        let config = SupabaseConfig::new(&url, key.clone()).unwrap();
        // Fix: Url::parse adds a trailing slash if missing path, format! needs literal and arg
        assert_eq!(config.url.to_string(), format!("{}/", url));
        assert_eq!(config.anon_key, key);
    }

    #[test]
    fn config_new_invalid_url() {
        let url = "not a valid url";
        let key = "some_anon_key";
        let config = SupabaseConfig::new(url, key.to_string());
        assert!(config.is_err());
        match config.err().unwrap() {
            SupabaseError::UrlParse(_) => {} // Expected error
            _ => panic!("Expected UrlParse error"),
        }
    }

    #[test]
    fn config_new_empty_key() {
        let url = "http://localhost:54321";
        let key = "";
        let config = SupabaseConfig::new(url, key.to_string());
        assert!(config.is_err());
        match config.err().unwrap() {
            SupabaseError::Config(msg) => assert!(msg.contains("anon_key cannot be empty")),
            _ => panic!("Expected Config error for empty key"),
        }
    }

    // Add tests for SupabaseConfig::from_env() - requires setting env vars for test
    // This might be better suited for integration tests or require helper libraries.
}
