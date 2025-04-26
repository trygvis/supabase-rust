// src/error.rs

// Define custom error types for the Supabase client operations.
// Use libraries like thiserror for easier error definition.

use thiserror::Error;

// Use correct error path from supabase-rust-auth v0.2.0
use supabase_rust_auth::AuthError;
use supabase_rust_postgrest::PostgrestError;

/// Universal error type for the Supabase client library operations.
/// This now mostly wraps or maps errors from the `supabase_rust_gftd` crate.
#[derive(Error, Debug)]
pub enum SupabaseError {
    #[error("Configuration error: Missing or invalid {0}")]
    Config(String), // e.g., "SUPABASE_URL"

    #[error("Initialization failed: {0}")]
    Initialization(String), // For general client setup issues

    // Revert to original error wrapping
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("Database error: {0}")]
    Postgrest(#[from] PostgrestError),

    #[error("Realtime error: {0}")]
    Realtime(String), // Keep as String for now as realtime code is commented out

    #[error("Storage error: {0}")]
    Storage(String), // Keep as String

    #[error("Function error: {0}")]
    Function(String), // Keep as String

    // Keep utility errors
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
