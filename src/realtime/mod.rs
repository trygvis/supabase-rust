//! Realtime client for Supabase

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Error;

/// Client for Supabase Realtime
#[derive(Debug, Clone)]
pub struct RealtimeClient {
    /// The base URL for the Supabase project
    url: String,
    
    /// The anonymous API key for the Supabase project
    key: String,
}

/// Subscription channel
pub struct Channel {
    /// The channel name
    pub name: String,
    
    /// The topic to subscribe to
    pub topic: String,
}

/// Realtime event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RealtimeEventType {
    /// Insert operation
    #[serde(rename = "INSERT")]
    Insert,
    
    /// Update operation
    #[serde(rename = "UPDATE")]
    Update,
    
    /// Delete operation
    #[serde(rename = "DELETE")]
    Delete,
    
    /// Select operation
    #[serde(rename = "SELECT")]
    Select,
}

/// Realtime event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeEvent<T> {
    /// The schema
    pub schema: String,
    
    /// The table
    pub table: String,
    
    /// Commit timestamp
    pub commit_timestamp: String,
    
    /// Event type
    #[serde(rename = "eventType")]
    pub event_type: RealtimeEventType,
    
    /// New data after the operation
    pub new: Option<T>,
    
    /// Old data before the operation
    pub old: Option<T>,
}

/// Payload for sending a message
#[derive(Debug, Clone, Serialize)]
pub struct Payload {
    /// The type of message
    #[serde(rename = "type")]
    pub message_type: String,
    
    /// The event
    pub event: String,
    
    /// The payload
    pub payload: serde_json::Value,
}

impl RealtimeClient {
    /// Create a new RealtimeClient
    pub(crate) fn new(url: &str, key: &str) -> Self {
        Self {
            url: url.to_string(),
            key: key.to_string(),
        }
    }
    
    /// Get the WebSocket URL for the Realtime API
    pub fn get_url(&self) -> String {
        let url = self.url.replace("http://", "ws://").replace("https://", "wss://");
        format!("{}/realtime/v1/websocket?apikey={}", url, self.key)
    }
    
    /// Create a channel subscription
    pub fn channel(&self, name: &str) -> ChannelBuilder {
        ChannelBuilder {
            name: name.to_string(),
            tables: Vec::new(),
            schemas: Vec::new(),
            filters: HashMap::new(),
        }
    }
}

/// Builder for creating a channel subscription
pub struct ChannelBuilder {
    /// The channel name
    pub name: String,
    
    /// The tables to subscribe to
    pub tables: Vec<String>,
    
    /// The schemas to subscribe to
    pub schemas: Vec<String>,
    
    /// The filters to apply
    pub filters: HashMap<String, String>,
}

impl ChannelBuilder {
    /// Subscribe to a specific table
    pub fn table(mut self, table: &str) -> Self {
        self.tables.push(table.to_string());
        self
    }
    
    /// Subscribe to a specific schema
    pub fn schema(mut self, schema: &str) -> Self {
        self.schemas.push(schema.to_string());
        self
    }
    
    /// Add a filter
    pub fn filter(mut self, key: &str, value: &str) -> Self {
        self.filters.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Build the channel
    pub fn build(self) -> Channel {
        // Format channel topic: realtime:{schema}.{table}
        let tables = if self.tables.is_empty() {
            "*".to_string()
        } else {
            self.tables.join(",")
        };
        
        let schemas = if self.schemas.is_empty() {
            "public".to_string()
        } else {
            self.schemas.join(",")
        };
        
        let topic = format!("realtime:{}:{}", schemas, tables);
        
        Channel {
            name: self.name,
            topic,
        }
    }
}