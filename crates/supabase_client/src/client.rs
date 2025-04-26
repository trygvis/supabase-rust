// src/client.rs

// Implement the Supabase client logic based on the 'state_machines'
// and 'api_interface' sections in supabase_interaction.ssot

use crate::error::{SupabaseError, Result};
use crate::models::{Item, User, AuthCredentials};
use supabase_rust_auth::{AuthClient, GoTrueError, Session as AuthSession};
use supabase_rust_postgrest::PostgrestClient;
use supabase_rust_realtime::{RealtimeClient, Channel, RealtimeMessage, Callback};
use url::Url;
use std::sync::Arc; // Use Arc for shared ownership of clients
use tokio::sync::{Mutex, mpsc}; // Add mpsc for sending updates from callback
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for the Supabase client.
/// It's recommended to load these values from environment variables or a secure config source.
#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub url: Url,          // Use Url type for validation
    pub anon_key: String,
}

impl SupabaseConfig {
    /// Creates a new configuration, validating the URL.
    pub fn new(url_str: &str, anon_key: String) -> Result<Self> {
        let url = Url::parse(url_str).map_err(SupabaseError::UrlParse)?;
        if anon_key.is_empty() {
            return Err(SupabaseError::Config("anon_key cannot be empty".to_string()));
        }
        Ok(Self { url, anon_key })
    }

    /// Attempts to create configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let url_str = std::env::var("SUPABASE_URL")
            .map_err(|_| SupabaseError::Config("SUPABASE_URL environment variable not found".to_string()))?;
        let anon_key = std::env::var("SUPABASE_ANON_KEY")
            .map_err(|_| SupabaseError::Config("SUPABASE_ANON_KEY environment variable not found".to_string()))?;
        Self::new(&url_str, anon_key)
    }
}

/// Represents the different types of changes received from a realtime subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemChange {
    Insert(Item),
    Update(Item),
    Delete(HashMap<String, Value>), // Contains old record keys/values
    Error(String),
}

/// The main Supabase client wrapper, providing access to different Supabase features.
/// This struct holds initialized clients for Auth, Postgrest, etc., and manages the current session.
#[derive(Debug, Clone)]
pub struct SupabaseClientWrapper {
    pub auth: Arc<AuthClient>,
    pub db: Arc<PostgrestClient>,
    pub realtime: Arc<RealtimeClient>,
    // Stores the current session details (including token)
    current_session: Arc<Mutex<Option<AuthSession>>>,
    // config: SupabaseConfig,
}

impl SupabaseClientWrapper {
    /// Creates a new Supabase client wrapper.
    /// Initializes Auth and Postgrest clients based on the provided config.
    /// Corresponds to the `initializeSupabaseClient` step in the SSOT.
    pub fn new(config: SupabaseConfig) -> Result<Self> {
        let auth_client = AuthClient::new(&config.url.to_string(), &config.anon_key)
            .map_err(|e| SupabaseError::Initialization(format!("Failed to create AuthClient: {}", e)))?;
        let rest_url = config.url.join("rest/v1/")
                                .map_err(|e| SupabaseError::Initialization(format!("Failed to construct PostgREST URL: {}", e)))?;
        let db_client = PostgrestClient::new(rest_url.clone(), config.anon_key.clone())
             .map_err(|e| SupabaseError::Initialization(format!("Failed to create PostgrestClient: {}", e)))?;

        // Construct Realtime URL (wss://<host>/realtime/v1)
        let mut rt_url_builder = config.url.clone();
        let scheme = if config.url.scheme() == "https" { "wss" } else { "ws" };
        rt_url_builder.set_scheme(scheme)
            .map_err(|_| SupabaseError::Initialization("Failed to set scheme for Realtime URL".to_string()))?;
        let rt_url = rt_url_builder.join("realtime/v1")
            .map_err(|e| SupabaseError::Initialization(format!("Failed to construct Realtime URL: {}", e)))?;

        let realtime_client = RealtimeClient::new(rt_url, config.anon_key.clone());

        println!(
            "Supabase client initialized. Auth URL: {}, DB URL: {}, Realtime URL: {}",
            auth_client.url(),
            db_client.url(),
            realtime_client.url()
        );

        Ok(Self {
            auth: Arc::new(auth_client),
            db: Arc::new(db_client),
            realtime: Arc::new(realtime_client),
            current_session: Arc::new(Mutex::new(None)), // Initialize session as None
            // config,
        })
    }

