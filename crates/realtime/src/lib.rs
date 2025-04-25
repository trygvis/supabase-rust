//! Supabase Realtime client for Rust
//!
//! This crate provides realtime functionality for Supabase,
//! allowing for subscribing to database changes in real-time.

// Declare modules
mod channel;
mod client;
mod error;
mod filters;
mod message;

// Re-export key public types
pub use channel::{
    BroadcastChanges, ChannelBuilder, DatabaseChanges, PresenceChanges, Subscription,
};
pub use client::{ConnectionState, RealtimeClient, RealtimeClientOptions};
pub use error::RealtimeError;
pub use filters::{DatabaseFilter, FilterOperator};
pub use message::{ChannelEvent, Payload, PresenceChange, PresenceState};

// TODO: Move tests from the original lib.rs into integration tests (`tests/`) or inline here.
// mod tests {
// }
