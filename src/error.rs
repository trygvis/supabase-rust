//! Error types for Supabase Rust client

use thiserror::Error;

/// Error types for the Supabase client
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP error from reqwest
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
    
    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthError(#[from] supabase_rust_auth::AuthError),
    
    /// PostgreSQL REST API error
    #[error("PostgreSQL REST error: {0}")]
    PostgrestError(#[from] supabase_rust_postgrest::PostgrestError),
    
    /// Storage error
    // #[error("Storage error: {0}")]
    // StorageError(#[from] supabase_rust_storage::StorageError),
    
    /// Realtime subscription error
    // #[error("Realtime error: {0}")]
    // RealtimeError(#[from] supabase_rust_realtime::RealtimeError),
    
    /// Edge Functions error
    // #[error("Functions error: {0}")]
    // FunctionsError(#[from] supabase_rust_functions::FunctionsError),
    
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Generic error
    #[error("{0}")]
    Other(String),
}

/// Result type for the Supabase client
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new API error
    pub fn api_error(code: impl Into<String> + std::fmt::Display, message: impl Into<String> + std::fmt::Display) -> Self {
        Self::Other(format!("API error: {message} (code: {code})"))
    }
    
    /// Create a new general error
    pub fn general(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

/// Prints an API error from the Supabase API
pub fn api_error<E: std::fmt::Display>(error: E) -> Error {
    Error::Other(format!("API error: {}", error))
}