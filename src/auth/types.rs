//! Types for authentication and user management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// The user data
    pub user: Option<User>,
    
    /// The session data
    pub session: Option<Session>,
    
    /// The access token
    #[serde(rename = "access_token")]
    pub access_token: Option<String>,
    
    /// The refresh token
    #[serde(rename = "refresh_token")]
    pub refresh_token: Option<String>,
    
    /// The token type
    #[serde(rename = "token_type")]
    pub token_type: Option<String>,
    
    /// The expiry time in seconds
    #[serde(rename = "expires_in")]
    pub expires_in: Option<i64>,
    
    /// Any error that occurred
    pub error: Option<String>,
    
    /// The error description
    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}

/// User data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// The user ID
    pub id: String,
    
    /// The app metadata
    #[serde(rename = "app_metadata")]
    pub app_metadata: HashMap<String, serde_json::Value>,
    
    /// The user metadata
    #[serde(rename = "user_metadata")]
    pub user_metadata: HashMap<String, serde_json::Value>,
    
    /// The user's authentication providers
    pub identities: Option<Vec<Identity>>,
    
    /// The user's email address
    pub email: Option<String>,
    
    /// Whether the email has been confirmed
    #[serde(rename = "email_confirmed_at")]
    pub email_confirmed_at: Option<String>,
    
    /// The user's phone number
    pub phone: Option<String>,
    
    /// Whether the phone has been confirmed
    #[serde(rename = "phone_confirmed_at")]
    pub phone_confirmed_at: Option<String>,
    
    /// The last sign-in time
    #[serde(rename = "last_sign_in_at")]
    pub last_sign_in_at: Option<String>,
    
    /// The creation time
    #[serde(rename = "created_at")]
    pub created_at: String,
    
    /// The update time
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    
    /// The user's role
    pub role: Option<String>,
    
    /// Whether the user is anonymous
    #[serde(rename = "is_anonymous")]
    pub is_anonymous: Option<bool>,
}

/// A user identity (authentication provider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// The identity ID
    pub id: String,
    
    /// The identity provider
    pub provider: String,
    
    /// The user ID
    #[serde(rename = "user_id")]
    pub user_id: String,
    
    /// The identity metadata
    pub identity_data: Option<HashMap<String, serde_json::Value>>,
    
    /// The creation time
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    
    /// The update time
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
    
    /// The last sign-in time
    #[serde(rename = "last_sign_in_at")]
    pub last_sign_in_at: Option<String>,
}