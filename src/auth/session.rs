//! Session management for authentication

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// The access token
    #[serde(rename = "access_token")]
    pub access_token: String,
    
    /// The refresh token
    #[serde(rename = "refresh_token")]
    pub refresh_token: String,
    
    /// The user ID
    #[serde(rename = "user_id")]
    pub user_id: String,
    
    /// The token type
    #[serde(rename = "token_type")]
    pub token_type: String,
    
    /// The expiry time in seconds
    #[serde(rename = "expires_in")]
    pub expires_in: i64,
    
    /// The expiry timestamp
    #[serde(rename = "expires_at")]
    pub expires_at: Option<i64>,
}

impl Session {
    /// Create a new session
    pub fn new(
        access_token: String,
        refresh_token: String,
        user_id: String,
        expires_in: i64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;
        
        Self {
            access_token,
            refresh_token,
            user_id,
            token_type: "bearer".to_string(),
            expires_in,
            expires_at: Some(now + expires_in),
        }
    }
    
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as i64;
            
            now >= expires_at
        } else {
            false
        }
    }
}