// src/models.rs

// Define Rust structs corresponding to the 'models' section in supabase_interaction.ssot
// Example based on the 'item' model in the SSOT:

// Potentially use crates like serde for serialization/deserialization
// and uuid for the ID type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the 'item' model structure defined in the SSOT.
/// Corresponds to the 'items' table in Supabase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    #[serde(default = "Uuid::new_v4")] // Generate UUID if missing, useful for inserts
    pub id: Uuid,
    pub user_id: Uuid, // Assuming user_id is known when creating/fetching
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] // Don't include in JSON if None
    pub description: Option<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// Represents authentication credentials based on SSOT validation rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthCredentials {
    // Add validation attributes if using a validation library
    pub email: String,
    pub password: String,
}

/// Represents a Supabase User, typically returned after authentication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub aud: String, // Audience
    pub role: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub confirmation_sent_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub email_confirmed_at: Option<DateTime<Utc>>,
    pub phone_confirmed_at: Option<DateTime<Utc>>,
    pub recovery_sent_at: Option<DateTime<Utc>>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    // pub app_metadata: serde_json::Value, // Use Value for flexible metadata
    // pub user_metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Add other relevant fields from Supabase Auth user object as needed
}

// --- From Trait Implementation ---

impl From<supabase_rust_auth::User> for User {
    fn from(auth_user: supabase_rust_auth::User) -> Self {
        // Attempt to parse UUID and timestamps, providing defaults on failure
        let parsed_id = Uuid::parse_str(&auth_user.id).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to parse user ID '{}' as UUID: {}. Using default UUID.",
                auth_user.id, e
            );
            Uuid::nil() // Or Uuid::new_v4() if a default is not appropriate
        });

        let parsed_created_at = DateTime::parse_from_rfc3339(&auth_user.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to parse created_at '{}': {}. Using current time.",
                    auth_user.created_at, e
                );
                Utc::now()
            });

        let parsed_updated_at = DateTime::parse_from_rfc3339(&auth_user.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to parse updated_at '{}': {}. Using current time.",
                    auth_user.updated_at, e
                );
                Utc::now()
            });

        User {
            id: parsed_id,
            email: auth_user.email,
            phone: auth_user.phone,
            created_at: parsed_created_at,
            updated_at: parsed_updated_at,
            // Set missing fields to defaults
            aud: String::new(), // Or some default? Check Supabase docs
            role: None,
            confirmation_sent_at: None,
            confirmed_at: None,
            email_confirmed_at: None,
            phone_confirmed_at: None,
            recovery_sent_at: None,
            last_sign_in_at: None,
            // app_metadata: Default::default(), // If added back as serde_json::Value
            // user_metadata: Default::default(), // If added back as serde_json::Value
        }
    }
}

// Add other models defined in SSOT here.

// Define AuthCredentials struct based on SSOT validation rules
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct AuthCredentials {
//     pub email: String,
//     pub password: String,
// }

// Define User struct based on Supabase response
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub struct User {
//     pub id: Uuid,
//     pub email: Option<String>,
//     // Add other relevant user fields
// }
