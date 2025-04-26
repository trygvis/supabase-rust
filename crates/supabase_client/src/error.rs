// src/error.rs

// Define custom error types for the Supabase client operations.
// Use libraries like thiserror for easier error definition.

use thiserror::Error;

/// Universal error type for the Supabase client library operations.
#[derive(Error, Debug)]
pub enum SupabaseError {
    #[error("Configuration error: Missing or invalid {0}")]
    Config(String), // e.g., "SUPABASE_URL"

    #[error("Initialization failed: {0}")]
    Initialization(String), // For general client setup issues

    #[error("Authentication error: {0}")]
    Auth(#[from] supabase_rust_auth::AuthError), // Wrap the specific auth crate error

    #[error("Database error: {0}")]
    Postgrest(#[from] supabase_rust_postgrest::PostgrestError), // Wrap the postgrest crate error

    #[error("Realtime error: {0}")]
    Realtime(String), // Define more specific realtime errors if needed or wrap from crate

    #[error("Storage error: {0}")]
    Storage(String), // Placeholder, wrap actual StorageError if storage crate is used

    #[error("Function error: {0}")]
    Function(String), // Placeholder, wrap actual FunctionError if functions crate is used

    #[error("Network request error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("URL parsing error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Operation timed out")]
    Timeout,

    #[error("An unexpected error occurred: {0}")]
    Internal(String),

    #[error("Unknown error")]
    Unknown,
}

// Optional: Type aliases for convenience if needed elsewhere
pub type Result<T> = std::result::Result<T, SupabaseError>;

// Define specific AuthError, DbError etc. as needed, potentially wrapping SupabaseError
// or being distinct enums. 