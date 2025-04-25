//! Supabase Realtime client for Rust
//!
//! This crate provides realtime functionality for Supabase,
//! allowing for subscribing to database changes in real-time.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

// Declare modules
mod error;
mod message;
mod filters;
mod channel;
mod client;

// Re-export key public types
pub use error::RealtimeError;
pub use message::{ChannelEvent, Payload, PresenceChange, PresenceState};
pub use filters::{DatabaseFilter, FilterOperator};
pub use channel::{BroadcastChanges, ChannelBuilder, DatabaseChanges, PresenceChanges, Subscription};
pub use client::{ConnectionState, RealtimeClient, RealtimeClientOptions};

/*
/// エラー型 <- Start commenting out
#[derive(Error, Debug)]
pub enum RealtimeError {
    // ... (all original code commented out) ...
}

// ... (rest of the original file commented out) ...

mod tests {
    // ... (tests commented out) ...
}
*/ // <- End commenting out
