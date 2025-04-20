//! Error types for Supabase Rust client

use thiserror::Error;

/// The main error type for the Supabase Rust client
#[derive(Error, Debug)]
pub enum Error {
    /// API related errors
    #[error("API error: {message} (code: {code})")]
    ApiError {
        code: String,
        message: String,
    },
    
    /// Authentication errors
    #[error("Authentication error: {0}")]
    AuthError(#[from] crate::auth::AuthError),
    
    /// PostgreREST errors
    #[error("PostgreREST error: {0}")]
    PostgrestError(String),
    
    /// Storage errors
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Realtime errors
    #[error("Realtime error: {0}")]
    RealtimeError(String),
    
    /// Functions errors
    #[error("Functions error: {0}")]
    FunctionsError(String),
    
    /// Network errors
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    /// JSON serialization errors
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    /// URL parsing errors
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    /// Unexpected errors
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    
    /// Parameter validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Result type for Supabase operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a new API error
    pub fn api_error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ApiError {
            code: code.into(),
            message: message.into(),
        }
    }
    
    /// Create a new validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }
}