    /// Convenience function to create a client directly from environment variables.
    pub fn from_env() -> Result<Self> {
        let config = SupabaseConfig::from_env()?;
        Self::new(config)
    }

    // --- Methods corresponding to SSOT api_interface ---

    /// Authenticates a user using email and password.
    /// Corresponds to `authenticateUser` in the SSOT.
    /// Returns the Supabase User details on success.
    pub async fn authenticate(&self, credentials: AuthCredentials) -> Result<User> {
        if credentials.email.is_empty() || credentials.password.is_empty() {
            return Err(SupabaseError::InvalidInput(
                "Email and password cannot be empty".to_string(),
            ));
        }
        println!("Attempting to authenticate user: {}", credentials.email);

        let session = self
            .auth
            .sign_in_with_password(&credentials.email, &credentials.password)
            .await
            .map_err(SupabaseError::Auth)?;

        let auth_user = session
            .user
            .clone() // Clone user data before moving session
            .ok_or_else(|| SupabaseError::Internal("Authentication successful but no user data received".to_string()))?;

        // Store the session internally
        let mut session_guard = self.current_session.lock().await;
        *session_guard = Some(session.clone()); // Clone session to store
        drop(session_guard); // Release lock explicitly

        // Set JWT for realtime client
        if let Some(ref token) = session.access_token {
             self.realtime.set_jwt(token.clone()).await;
        }

        // Convert to our model
        let user_model = User {
            id: auth_user.id.parse::<uuid::Uuid>().map_err(|_| SupabaseError::Internal("Failed to parse user ID as UUID".to_string()))?,
            aud: auth_user.aud.unwrap_or_default(),
            role: auth_user.role,
            email: auth_user.email,
            phone: auth_user.phone,
            confirmation_sent_at: auth_user.confirmation_sent_at,
            confirmed_at: auth_user.confirmed_at,
            email_confirmed_at: auth_user.email_confirmed_at,
            phone_confirmed_at: auth_user.phone_confirmed_at,
            recovery_sent_at: auth_user.recovery_sent_at,
            last_sign_in_at: auth_user.last_sign_in_at,
            created_at: auth_user.created_at.unwrap_or_else(Utc::now),
            updated_at: auth_user.updated_at.unwrap_or_else(Utc::now),
        };

        println!("User authenticated successfully: {}", user_model.email.as_deref().unwrap_or("N/A"));
        Ok(user_model)
    }

    /// Logs out the currently authenticated user by invalidating the session/token.
    /// Corresponds to `logoutUser` in the SSOT.
    pub async fn logout(&self) -> Result<()> {
        println!("Attempting to log out user...");
        let mut session_guard = self.current_session.lock().await;
        *session_guard = None;
        drop(session_guard);
        self.realtime.remove_jwt().await; // Clear JWT from realtime
        self.auth.sign_out().await.map_err(SupabaseError::Auth)?;
        println!("User logged out successfully.");
        Ok(())
    }

