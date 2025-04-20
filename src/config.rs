//! Configuration options for the Supabase client

use std::time::Duration;

/// Configuration options for the Supabase client
#[derive(Debug, Clone)]
pub struct ClientOptions {
    /// Whether to automatically refresh the token
    pub auto_refresh_token: bool,
    
    /// The persist session key
    pub persist_session: bool,
    
    /// The request timeout
    pub request_timeout: Option<Duration>,
    
    /// The database schema
    pub db_schema: String,
    
    /// The storage schema
    pub storage_schema: String,
    
    /// The auth schema
    pub auth_schema: String,
    
    /// The realtime schema
    pub realtime_schema: String,
    
    /// The functions schema
    pub functions_schema: String,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            auto_refresh_token: true,
            persist_session: true,
            request_timeout: Some(Duration::from_secs(30)),
            db_schema: "public".to_string(),
            storage_schema: "storage".to_string(),
            auth_schema: "auth".to_string(),
            realtime_schema: "realtime".to_string(),
            functions_schema: "functions".to_string(),
        }
    }
}

impl ClientOptions {
    /// Set whether to automatically refresh the token
    pub fn with_auto_refresh_token(mut self, value: bool) -> Self {
        self.auto_refresh_token = value;
        self
    }
    
    /// Set whether to persist the session
    pub fn with_persist_session(mut self, value: bool) -> Self {
        self.persist_session = value;
        self
    }
    
    /// Set the request timeout
    pub fn with_request_timeout(mut self, value: Option<Duration>) -> Self {
        self.request_timeout = value;
        self
    }
    
    /// Set the database schema
    pub fn with_db_schema(mut self, value: &str) -> Self {
        self.db_schema = value.to_string();
        self
    }
    
    /// Set the storage schema
    pub fn with_storage_schema(mut self, value: &str) -> Self {
        self.storage_schema = value.to_string();
        self
    }
    
    /// Set the auth schema
    pub fn with_auth_schema(mut self, value: &str) -> Self {
        self.auth_schema = value.to_string();
        self
    }
    
    /// Set the realtime schema
    pub fn with_realtime_schema(mut self, value: &str) -> Self {
        self.realtime_schema = value.to_string();
        self
    }
    
    /// Set the functions schema
    pub fn with_functions_schema(mut self, value: &str) -> Self {
        self.functions_schema = value.to_string();
        self
    }
}