//! Error handling for the Supabase Rust client

use std::fmt;
use thiserror::Error;

/// Unified error type for the Supabase Rust client
#[derive(Error, Debug)]
pub enum Error {
    /// Network or HTTP related errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization or deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Database query errors
    #[error("Database error: {0}")]
    Database(String),
    
    /// Storage errors
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Realtime subscription errors
    #[error("Realtime error: {0}")]
    Realtime(String),
    
    /// Edge Function errors
    #[error("Function error: {0}")]
    Function(String),
    
    /// URL parsing errors
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    
    /// JWT errors
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    
    /// General errors
    #[error("{0}")]
    General(String),
}

impl Error {
    /// Create a new authentication error
    pub fn auth<T: fmt::Display>(msg: T) -> Self {
        Error::Auth(msg.to_string())
    }

    /// Create a new database error
    pub fn database<T: fmt::Display>(msg: T) -> Self {
        Error::Database(msg.to_string())
    }
    
    /// Create a new storage error
    pub fn storage<T: fmt::Display>(msg: T) -> Self {
        Error::Storage(msg.to_string())
    }
    
    /// Create a new realtime error
    pub fn realtime<T: fmt::Display>(msg: T) -> Self {
        Error::Realtime(msg.to_string())
    }
    
    /// Create a new function error
    pub fn function<T: fmt::Display>(msg: T) -> Self {
        Error::Function(msg.to_string())
    }
    
    /// Create a new general error
    pub fn general<T: fmt::Display>(msg: T) -> Self {
        Error::General(msg.to_string())
    }
}