    /// Fetches 'items' from the database.
    /// Assumes the user is authenticated and uses the stored session token.
    /// Relies on RLS being configured in Supabase for the 'items' table.
    /// Corresponds to `fetchItemsFromSupabase` in the SSOT.
    pub async fn fetch_items(&self) -> Result<Vec<Item>> {
        println!("Attempting to fetch items...");
        let token = {
            let session_guard = self.current_session.lock().await;
            session_guard
                .as_ref()
                .and_then(|s| s.access_token.clone())
                .ok_or_else(|| SupabaseError::Auth("Not authenticated or session token missing".to_string().into()))?
        };
        let response = self.db.from("items").auth(&token).select("*").execute().await.map_err(SupabaseError::Postgrest)?;
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SupabaseError::Postgrest(supabase_rust_postgrest::PostgrestError::RequestError(error_body)));
        }
        let items = response.json::<Vec<Item>>().await
            .map_err(|reqwest_err| SupabaseError::Json(serde_json::Error::custom(format!("Failed to deserialize items: {}", reqwest_err))))?;
        println!("Fetched {} items successfully.", items.len());
        Ok(items)
    }

    /// Establishes a real-time subscription for changes to the 'items' table.
    /// Corresponds to `subscribeToItemChanges` in the SSOT.
    /// Returns a channel sender to receive `ItemChange` updates.
    pub async fn subscribe_to_item_changes(&self) -> Result<mpsc::UnboundedSender<ItemChange>> { // Return a handle/sender?
        println!("Setting up items subscription...");

        // Ensure connected and authenticated (check token)
        let token = {
            let session_guard = self.current_session.lock().await;
            session_guard.as_ref().and_then(|s| s.access_token.clone())
               .ok_or_else(|| SupabaseError::Auth("Realtime requires authentication".to_string().into()))?
        };
        // The realtime client might need the token set explicitly if not done at auth
        self.realtime.set_jwt(token).await;

        // Create an MPSC channel to send updates back to the caller
        let (tx, mut rx) = mpsc::unbounded_channel::<ItemChange>();

        let channel_tx = tx.clone(); // Clone sender for the callback
        let callback: Callback = Arc::new(move |message: Arc<RealtimeMessage>| {
            let tx = channel_tx.clone();
            async move {
                println!("Realtime message received: {:?}", message.event);
                match message.event.as_str() {
                    "postgres_changes" => {
                        if let Some(payload) = message.payload.as_ref() {
                            match payload.get("type").and_then(|t| t.as_str()) {
                                Some("INSERT") => {
                                    if let Some(record) = payload.get("record") {
                                        match serde_json::from_value::<Item>(record.clone()) {
                                            Ok(item) => tx.send(ItemChange::Insert(item)).unwrap_or(()),
                                            Err(e) => tx.send(ItemChange::Error(format!("Failed to deserialize INSERT: {}", e))).unwrap_or(()),
                                        }
                                    }
                                }
                                Some("UPDATE") => {
                                    if let Some(record) = payload.get("record") {
                                        match serde_json::from_value::<Item>(record.clone()) {
                                            Ok(item) => tx.send(ItemChange::Update(item)).unwrap_or(()),
                                            Err(e) => tx.send(ItemChange::Error(format!("Failed to deserialize UPDATE: {}", e))).unwrap_or(()),
                                        }
                                    }
                                }
                                Some("DELETE") => {
                                     if let Some(old_record) = payload.get("old_record") {
                                         match serde_json::from_value::<HashMap<String, Value>>(old_record.clone()) {
                                             Ok(keys) => tx.send(ItemChange::Delete(keys)).unwrap_or(()),
                                             Err(e) => tx.send(ItemChange::Error(format!("Failed to deserialize DELETE: {}", e))).unwrap_or(()),
                                         }
                                     }
                                }
                                _ => { println!("Unknown change type"); }
                            }
                        }
                    }
                    _ => { println!("Non-postgres change event: {}", message.event); }
                }
            }
        });

        let channel = self.realtime.channel("realtime:public:items").await; // Use appropriate channel name
        channel.on("postgres_changes", callback).await; // Listen to postgres changes

        // Spawn a task to connect the client and keep it running
        let rt_client = self.realtime.clone();
        tokio::spawn(async move {
             println!("Connecting Realtime client...");
             if let Err(e) = rt_client.connect().await {
                 eprintln!("Realtime connection error: {}", e); // Log error
                 // Optionally send an error through the channel if possible
                 // tx.send(ItemChange::Error(format!("Connection failed: {}", e))).unwrap_or(());
             }
             println!("Realtime client task finished.");
        });

        println!("Realtime subscription setup initiated for items.");
        Ok(tx) // Return the sender part of the channel
    }

    // --- CRUD Operations for Items ---

    /// Creates a new item in the database.
    /// Requires authentication.
    pub async fn create_item(&self, new_item: Item) -> Result<Item> {
        println!("Attempting to create item: {:?}", new_item.name);
        let token = self.get_auth_token().await?;

        // PostgREST typically expects a Vec for insertion, even for a single item
        let response = self
            .db
            .from("items")
            .auth(&token)
            .insert(vec![new_item]) // Insert expects a collection
            .execute()
            .await
            .map_err(SupabaseError::Postgrest)?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SupabaseError::Postgrest(supabase_rust_postgrest::PostgrestError::RequestError(error_body)));
        }

        // Supabase insert returns the created records in the body
        let created_items = response
            .json::<Vec<Item>>()
            .await
            .map_err(|e| SupabaseError::Json(serde_json::Error::custom(format!("Failed to deserialize created item: {}", e))))?;

        // Return the first item from the result vec (should be exactly one)
        created_items.into_iter().next().ok_or_else(|| {
            SupabaseError::Internal("No item returned after successful creation".to_string())
        })
    }

    /// Fetches a single item by its ID.
    /// Requires authentication.
    pub async fn fetch_item_by_id(&self, item_id: Uuid) -> Result<Option<Item>> {
        println!("Attempting to fetch item by ID: {}", item_id);
        let token = self.get_auth_token().await?;

        let response = self
            .db
            .from("items")
            .auth(&token)
            .select("*")
            .eq("id", item_id.to_string()) // Filter by ID
            .maybe_single() // Expect 0 or 1 result
            .execute()
            .await
            .map_err(SupabaseError::Postgrest)?;

        if !response.status().is_success() {
            // Handle potential 404 Not Found specifically if maybe_single() doesn't
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(None); // Not found is not an error in this context
            }
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SupabaseError::Postgrest(supabase_rust_postgrest::PostgrestError::RequestError(error_body)));
        }

        // maybe_single() response body is the object itself, or null if not found
        let item_option = response
            .json::<Option<Item>>()
            .await
            .map_err(|e| SupabaseError::Json(serde_json::Error::custom(format!("Failed to deserialize item option: {}", e))))?;

        Ok(item_option)
    }

    /// Updates an existing item in the database.
    /// Requires authentication.
    /// Note: This performs a full update. For partial updates, use a different struct or serde attributes.
    pub async fn update_item(&self, item_id: Uuid, item_update: Item) -> Result<Item> {
        println!("Attempting to update item ID: {}", item_id);
        let token = self.get_auth_token().await?;

        let response = self
            .db
            .from("items")
            .auth(&token)
            .update(item_update) // The update data
            .eq("id", item_id.to_string()) // Specify which item to update
            .execute()
            .await
            .map_err(SupabaseError::Postgrest)?;

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SupabaseError::Postgrest(supabase_rust_postgrest::PostgrestError::RequestError(error_body)));
        }

        // Update also returns the updated records
         let updated_items = response
            .json::<Vec<Item>>()
            .await
            .map_err(|e| SupabaseError::Json(serde_json::Error::custom(format!("Failed to deserialize updated item: {}", e))))?;

        updated_items.into_iter().next().ok_or_else(|| {
            SupabaseError::Internal("No item returned after successful update".to_string())
        })
    }

    /// Deletes an item by its ID from the database.
    /// Requires authentication.
    pub async fn delete_item(&self, item_id: Uuid) -> Result<()> {
        println!("Attempting to delete item ID: {}", item_id);
        let token = self.get_auth_token().await?;

        let response = self
            .db
            .from("items")
            .auth(&token)
            .delete()
            .eq("id", item_id.to_string()) // Specify which item to delete
            .execute()
            .await
            .map_err(SupabaseError::Postgrest)?;

        if !response.status().is_success() {
             // Consider if 404 on delete should be an error or silent success
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SupabaseError::Postgrest(supabase_rust_postgrest::PostgrestError::RequestError(error_body)));
        }

        // Delete often returns minimal content, just check status
        println!("Item deleted successfully: {}", item_id);
        Ok(())
    }

    // Helper function to get the current auth token
    async fn get_auth_token(&self) -> Result<String> {
        let session_guard = self.current_session.lock().await;
        session_guard
            .as_ref()
            .and_then(|s| s.access_token.clone())
            .ok_or_else(|| SupabaseError::Auth("Not authenticated or session token missing".to_string().into()))
    }
} 